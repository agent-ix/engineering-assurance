// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Pure evaluation-envelope validation and complete-only matrix aggregation.
//!
//! This module owns no host execution, filesystem access, transcript loading,
//! evidence persistence, current-revision observation, or release decision.

use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    evidence::GoverningVersions,
    workflow::{DecisionChoice, DecisionEvent},
};

/// Protocol accepted by the pure evaluation aggregation boundary.
pub const REQUEST_PROTOCOL: &str = "engineering-assurance.evaluation-aggregate-request/v1";

/// Protocol emitted by a successful evaluation aggregation operation.
pub const RESULT_PROTOCOL: &str = "engineering-assurance.evaluation-aggregate-result/v1";

/// Maximum accepted serialized request size.
pub const MAX_REQUEST_BYTES: usize = 8 * 1024 * 1024;

const REQUIRED_CELL_COUNT: usize = EvaluationHost::ALL.len() * EvaluationScenario::ALL.len();

/// One supported agent host in canonical matrix order.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EvaluationHost {
    /// Anthropic Claude Code.
    Claude,
    /// `OpenAI` Codex.
    Codex,
    /// `OpenCode`.
    Opencode,
    /// GitHub Copilot.
    Copilot,
}

impl EvaluationHost {
    /// Closed supported host population in canonical order.
    pub const ALL: [Self; 4] = [Self::Claude, Self::Codex, Self::Opencode, Self::Copilot];

    /// Return the stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Opencode => "opencode",
            Self::Copilot => "copilot",
        }
    }
}

/// One required evaluation scenario in canonical matrix order.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvaluationScenario {
    /// Reuse an existing applicable profile.
    ExistingProfile,
    /// Correctly find no applicable profile work.
    NoProfile,
    /// Preserve a malformed producer outcome.
    MalformedProducer,
    /// Preserve an unavailable producer outcome.
    UnavailableProducer,
    /// Resume an interrupted workflow.
    InterruptionResume,
    /// Retain an explicit human acceptance.
    HumanAcceptance,
    /// Retain an explicit human rejection.
    HumanRejection,
}

impl EvaluationScenario {
    /// Closed scenario population in canonical order.
    pub const ALL: [Self; 7] = [
        Self::ExistingProfile,
        Self::NoProfile,
        Self::MalformedProducer,
        Self::UnavailableProducer,
        Self::InterruptionResume,
        Self::HumanAcceptance,
        Self::HumanRejection,
    ];

    /// Return the stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExistingProfile => "existing-profile",
            Self::NoProfile => "no-profile",
            Self::MalformedProducer => "malformed-producer",
            Self::UnavailableProducer => "unavailable-producer",
            Self::InterruptionResume => "interruption-resume",
            Self::HumanAcceptance => "human-acceptance",
            Self::HumanRejection => "human-rejection",
        }
    }

    /// Return the outcome required for a complete cell.
    #[must_use]
    pub const fn expected_outcome(self) -> &'static str {
        match self {
            Self::ExistingProfile => "reused",
            Self::NoProfile => "no-applicable-work",
            Self::MalformedProducer => "validation-failure",
            Self::UnavailableProducer => "unavailable",
            Self::InterruptionResume => "resumed",
            Self::HumanAcceptance => "accepted",
            Self::HumanRejection => "rejected",
        }
    }

    const fn terminal_choice(self) -> Option<DecisionChoice> {
        match self {
            Self::HumanAcceptance => Some(DecisionChoice::Accept),
            Self::HumanRejection => Some(DecisionChoice::Reject),
            Self::ExistingProfile
            | Self::NoProfile
            | Self::MalformedProducer
            | Self::UnavailableProducer
            | Self::InterruptionResume => None,
        }
    }
}

/// Whether one required scenario actually executed.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// The scenario ran and supplied an observation.
    Executed,
    /// The scenario did not run and cannot satisfy the aggregate.
    NotExecuted,
}

