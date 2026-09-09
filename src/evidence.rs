// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Pure evidence-availability classification with immutable producer provenance.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

const MUTABLE_VERSIONS: [&str; 7] = [
    "latest", "main", "master", "head", "current", "nightly", "*",
];
const GOVERNING_FIELDS: [&str; 9] = [
    "module", "plugin", "skill", "workflow", "quire", "quoin", "ix_flow", "schema", "producer",
];

/// One closed evidence-availability state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityState {
    /// Valid output was observed with complete immutable provenance.
    Observed,
    /// The producer was invoked but could not produce an observation.
    Unavailable,
    /// The producer was applicable but was not invoked.
    NotComputed,
    /// The producer was outside the selected boundary.
    NotApplicable,
}

impl AvailabilityState {
    /// Return the stable wire spelling retained from the reference implementation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Unavailable => "unavailable",
            Self::NotComputed => "not_computed",
            Self::NotApplicable => "not_applicable",
        }
    }
}

/// Failure returned when zero, duplicate, conflicting, or unknown state labels are supplied.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("{message}")]
pub struct EvidenceValidationError {
    message: String,
}

/// Return the one valid availability state or reject an ambiguous selection.
///
/// # Errors
///
/// Returns [`EvidenceValidationError`] unless `labels` contains exactly one
/// supported state label.
pub fn validate_state_labels<I, S>(labels: I) -> Result<AvailabilityState, EvidenceValidationError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let selected: Vec<String> = labels
        .into_iter()
        .map(|label| label.as_ref().to_owned())
        .collect();
    if selected.len() != 1 {
        return Err(EvidenceValidationError {
            message: format!(
                "exactly one availability state is required: {}",
                format_labels(&selected)
            ),
        });
    }
    match selected[0].as_str() {
        "observed" => Ok(AvailabilityState::Observed),
        "unavailable" => Ok(AvailabilityState::Unavailable),
        "not_computed" => Ok(AvailabilityState::NotComputed),
        "not_applicable" => Ok(AvailabilityState::NotApplicable),
        state => Err(EvidenceValidationError {
            message: format!("unsupported availability state: {state}"),
        }),
    }
}

fn format_labels(labels: &[String]) -> String {
    let values = labels
        .iter()
        .map(|label| format!("'{label}'"))
        .collect::<Vec<_>>()
        .join(", ");
    match labels {
        [] => "()".to_owned(),
        [_] => format!("({values},)"),
        _ => format!("({values})"),
    }
}

/// Immutable identity of one governing component.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VersionIdentity {
    /// Stable component name.
    pub name: String,
    /// Exact immutable version.
    pub version: String,
    /// Lowercase SHA-256 digest of the governed bytes.
    pub digest: String,
}

impl VersionIdentity {
    /// Return every stable validation failure in reference order.
    #[must_use]
    pub fn errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push("identity-name-missing".to_owned());
        }
        let version = self.version.trim().to_lowercase();
        if version.is_empty() {
            errors.push("identity-version-missing".to_owned());
        } else if MUTABLE_VERSIONS.contains(&version.as_str())
            || ['>', '<', '^', '~', 'x']
                .into_iter()
                .any(|marker| version.contains(marker))
        {
            errors.push("identity-version-mutable".to_owned());
        }
        if !is_lower_sha256(&self.digest) {
            errors.push("identity-digest-not-sha256".to_owned());
        }
        errors
    }
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn legacy_canonical_json(output: &Value) -> Vec<u8> {
    let serde_encoded = output.to_string();
    let bytes = serde_encoded.as_bytes();
    let mut canonical = Vec::with_capacity(bytes.len());
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if in_string {
            canonical.push(byte);
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            index += 1;
            continue;
        }
        if byte == b'"' {
            in_string = true;
            canonical.push(b'"');
            index += 1;
            continue;
        }
        if byte == b'-' || byte.is_ascii_digit() {
            let number_start = index;
            index += 1;
            while bytes.get(index).is_some_and(|candidate| {
                candidate.is_ascii_digit() || matches!(candidate, b'.' | b'e' | b'E' | b'+' | b'-')
            }) {
                index += 1;
            }
            canonical.extend_from_slice(&python_number_token(&bytes[number_start..index]));
            continue;
        }
        canonical.push(byte);
        index += 1;
    }
    canonical
}

