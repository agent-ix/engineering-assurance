// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Pure decoding of retained cli-agent-evals reports into typed observations.
//!
//! This module owns the closed report shape and its conversion to evaluation
//! envelopes. Filesystem traversal and transcript-byte verification remain in
//! the binary host adapter.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer};
use serde_json::Value;
use thiserror::Error;

use crate::{
    evaluation::{
        EvaluationEnvelope, EvaluationHost, EvaluationScenario, ExecutionStatus,
        is_immutable_revision, is_lower_sha256_digest, is_safe_transcript_reference,
    },
    evidence::GoverningVersions,
    workflow::DecisionEvent,
};

/// Exact cli-agent-evals report discriminator accepted by this adapter.
pub const CLI_REPORT_PROTOCOL: &str = "cli-agent-evals.report/v1";

/// Largest report accepted before JSON decoding.
pub const MAX_CLI_REPORT_BYTES: usize = 8 * 1024 * 1024;

/// Largest result population accepted in one report.
pub const MAX_CLI_REPORT_RESULTS: usize = 256;

/// Stable model identity used when the runner selected its host default.
pub const RUNNER_DEFAULT_MODEL: &str = "runner-default";

/// One decoded report whose samples are ready for host transcript verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedEvaluationReport {
    host: EvaluationHost,
    model: String,
    samples: Vec<DecodedEvaluationSample>,
}

impl DecodedEvaluationReport {
    /// Return the report's closed host identity.
    #[must_use]
    pub const fn host(&self) -> EvaluationHost {
        self.host
    }

    /// Return the exact model or the stable host-default marker.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Return the decoded samples in report order.
    #[must_use]
    pub fn samples(&self) -> &[DecodedEvaluationSample] {
        &self.samples
    }
}

/// One failed attempt or retained successful sample from a decoded report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodedEvaluationSample {
    /// A failed sample retained only as a diagnostic.
    Failed {
        /// Closed scenario identity.
        scenario: EvaluationScenario,
        /// Stable diagnostic assembled from the runner outcome.
        diagnostic: String,
    },
    /// A successful sample requiring host-side transcript verification.
    Retained {
        /// Reported work directory, still untrusted until host confinement.
        work_dir: String,
        /// Safe portable path relative to the work directory.
        transcript_path: String,
        /// Reported lowercase SHA-256 transcript identity.
        transcript_digest: String,
        /// Typed envelope to admit only after transcript verification.
        envelope: Box<EvaluationEnvelope>,
    },
}

/// Stable failures at the cli-agent-evals report boundary.
#[derive(Debug, Error)]
pub enum EvaluationReportError {
    /// The report exceeded the pre-decode byte ceiling.
    #[error("cli-agent-evals report is {actual} bytes; limit is {limit} bytes")]
    ReportTooLarge {
        /// Observed serialized size.
        actual: usize,
        /// Maximum accepted serialized size.
        limit: usize,
    },
    /// JSON or its closed structural shape was invalid.
    #[error("cli-agent-evals report is invalid: {detail}")]
    InvalidReport {
        /// Non-sensitive structural diagnostic.
        detail: String,
    },
    /// The report used an unsupported discriminator.
    #[error("unsupported cli-agent-evals report protocol: {0}")]
    UnsupportedProtocol(String),
    /// The report exceeded its result ceiling.
    #[error("cli-agent-evals report has {actual} results; limit is {limit}")]
    ResultLimit {
        /// Observed result count.
        actual: usize,
        /// Maximum accepted result count.
        limit: usize,
    },
    /// A supported report invariant was contradicted.
    #[error("cli-agent-evals report contract violation: {0}")]
    Contract(&'static str),
}

impl EvaluationReportError {
    /// Return the stable machine error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ReportTooLarge { .. } => "evaluation_report_too_large",
            Self::InvalidReport { .. } => "evaluation_report_invalid",
            Self::UnsupportedProtocol(_) => "evaluation_report_protocol_unsupported",
            Self::ResultLimit { .. } => "evaluation_report_result_limit",
            Self::Contract(_) => "evaluation_report_contract_invalid",
        }
    }
}