/// One typed completeness failure for an individual evaluation envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnvelopeFailure {
    /// The suite revision is blank or mutable.
    SuiteRevisionNotImmutable,
    /// The fixture revision is blank or mutable.
    FixtureRevisionNotImmutable,
    /// The source revision is blank or mutable.
    SourceRevisionNotImmutable,
    /// The required scenario did not execute.
    ScenarioNotExecuted,
    /// A non-executed scenario has no diagnostic.
    NotExecutedDiagnosticMissing,
    /// The host version is absent, blank, or mutable.
    HostVersionNotImmutable,
    /// No complete governing tuple was supplied.
    GoverningVersionsMissing,
    /// One governing identity failed its existing typed validator.
    GoverningIdentity(String),
    /// The transcript path is absent, absolute, or traversing.
    TranscriptPathInvalid,
    /// The transcript digest is not lowercase SHA-256.
    TranscriptDigestInvalid,
    /// The command count is absent or negative.
    CommandCountInvalid,
    /// The elapsed duration is absent or negative.
    ElapsedCountInvalid,
    /// The human-prompt count is absent or negative.
    HumanPromptCountInvalid,
    /// The manual-translation count is absent or negative.
    ManualTranslationCountInvalid,
    /// The repeated-prompt count is absent or negative.
    RepeatedPromptCountInvalid,
    /// The observed outcome is absent or does not match the scenario.
    ObservedOutcomeMismatch,
    /// The evaluator reported a first-party addition outside the contract.
    UnsupportedAssuranceAddition,
    /// A non-decision scenario supplied a terminal event.
    UnexpectedTerminalEvent,
    /// A human-decision scenario supplied no terminal event.
    TerminalEventMissing,
    /// A terminal event has an invalid identity, choice, outcome, or timestamp.
    TerminalEventInvalid,
    /// The host reported the scenario as failed.
    ScenarioFailed,
}

impl EnvelopeFailure {
    /// Return the stable wire code.
    #[must_use]
    pub fn code(&self) -> &str {
        match self {
            Self::SuiteRevisionNotImmutable => "suite-revision-not-immutable",
            Self::FixtureRevisionNotImmutable => "fixture-revision-not-immutable",
            Self::SourceRevisionNotImmutable => "source-revision-not-immutable",
            Self::ScenarioNotExecuted => "scenario-not-executed",
            Self::NotExecutedDiagnosticMissing => "not-executed-diagnostic-missing",
            Self::HostVersionNotImmutable => "host-version-not-immutable",
            Self::GoverningVersionsMissing => "governing-versions-missing",
            Self::GoverningIdentity(code) => code,
            Self::TranscriptPathInvalid => "transcript-path-invalid",
            Self::TranscriptDigestInvalid => "transcript-digest-invalid",
            Self::CommandCountInvalid => "command-count-invalid",
            Self::ElapsedCountInvalid => "elapsed-count-invalid",
            Self::HumanPromptCountInvalid => "human-prompt-count-invalid",
            Self::ManualTranslationCountInvalid => "manual-translation-count-invalid",
            Self::RepeatedPromptCountInvalid => "repeated-prompt-count-invalid",
            Self::ObservedOutcomeMismatch => "observed-outcome-mismatch",
            Self::UnsupportedAssuranceAddition => "unsupported-assurance-addition",
            Self::UnexpectedTerminalEvent => "unexpected-terminal-event",
            Self::TerminalEventMissing => "terminal-event-missing",
            Self::TerminalEventInvalid => "terminal-event-invalid",
            Self::ScenarioFailed => "scenario-failed",
        }
    }
}