fn python_number_token(token: &[u8]) -> Vec<u8> {
    if let Some(exponent) = token.iter().position(|byte| matches!(byte, b'e' | b'E')) {
        let mut normalized = Vec::with_capacity(token.len() + 2);
        normalized.extend_from_slice(&token[..exponent]);
        normalized.push(b'e');
        let mut cursor = exponent + 1;
        if token
            .get(cursor)
            .is_some_and(|byte| matches!(byte, b'+' | b'-'))
        {
            normalized.push(token[cursor]);
            cursor += 1;
        } else {
            normalized.push(b'+');
        }
        if token.len() - cursor == 1 {
            normalized.push(b'0');
        }
        normalized.extend_from_slice(&token[cursor..]);
        return normalized;
    }

    let (sign, unsigned) = token
        .strip_prefix(b"-")
        .map_or((&b""[..], token), |unsigned| (&b"-"[..], unsigned));
    let Some(fraction) = unsigned.strip_prefix(b"0.") else {
        return token.to_vec();
    };
    let leading_zeroes = fraction.iter().take_while(|byte| **byte == b'0').count();
    if leading_zeroes < 4 || leading_zeroes == fraction.len() {
        return token.to_vec();
    }

    let significant = &fraction[leading_zeroes..];
    let mut normalized = Vec::with_capacity(token.len() + 3);
    normalized.extend_from_slice(sign);
    normalized.push(significant[0]);
    if significant.len() > 1 {
        normalized.push(b'.');
        normalized.extend_from_slice(&significant[1..]);
    }
    normalized.extend_from_slice(b"e-");
    let exponent = leading_zeroes + 1;
    if exponent < 10 {
        normalized.push(b'0');
    }
    normalized.extend_from_slice(exponent.to_string().as_bytes());
    normalized
}

/// Complete set of component identities governing one observed result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GoverningVersions {
    /// Engineering Assurance module identity.
    pub module: VersionIdentity,
    /// Engineering Assurance plugin identity.
    pub plugin: VersionIdentity,
    /// Skill identity.
    pub skill: VersionIdentity,
    /// Workflow identity.
    pub workflow: VersionIdentity,
    /// Quire identity.
    pub quire: VersionIdentity,
    /// Quoin identity.
    pub quoin: VersionIdentity,
    /// ix-flow identity.
    pub ix_flow: VersionIdentity,
    /// Producer-output schema identity.
    pub schema: VersionIdentity,
    /// Native producer identity.
    pub producer: VersionIdentity,
}

impl GoverningVersions {
    /// Return every stable identity validation failure in reference order.
    #[must_use]
    pub fn errors(&self) -> Vec<String> {
        let identities = [
            &self.module,
            &self.plugin,
            &self.skill,
            &self.workflow,
            &self.quire,
            &self.quoin,
            &self.ix_flow,
            &self.schema,
            &self.producer,
        ];
        GOVERNING_FIELDS
            .into_iter()
            .zip(identities)
            .flat_map(|(field, identity)| {
                identity
                    .errors()
                    .into_iter()
                    .map(move |error| format!("{field}:{error}"))
            })
            .collect()
    }
}

/// Direct operator observation of one producer attempt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorObservation {
    /// Exact command argument vector.
    pub command: Vec<String>,
    /// Non-negative elapsed wall time in milliseconds.
    pub elapsed_ms: i64,
    /// Process exit code, absent only when the producer was not run.
    pub exit_code: Option<i64>,
    /// Stable outcome: `succeeded`, `failed`, or `not-run`.
    pub outcome: String,
    /// Stable diagnostic category required for failures.
    pub diagnostic_category: Option<String>,
}

impl OperatorObservation {
    /// Return every stable observation validation failure in reference order.
    #[must_use]
    pub fn errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.command.is_empty() || self.command.iter().any(|item| item.trim().is_empty()) {
            errors.push("command-missing".to_owned());
        }
        if self.elapsed_ms < 0 {
            errors.push("elapsed-invalid".to_owned());
        }
        if !matches!(self.outcome.as_str(), "succeeded" | "failed" | "not-run") {
            errors.push("outcome-invalid".to_owned());
        }
        if self.outcome == "not-run" && self.exit_code.is_some() {
            errors.push("not-run-has-exit-code".to_owned());
        }
        if self.outcome != "not-run" && self.exit_code.is_none() {
            errors.push("exit-code-missing".to_owned());
        }
        if self.outcome == "failed"
            && self
                .diagnostic_category
                .as_ref()
                .is_none_or(String::is_empty)
        {
            errors.push("failure-category-missing".to_owned());
        }
        errors
    }
}

/// Inputs observed for one producer-classification decision.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProducerAttempt {
    /// Stable producer identity.
    pub producer_id: String,
    /// Whether the producer applies to the selected boundary.
    pub applicable: bool,
    /// Whether the producer was invoked.
    pub invoked: bool,
    /// Direct operator observation.
    pub observation: OperatorObservation,
    /// Immutable governing identities, required for observed evidence.
    pub governing: Option<GoverningVersions>,
    /// Parsed producer output, required to be a JSON object for observed evidence.
    pub output: Option<Value>,
    /// Whether the producer output passed its owned contract.
    pub output_valid: bool,
    /// Deferred next action for a not-computed producer.
    pub next_action: Option<String>,
    /// Owner responsible for a not-computed producer.
    pub owner: Option<String>,
    /// Boundary rationale for a non-applicable producer.
    pub boundary_rationale: Option<String>,
    /// Quoin handoff identity for observed evidence.
    pub quoin_reference: Option<String>,
}