/// Decode and validate one complete cli-agent-evals report without performing I/O.
///
/// # Errors
///
/// Returns [`EvaluationReportError`] when the byte ceiling, JSON shape,
/// protocol, closed populations, outcome relationships, retention identity,
/// or Engineering Assurance result observation is invalid.
pub fn decode_cli_eval_report(
    bytes: &[u8],
    expected_source_revision: &str,
) -> Result<DecodedEvaluationReport, EvaluationReportError> {
    if bytes.len() > MAX_CLI_REPORT_BYTES {
        return Err(EvaluationReportError::ReportTooLarge {
            actual: bytes.len(),
            limit: MAX_CLI_REPORT_BYTES,
        });
    }
    let report: RawReport =
        serde_json::from_slice(bytes).map_err(|error| EvaluationReportError::InvalidReport {
            detail: error.to_string(),
        })?;
    decode_report(report, expected_source_revision)
}

fn decode_report(
    report: RawReport,
    expected_source_revision: &str,
) -> Result<DecodedEvaluationReport, EvaluationReportError> {
    if !is_immutable_revision(expected_source_revision) {
        return Err(EvaluationReportError::Contract(
            "expected-source-revision-not-immutable",
        ));
    }
    let RawReport {
        report_version,
        ok,
        generated_at,
        suite,
        agent,
        model,
        repeats,
        results,
        aggregates,
    } = report;
    drop((generated_at, suite, aggregates));
    if report_version != CLI_REPORT_PROTOCOL {
        return Err(EvaluationReportError::UnsupportedProtocol(report_version));
    }
    if repeats != 1 {
        return Err(EvaluationReportError::Contract("repeat-count-not-one"));
    }
    if results.len() > MAX_CLI_REPORT_RESULTS {
        return Err(EvaluationReportError::ResultLimit {
            actual: results.len(),
            limit: MAX_CLI_REPORT_RESULTS,
        });
    }
    let host = parse_host(agent.as_deref().unwrap_or_default())?;
    if ok != results.iter().all(|result| result.ok) {
        return Err(EvaluationReportError::Contract("report-outcome-mismatch"));
    }
    let samples = results
        .into_iter()
        .map(|result| decode_result(host, result, expected_source_revision))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(DecodedEvaluationReport {
        host,
        model: model
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| RUNNER_DEFAULT_MODEL.to_owned()),
        samples,
    })
}

fn decode_result(
    host: EvaluationHost,
    result: RawResult,
    expected_source_revision: &str,
) -> Result<DecodedEvaluationSample, EvaluationReportError> {
    let RawResult {
        id,
        use_case,
        ok,
        pass_rate,
        aggregate,
        mut runs,
    } = result;
    drop(id);
    validate_summaries(aggregate);
    let scenario = parse_scenario(use_case.as_deref().unwrap_or_default())?;
    if runs.len() != 1 {
        return Err(EvaluationReportError::Contract("run-count-not-one"));
    }
    let run = runs
        .pop()
        .ok_or(EvaluationReportError::Contract("run-count-not-one"))?;
    if ok != run.ok || pass_rate != if run.ok { "1/1" } else { "0/1" } {
        return Err(EvaluationReportError::Contract("result-outcome-mismatch"));
    }
    decode_sample(host, scenario, run, expected_source_revision)
}