/// One typed matrix-level failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvaluationFailure {
    /// A required cell is absent.
    Missing(EvaluationCell),
    /// A required cell appears more than once.
    Duplicate(EvaluationCell),
    /// One unique cell is incomplete or invalid.
    Envelope {
        /// Cell containing the failure.
        cell: EvaluationCell,
        /// Typed envelope failure.
        failure: EnvelopeFailure,
    },
    /// Source revisions differ across the matrix.
    MatrixSourceRevisionMismatch,
    /// Suite revisions differ across the matrix.
    MatrixSuiteRevisionMismatch,
    /// Fixture revisions differ across the matrix.
    MatrixFixtureRevisionMismatch,
    /// Non-workflow governing identities differ across the matrix.
    MatrixGoverningVersionsMismatch,
    /// Workflow identities differ within one scenario.
    ScenarioWorkflowVersionMismatch(EvaluationScenario),
    /// A host's acceptance/rejection pair is not equivalent and distinct.
    TerminalPairInvalid(EvaluationHost),
}

impl fmt::Display for EvaluationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(cell) => write!(
                formatter,
                "missing:{}:{}",
                cell.host.as_str(),
                cell.scenario.as_str()
            ),
            Self::Duplicate(cell) => write!(
                formatter,
                "duplicate:{}:{}",
                cell.host.as_str(),
                cell.scenario.as_str()
            ),
            Self::Envelope { cell, failure } => write!(
                formatter,
                "{}:{}:{}",
                cell.host.as_str(),
                cell.scenario.as_str(),
                failure.code()
            ),
            Self::MatrixSourceRevisionMismatch => {
                formatter.write_str("matrix-source-revision-mismatch")
            }
            Self::MatrixSuiteRevisionMismatch => {
                formatter.write_str("matrix-suite-revision-mismatch")
            }
            Self::MatrixFixtureRevisionMismatch => {
                formatter.write_str("matrix-fixture-revision-mismatch")
            }
            Self::MatrixGoverningVersionsMismatch => {
                formatter.write_str("matrix-governing-versions-mismatch")
            }
            Self::ScenarioWorkflowVersionMismatch(scenario) => {
                write!(formatter, "{}:workflow-version-mismatch", scenario.as_str())
            }
            Self::TerminalPairInvalid(host) => {
                write!(formatter, "{}:terminal-pair-invalid", host.as_str())
            }
        }
    }
}

impl Serialize for EvaluationFailure {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Canonical identity of one required matrix cell.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EvaluationCell {
    /// Agent host.
    pub host: EvaluationHost,
    /// Evaluation scenario.
    pub scenario: EvaluationScenario,
}

/// One retained evaluation envelope before completeness validation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationEnvelope {
    /// Agent host.
    pub host: EvaluationHost,
    /// Evaluated scenario.
    pub scenario: EvaluationScenario,
    /// Whether the scenario executed.
    pub execution_status: ExecutionStatus,
    /// Whether the host reported the scenario as passing.
    pub passed: bool,
    /// Immutable suite revision.
    pub suite_revision: String,
    /// Immutable fixture revision.
    pub fixture_revision: String,
    /// Immutable source revision.
    pub source_revision: String,
    /// Exact host version, required for executed cells.
    pub host_version: Option<String>,
    /// Complete governing identity tuple, required for executed cells.
    pub governing: Option<GoverningVersions>,
    /// Relative retained transcript path, required for executed cells.
    pub transcript_path: Option<String>,
    /// Lowercase SHA-256 transcript digest, required for executed cells.
    pub transcript_digest: Option<String>,
    /// Commands issued during the scenario.
    pub command_count: Option<i64>,
    /// Elapsed wall time in milliseconds.
    pub elapsed_ms: Option<i64>,
    /// Human prompts observed.
    pub human_prompt_count: Option<i64>,
    /// Manual translations observed.
    pub manual_translation_count: Option<i64>,
    /// Repeated prompts observed.
    pub repeated_prompt_count: Option<i64>,
    /// Scenario outcome observed by the evaluator.
    pub observed_outcome: Option<String>,
    /// Explicit terminal event for a human-decision scenario.
    pub terminal_event: Option<DecisionEvent>,
    /// First-party assurance behavior added outside the expected scenario.
    pub unsupported_additions: Vec<String>,
    /// Required explanation when the scenario did not execute.
    pub diagnostic: Option<String>,
}