/// Classified evidence availability without persistence or producer execution.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceEnvelope {
    /// Stable producer identity.
    pub producer_id: String,
    /// Availability, absent when validation failed.
    pub availability: Option<AvailabilityState>,
    /// Direct operator observation.
    pub observation: OperatorObservation,
    /// Immutable governing identities retained for observed evidence and invalid attempts.
    pub governing: Option<GoverningVersions>,
    /// SHA-256 of retained canonical JSON output bytes.
    pub output_digest: Option<String>,
    /// Deferred next action.
    pub next_action: Option<String>,
    /// Responsible owner.
    pub owner: Option<String>,
    /// Non-applicability rationale.
    pub boundary_rationale: Option<String>,
    /// Quoin handoff identity.
    pub quoin_reference: Option<String>,
    /// Stable validation errors in reference order.
    pub validation_errors: Vec<String>,
}

impl EvidenceEnvelope {
    /// Whether the attempt has no validation errors.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.validation_errors.is_empty()
    }
}

fn invalid(attempt: &ProducerAttempt, errors: Vec<String>) -> EvidenceEnvelope {
    EvidenceEnvelope {
        producer_id: attempt.producer_id.clone(),
        availability: None,
        observation: attempt.observation.clone(),
        governing: attempt.governing.clone(),
        output_digest: None,
        next_action: None,
        owner: None,
        boundary_rationale: None,
        quoin_reference: attempt.quoin_reference.clone(),
        validation_errors: errors,
    }
}

fn classified(attempt: &ProducerAttempt, availability: AvailabilityState) -> EvidenceEnvelope {
    EvidenceEnvelope {
        producer_id: attempt.producer_id.clone(),
        availability: Some(availability),
        observation: attempt.observation.clone(),
        governing: None,
        output_digest: None,
        next_action: None,
        owner: None,
        boundary_rationale: None,
        quoin_reference: None,
        validation_errors: Vec::new(),
    }
}

/// Classify one producer without promoting invalid output to evidence.
#[must_use]
pub fn classify_producer(attempt: &ProducerAttempt) -> EvidenceEnvelope {
    if attempt.producer_id.trim().is_empty() {
        return invalid(attempt, vec!["producer-id-missing".to_owned()]);
    }
    let observation_errors = attempt.observation.errors();
    if !observation_errors.is_empty() {
        return invalid(attempt, observation_errors);
    }

    if !attempt.applicable {
        if attempt
            .boundary_rationale
            .as_ref()
            .is_none_or(String::is_empty)
        {
            return invalid(attempt, vec!["boundary-rationale-missing".to_owned()]);
        }
        let mut result = classified(attempt, AvailabilityState::NotApplicable);
        result
            .boundary_rationale
            .clone_from(&attempt.boundary_rationale);
        return result;
    }

    if !attempt.invoked {
        if attempt.next_action.as_ref().is_none_or(String::is_empty)
            && attempt.owner.as_ref().is_none_or(String::is_empty)
        {
            return invalid(attempt, vec!["next-action-or-owner-missing".to_owned()]);
        }
        let mut result = classified(attempt, AvailabilityState::NotComputed);
        result.next_action.clone_from(&attempt.next_action);
        result.owner.clone_from(&attempt.owner);
        return result;
    }

    if attempt.observation.outcome == "failed" {
        return classified(attempt, AvailabilityState::Unavailable);
    }
    if attempt.observation.outcome != "succeeded" {
        return invalid(
            attempt,
            vec!["invoked-producer-has-invalid-outcome".to_owned()],
        );
    }
    let Some(output) = attempt.output.as_ref().filter(|output| output.is_object()) else {
        return invalid(attempt, vec!["producer-output-malformed".to_owned()]);
    };
    if !attempt.output_valid {
        return invalid(attempt, vec!["producer-output-malformed".to_owned()]);
    }
    let Some(governing) = attempt.governing.as_ref() else {
        return invalid(attempt, vec!["governing-versions-missing".to_owned()]);
    };
    let governing_errors = governing.errors();
    if !governing_errors.is_empty() {
        return invalid(attempt, governing_errors);
    }
    if !attempt
        .quoin_reference
        .as_deref()
        .is_some_and(|reference| reference.starts_with("ix://agent-ix/quoin/"))
    {
        return invalid(attempt, vec!["quoin-handoff-missing".to_owned()]);
    }

    let encoded = legacy_canonical_json(output);
    let mut result = classified(attempt, AvailabilityState::Observed);
    result.governing.clone_from(&attempt.governing);
    result.output_digest = Some(sha256_hex(&encoded));
    result.quoin_reference.clone_from(&attempt.quoin_reference);
    result
}