fn decode_sample(
    host: EvaluationHost,
    scenario: EvaluationScenario,
    run: RawSample,
    expected_source_revision: &str,
) -> Result<DecodedEvaluationSample, EvaluationReportError> {
    let RawSample {
        ok,
        latency_ms,
        exit_reason,
        metric_status,
        token_usage,
        tool_calls,
        tool_breakdown,
        classified,
        checks,
        failures,
        work_dir,
        session_id,
        transcript_digest,
        transcript_retention,
        transcript_path,
    } = run;
    validate_metrics(
        latency_ms,
        metric_status,
        token_usage,
        tool_calls,
        tool_breakdown,
        classified,
    );
    drop(session_id);
    validate_retention(
        transcript_retention,
        transcript_digest.as_deref(),
        transcript_path.as_deref(),
    )?;
    if !ok {
        let detail = if failures.is_empty() {
            "unspecified failure".to_owned()
        } else {
            failures.join(", ")
        };
        return Ok(DecodedEvaluationSample::Failed {
            scenario,
            diagnostic: format!(
                "{}:{}:{}:{detail}",
                host.as_str(),
                scenario.as_str(),
                exit_reason.as_str()
            ),
        });
    }
    if exit_reason != RawExitReason::Complete {
        return Err(EvaluationReportError::Contract(
            "successful-run-not-complete",
        ));
    }
    if transcript_retention != RawTranscriptRetention::Retained {
        return Err(EvaluationReportError::Contract(
            "successful-run-not-retained",
        ));
    }
    let transcript_path = transcript_path.ok_or(EvaluationReportError::Contract(
        "retained-transcript-path-missing",
    ))?;
    let transcript_digest = transcript_digest.ok_or(EvaluationReportError::Contract(
        "retained-transcript-digest-missing",
    ))?;
    decode_retained_sample(
        host,
        scenario,
        work_dir,
        transcript_path,
        transcript_digest,
        checks,
        expected_source_revision,
    )
}

fn decode_retained_sample(
    host: EvaluationHost,
    scenario: EvaluationScenario,
    work_dir: String,
    transcript_path: String,
    transcript_digest: String,
    mut checks: BTreeMap<String, Value>,
    expected_source_revision: &str,
) -> Result<DecodedEvaluationSample, EvaluationReportError> {
    let result_value = checks
        .remove("evaluation_result")
        .ok_or(EvaluationReportError::Contract("evaluation-result-missing"))?;
    let observation: EvaluationResultObservation =
        serde_json::from_value(result_value).map_err(|error| {
            EvaluationReportError::InvalidReport {
                detail: error.to_string(),
            }
        })?;
    if observation.host != host.as_str() {
        return Err(EvaluationReportError::Contract("result-host-mismatch"));
    }
    if observation.source_revision != expected_source_revision {
        return Err(EvaluationReportError::Contract(
            "result-source-revision-mismatch",
        ));
    }
    let envelope = EvaluationEnvelope {
        host,
        scenario,
        execution_status: ExecutionStatus::Executed,
        passed: true,
        suite_revision: observation.suite_revision,
        fixture_revision: observation.fixture_revision,
        source_revision: observation.source_revision,
        host_version: Some(observation.host_version),
        governing: Some(observation.governing),
        transcript_path: Some(transcript_path.clone()),
        transcript_digest: Some(transcript_digest.clone()),
        command_count: Some(observation.command_count),
        elapsed_ms: Some(observation.elapsed_ms),
        human_prompt_count: Some(observation.human_prompt_count),
        manual_translation_count: Some(observation.manual_translation_count),
        repeated_prompt_count: Some(observation.repeated_prompt_count),
        observed_outcome: Some(observation.observed_outcome),
        terminal_event: observation.terminal_event,
        unsupported_additions: observation.unsupported_additions,
        diagnostic: None,
    };
    Ok(DecodedEvaluationSample::Retained {
        work_dir,
        transcript_path,
        transcript_digest,
        envelope: Box::new(envelope),
    })
}

fn validate_retention(
    retention: RawTranscriptRetention,
    digest: Option<&str>,
    path: Option<&str>,
) -> Result<(), EvaluationReportError> {
    let valid = match retention {
        RawTranscriptRetention::Retained => {
            digest.is_some_and(is_lower_sha256_digest)
                && path.is_some_and(is_safe_transcript_reference)
        }
        RawTranscriptRetention::NotRetained => {
            digest.is_some_and(is_lower_sha256_digest) && path.is_none()
        }
        RawTranscriptRetention::Unavailable => digest.is_none() && path.is_none(),
    };
    if valid {
        Ok(())
    } else {
        Err(EvaluationReportError::Contract(
            "transcript-retention-identity-mismatch",
        ))
    }
}

fn parse_host(value: &str) -> Result<EvaluationHost, EvaluationReportError> {
    match value {
        "claude" => Ok(EvaluationHost::Claude),
        "codex" => Ok(EvaluationHost::Codex),
        "opencode" => Ok(EvaluationHost::Opencode),
        "copilot" => Ok(EvaluationHost::Copilot),
        _ => Err(EvaluationReportError::Contract("host-unsupported")),
    }
}