impl EvaluationEnvelope {
    /// Return this envelope's canonical cell identity.
    #[must_use]
    pub const fn cell(&self) -> EvaluationCell {
        EvaluationCell {
            host: self.host,
            scenario: self.scenario,
        }
    }

    /// Return every stable completeness failure in contract order.
    #[must_use]
    pub fn errors(&self) -> Vec<EnvelopeFailure> {
        let mut errors = Vec::new();
        for (failure, revision) in [
            (
                EnvelopeFailure::SuiteRevisionNotImmutable,
                self.suite_revision.as_str(),
            ),
            (
                EnvelopeFailure::FixtureRevisionNotImmutable,
                self.fixture_revision.as_str(),
            ),
            (
                EnvelopeFailure::SourceRevisionNotImmutable,
                self.source_revision.as_str(),
            ),
        ] {
            if !is_immutable_revision(revision) {
                errors.push(failure);
            }
        }

        if self.execution_status == ExecutionStatus::NotExecuted {
            errors.push(EnvelopeFailure::ScenarioNotExecuted);
            if self.diagnostic.as_deref().is_none_or(str::is_empty) {
                errors.push(EnvelopeFailure::NotExecutedDiagnosticMissing);
            }
            return errors;
        }

        if self
            .host_version
            .as_deref()
            .is_none_or(|version| !is_immutable_revision(version))
        {
            errors.push(EnvelopeFailure::HostVersionNotImmutable);
        }
        match self.governing.as_ref() {
            Some(governing) => errors.extend(
                governing
                    .errors()
                    .into_iter()
                    .map(EnvelopeFailure::GoverningIdentity),
            ),
            None => errors.push(EnvelopeFailure::GoverningVersionsMissing),
        }
        if self
            .transcript_path
            .as_deref()
            .is_none_or(|path| !is_safe_transcript_reference(path))
        {
            errors.push(EnvelopeFailure::TranscriptPathInvalid);
        }
        if self
            .transcript_digest
            .as_deref()
            .is_none_or(|digest| !is_lower_sha256_digest(digest))
        {
            errors.push(EnvelopeFailure::TranscriptDigestInvalid);
        }
        for (failure, count) in [
            (EnvelopeFailure::CommandCountInvalid, self.command_count),
            (EnvelopeFailure::ElapsedCountInvalid, self.elapsed_ms),
            (
                EnvelopeFailure::HumanPromptCountInvalid,
                self.human_prompt_count,
            ),
            (
                EnvelopeFailure::ManualTranslationCountInvalid,
                self.manual_translation_count,
            ),
            (
                EnvelopeFailure::RepeatedPromptCountInvalid,
                self.repeated_prompt_count,
            ),
        ] {
            if count.is_none_or(|value| value < 0) {
                errors.push(failure);
            }
        }
        if self.observed_outcome.as_deref() != Some(self.scenario.expected_outcome()) {
            errors.push(EnvelopeFailure::ObservedOutcomeMismatch);
        }
        if !self.unsupported_additions.is_empty() {
            errors.push(EnvelopeFailure::UnsupportedAssuranceAddition);
        }
        errors.extend(self.terminal_errors());
        if !self.passed {
            errors.push(EnvelopeFailure::ScenarioFailed);
        }
        errors
    }