fn parse_scenario(value: &str) -> Result<EvaluationScenario, EvaluationReportError> {
    match value {
        "existing-profile" => Ok(EvaluationScenario::ExistingProfile),
        "no-profile" => Ok(EvaluationScenario::NoProfile),
        "malformed-producer" => Ok(EvaluationScenario::MalformedProducer),
        "unavailable-producer" => Ok(EvaluationScenario::UnavailableProducer),
        "interruption-resume" => Ok(EvaluationScenario::InterruptionResume),
        "human-acceptance" => Ok(EvaluationScenario::HumanAcceptance),
        "human-rejection" => Ok(EvaluationScenario::HumanRejection),
        _ => Err(EvaluationReportError::Contract("scenario-unsupported")),
    }
}

fn validate_summaries(summaries: BTreeMap<String, RawSummary>) {
    for RawSummary { p50, p95 } in summaries.into_values() {
        let _ = (p50, p95);
    }
}

fn validate_metrics(
    latency_ms: f64,
    metric_status: RawMetricStatus,
    token_usage: RawTokenUsage,
    tool_calls: Option<f64>,
    tool_breakdown: BTreeMap<String, f64>,
    classified: BTreeMap<String, f64>,
) {
    let RawTokenUsage {
        input,
        output,
        cache_creation,
        cache_read,
        context_input,
        total,
    } = token_usage;
    drop((
        latency_ms,
        metric_status,
        input,
        output,
        cache_creation,
        cache_read,
        context_input,
        total,
        tool_calls,
        tool_breakdown,
        classified,
    ));
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawReport {
    report_version: String,
    ok: bool,
    generated_at: String,
    suite: String,
    #[serde(default, deserialize_with = "deserialize_optional_nonnull")]
    agent: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_nonnull")]
    model: Option<String>,
    repeats: u64,
    results: Vec<RawResult>,
    aggregates: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawResult {
    id: String,
    #[serde(default, deserialize_with = "deserialize_optional_nonnull")]
    use_case: Option<String>,
    ok: bool,
    pass_rate: String,
    aggregate: BTreeMap<String, RawSummary>,
    runs: Vec<RawSample>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawSummary {
    #[serde(deserialize_with = "deserialize_nullable")]
    p50: Option<f64>,
    #[serde(deserialize_with = "deserialize_nullable")]
    p95: Option<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawSample {
    ok: bool,
    latency_ms: f64,
    exit_reason: RawExitReason,
    metric_status: RawMetricStatus,
    token_usage: RawTokenUsage,
    #[serde(deserialize_with = "deserialize_nullable")]
    tool_calls: Option<f64>,
    tool_breakdown: BTreeMap<String, f64>,
    classified: BTreeMap<String, f64>,
    checks: BTreeMap<String, Value>,
    failures: Vec<String>,
    work_dir: String,
    session_id: String,
    #[serde(deserialize_with = "deserialize_nullable")]
    transcript_digest: Option<String>,
    transcript_retention: RawTranscriptRetention,
    #[serde(default, deserialize_with = "deserialize_optional_nonnull")]
    transcript_path: Option<String>,
}

fn deserialize_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

fn deserialize_optional_nonnull<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawTokenUsage {
    input: f64,
    output: f64,
    cache_creation: f64,
    cache_read: f64,
    context_input: f64,
    total: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum RawTranscriptRetention {
    Retained,
    NotRetained,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum RawMetricStatus {
    Available,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum RawExitReason {
    Complete,
    Failed,
    Timeout,
    Exit,
    Error,
}

impl RawExitReason {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Failed => "failed",
            Self::Timeout => "timeout",
            Self::Exit => "exit",
            Self::Error => "error",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluationResultObservation {
    host: String,
    host_version: String,
    source_revision: String,
    suite_revision: String,
    fixture_revision: String,
    governing: GoverningVersions,
    command_count: i64,
    elapsed_ms: i64,
    human_prompt_count: i64,
    manual_translation_count: i64,
    repeated_prompt_count: i64,
    observed_outcome: String,
    terminal_event: Option<DecisionEvent>,
    unsupported_additions: Vec<String>,
}