    fn terminal_errors(&self) -> Vec<EnvelopeFailure> {
        let Some(choice) = self.scenario.terminal_choice() else {
            return if self.terminal_event.is_some() {
                vec![EnvelopeFailure::UnexpectedTerminalEvent]
            } else {
                Vec::new()
            };
        };
        let Some(event) = self.terminal_event.as_ref() else {
            return vec![EnvelopeFailure::TerminalEventMissing];
        };
        let governing_workflow = self.governing.as_ref().map(|value| &value.workflow);
        let valid = event.choice == choice
            && event.outcome == self.scenario.expected_outcome()
            && !event.owner.trim().is_empty()
            && is_safe_run_id(&event.run_id)
            && governing_workflow.is_some_and(|workflow| {
                event.workflow == workflow.name && event.workflow_version == workflow.version
            })
            && OffsetDateTime::parse(&event.timestamp, &Rfc3339).is_ok();
        if valid {
            Vec::new()
        } else {
            vec![EnvelopeFailure::TerminalEventInvalid]
        }
    }
}

/// Versioned pure aggregation request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationAggregateRequest {
    /// Exact request protocol discriminator.
    pub protocol: String,
    /// Attempted evaluation cells.
    pub envelopes: Vec<EvaluationEnvelope>,
}

/// Deterministic complete-only aggregation result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationAggregateResult {
    /// Exact result protocol discriminator.
    pub protocol: &'static str,
    /// True only for one complete, valid 28-cell matrix.
    pub ok: bool,
    /// Required cell count for this protocol.
    pub required_cells: usize,
    /// Unique cells that passed complete envelope validation.
    pub complete_cells: usize,
    /// Stable failures in deterministic contract order.
    pub failures: Vec<EvaluationFailure>,
}

impl EvaluationAggregateResult {
    /// Serialize one newline-terminated machine result.
    ///
    /// # Errors
    ///
    /// Returns [`EvaluationError::ResultSerialization`] if serialization fails.
    pub fn to_json_line(&self) -> Result<Vec<u8>, EvaluationError> {
        let mut encoded = serde_json::to_vec(self).map_err(EvaluationError::ResultSerialization)?;
        encoded.push(b'\n');
        Ok(encoded)
    }
}

/// Stable request-boundary failures.
#[derive(Debug, Error)]
pub enum EvaluationError {
    /// The input was not one closed v1 request.
    #[error("evaluation aggregate request is invalid: {0}")]
    InvalidRequest(serde_json::Error),
    /// The serialized request exceeds the fixed byte ceiling.
    #[error("evaluation aggregate request is {actual} bytes; limit is {limit} bytes")]
    RequestTooLarge {
        /// Observed input length.
        actual: usize,
        /// Maximum accepted input length.
        limit: usize,
    },
    /// The protocol discriminator is not supported.
    #[error("unsupported evaluation aggregate protocol: {0}")]
    UnsupportedProtocol(String),
    /// A result could not be encoded.
    #[error("evaluation aggregate result serialization failed: {0}")]
    ResultSerialization(serde_json::Error),
}

impl EvaluationError {
    /// Return the stable machine error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequest(_) => "evaluation_aggregate_request_invalid",
            Self::RequestTooLarge { .. } => "evaluation_aggregate_request_too_large",
            Self::UnsupportedProtocol(_) => "evaluation_aggregate_protocol_unsupported",
            Self::ResultSerialization(_) => "evaluation_aggregate_result_serialization_failed",
        }
    }
}

/// Return the exact required host-scenario matrix in canonical order.
#[must_use]
pub fn required_matrix() -> Vec<EvaluationCell> {
    EvaluationHost::ALL
        .into_iter()
        .flat_map(|host| {
            EvaluationScenario::ALL
                .into_iter()
                .map(move |scenario| EvaluationCell { host, scenario })
        })
        .collect()
}

/// Parse and aggregate one versioned pure request.
///
/// # Errors
///
/// Returns [`EvaluationError`] for malformed, structurally open, or
/// unsupported-version requests. Semantically incomplete cells produce a
/// successful result whose `ok` field is false.
pub fn evaluate_request_bytes(input: &[u8]) -> Result<EvaluationAggregateResult, EvaluationError> {
    if input.len() > MAX_REQUEST_BYTES {
        return Err(EvaluationError::RequestTooLarge {
            actual: input.len(),
            limit: MAX_REQUEST_BYTES,
        });
    }
    let request: EvaluationAggregateRequest =
        serde_json::from_slice(input).map_err(EvaluationError::InvalidRequest)?;
    if request.protocol != REQUEST_PROTOCOL {
        return Err(EvaluationError::UnsupportedProtocol(request.protocol));
    }
    Ok(aggregate_evaluations(&request.envelopes))
}

/// Aggregate typed envelopes without performing I/O or inferring a decision.
#[must_use]
pub fn aggregate_evaluations(envelopes: &[EvaluationEnvelope]) -> EvaluationAggregateResult {
    let mut grouped: BTreeMap<EvaluationCell, (usize, &EvaluationEnvelope)> = BTreeMap::new();
    for envelope in envelopes {
        grouped
            .entry(envelope.cell())
            .and_modify(|(count, _)| *count = count.saturating_add(1))
            .or_insert((1, envelope));
    }

    let mut failures = Vec::new();
    let required = required_matrix();
    for cell in &required {
        if !grouped.contains_key(cell) {
            failures.push(EvaluationFailure::Missing(*cell));
        }
    }

    let mut complete_cells = 0;
    for cell in &required {
        let Some(selected) = grouped.get(cell) else {
            continue;
        };
        if selected.0 != 1 {
            failures.push(EvaluationFailure::Duplicate(*cell));
            continue;
        }
        let errors = selected.1.errors();
        if errors.is_empty() {
            complete_cells += 1;
        } else {
            failures.extend(
                errors
                    .into_iter()
                    .map(|failure| EvaluationFailure::Envelope {
                        cell: *cell,
                        failure,
                    }),
            );
        }
    }

    append_aggregate_identity_failures(envelopes, &mut failures);
    append_scenario_workflow_failures(envelopes, &mut failures);
    append_terminal_pair_failures(&grouped, &mut failures);

    EvaluationAggregateResult {
        protocol: RESULT_PROTOCOL,
        ok: failures.is_empty() && complete_cells == REQUIRED_CELL_COUNT,
        required_cells: REQUIRED_CELL_COUNT,
        complete_cells,
        failures,
    }
}

fn append_aggregate_identity_failures(
    envelopes: &[EvaluationEnvelope],
    failures: &mut Vec<EvaluationFailure>,
) {
    if values_differ(envelopes, |item| &item.source_revision) {
        failures.push(EvaluationFailure::MatrixSourceRevisionMismatch);
    }
    if values_differ(envelopes, |item| &item.suite_revision) {
        failures.push(EvaluationFailure::MatrixSuiteRevisionMismatch);
    }
    if values_differ(envelopes, |item| &item.fixture_revision) {
        failures.push(EvaluationFailure::MatrixFixtureRevisionMismatch);
    }
    let first = envelopes.first().and_then(|item| item.governing.as_ref());
    if envelopes
        .iter()
        .skip(1)
        .any(|item| !same_shared_governing(first, item.governing.as_ref()))
    {
        failures.push(EvaluationFailure::MatrixGoverningVersionsMismatch);
    }
}

fn append_scenario_workflow_failures(
    envelopes: &[EvaluationEnvelope],
    failures: &mut Vec<EvaluationFailure>,
) {
    for scenario in EvaluationScenario::ALL {
        let mut workflows = envelopes
            .iter()
            .filter(|item| item.scenario == scenario)
            .map(|item| item.governing.as_ref().map(|value| &value.workflow));
        let first = workflows.next();
        if workflows.any(|workflow| workflow != first.flatten()) {
            failures.push(EvaluationFailure::ScenarioWorkflowVersionMismatch(scenario));
        }
    }
}

fn append_terminal_pair_failures(
    grouped: &BTreeMap<EvaluationCell, (usize, &EvaluationEnvelope)>,
    failures: &mut Vec<EvaluationFailure>,
) {
    for host in EvaluationHost::ALL {
        let acceptance = grouped.get(&EvaluationCell {
            host,
            scenario: EvaluationScenario::HumanAcceptance,
        });
        let rejection = grouped.get(&EvaluationCell {
            host,
            scenario: EvaluationScenario::HumanRejection,
        });
        let (Some(acceptance), Some(rejection)) = (acceptance, rejection) else {
            continue;
        };
        if acceptance.0 != 1 || rejection.0 != 1 {
            continue;
        }
        if !valid_terminal_pair(acceptance.1, rejection.1) {
            failures.push(EvaluationFailure::TerminalPairInvalid(host));
        }
    }
}

fn valid_terminal_pair(acceptance: &EvaluationEnvelope, rejection: &EvaluationEnvelope) -> bool {
    let (Some(acceptance_event), Some(rejection_event)) = (
        acceptance.terminal_event.as_ref(),
        rejection.terminal_event.as_ref(),
    ) else {
        return false;
    };
    acceptance_event.choice == DecisionChoice::Accept
        && rejection_event.choice == DecisionChoice::Reject
        && acceptance_event.run_id != rejection_event.run_id
        && acceptance.fixture_revision == rejection.fixture_revision
        && acceptance.source_revision == rejection.source_revision
        && acceptance.governing == rejection.governing
}

fn values_differ<T: PartialEq>(
    values: &[EvaluationEnvelope],
    select: impl Fn(&EvaluationEnvelope) -> &T,
) -> bool {
    let Some(first) = values.first().map(&select) else {
        return false;
    };
    values.iter().skip(1).any(|value| select(value) != first)
}

fn same_shared_governing(
    left: Option<&GoverningVersions>,
    right: Option<&GoverningVersions>,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.module == right.module
                && left.plugin == right.plugin
                && left.skill == right.skill
                && left.quire == right.quire
                && left.quoin == right.quoin
                && left.ix_flow == right.ix_flow
                && left.schema == right.schema
                && left.producer == right.producer
        }
        (None, Some(_)) | (Some(_), None) => false,
    }
}

/// Return whether a revision or version is nonblank and contains no mutable selector.
#[must_use]
pub fn is_immutable_revision(value: &str) -> bool {
    !value.trim().is_empty() && !contains_mutable_revision(value)
}

fn contains_mutable_revision(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    value
        .bytes()
        .any(|byte| matches!(byte, b'>' | b'<' | b'^' | b'~' | b'*'))
        || lower
            .split(['.', '-', '_'])
            .any(|component| component == "x")
        || ["latest", "main", "master", "head", "current", "nightly"]
            .into_iter()
            .any(|candidate| {
                lower.match_indices(candidate).any(|(start, matched)| {
                    let end = start + matched.len();
                    boundary(
                        lower.as_bytes().get(start.wrapping_sub(1)).copied(),
                        start == 0,
                    ) && boundary(lower.as_bytes().get(end).copied(), end == lower.len())
                })
            })
}

fn boundary(byte: Option<u8>, at_edge: bool) -> bool {
    at_edge || byte.is_some_and(|value| matches!(value, b'-' | b'_' | b'.'))
}

/// Return whether a transcript reference uses the portable relative grammar.
#[must_use]
pub fn is_safe_transcript_reference(value: &str) -> bool {
    if value.is_empty()
        || value.starts_with('/')
        || value.contains('\\')
        || value.bytes().any(|byte| byte.is_ascii_control())
        || value
            .as_bytes()
            .get(..2)
            .is_some_and(|prefix| prefix[0].is_ascii_alphabetic() && prefix[1] == b':')
    {
        return false;
    }
    value
        .split('/')
        .all(|component| !component.is_empty() && !matches!(component, "." | ".."))
}

/// Return whether a digest is exactly 64 lowercase hexadecimal characters.
#[must_use]
pub fn is_lower_sha256_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_safe_run_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=128).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}
