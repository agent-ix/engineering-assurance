// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Pure request, result, and binding types for the ix-flow host adapter.
//!
//! This module performs no I/O and owns no workflow state. The binary adapter
//! delegates lifecycle operations and human-gate mechanics to ix-flow.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Protocol accepted by the workflow-host CLI capability.
pub const REQUEST_PROTOCOL: &str = "engineering-assurance.workflow-host/v1";

/// Protocol emitted by a successful workflow-host operation.
pub const RESULT_PROTOCOL: &str = "engineering-assurance.workflow-host-result/v1";

/// The two lifecycle operations exposed by Engineering Assurance.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowOperation {
    /// Create and bind a missing run, or validate and resume an existing run.
    StartOrResume,
    /// Return the current decision-ready state or record an explicit decision.
    Decide,
}

/// An explicit terminal decision supplied by the bound human owner.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionChoice {
    /// Select the workflow's positive terminal phase.
    Accept,
    /// Select the workflow's negative terminal phase.
    Reject,
}

impl DecisionChoice {
    /// Return the stable wire spelling recorded in the ix-flow event note.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Reject => "reject",
        }
    }
}

impl fmt::Display for DecisionChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One of the four canonical Engineering Assurance workflows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkflowKind {
    /// Bounded assurance-profile intake.
    AssuranceIntake,
    /// Scenario-based architecture evaluation.
    ArchitectureEvaluation,
    /// One-stage measurement maturity promotion.
    MeasurementPromotion,
    /// Bounded change assurance.
    ChangeAssurance,
}

impl WorkflowKind {
    /// Return the canonical ix-flow definition name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AssuranceIntake => "assurance-intake",
            Self::ArchitectureEvaluation => "architecture-evaluation",
            Self::MeasurementPromotion => "measurement-promotion",
            Self::ChangeAssurance => "change-assurance",
        }
    }

    /// Return the terminal phase selected by `choice`.
    #[must_use]
    pub const fn terminal_phase(self, choice: DecisionChoice) -> &'static str {
        match (self, choice) {
            (Self::AssuranceIntake | Self::ArchitectureEvaluation, DecisionChoice::Accept) => {
                "accepted"
            }
            (
                Self::AssuranceIntake | Self::ArchitectureEvaluation | Self::ChangeAssurance,
                DecisionChoice::Reject,
            ) => "rejected",
            (Self::MeasurementPromotion, DecisionChoice::Accept) => "promoted",
            (Self::MeasurementPromotion, DecisionChoice::Reject) => "not_promoted",
            (Self::ChangeAssurance, DecisionChoice::Accept) => "approved",
        }
    }

    /// Return the terminal phase opposite `choice`.
    #[must_use]
    pub const fn opposite_terminal_phase(self, choice: DecisionChoice) -> &'static str {
        match choice {
            DecisionChoice::Accept => self.terminal_phase(DecisionChoice::Reject),
            DecisionChoice::Reject => self.terminal_phase(DecisionChoice::Accept),
        }
    }

    /// Return the definition's initial phase.
    #[must_use]
    pub const fn initial_phase(self) -> &'static str {
        "capture"
    }
}

impl fmt::Display for WorkflowKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for WorkflowKind {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "assurance-intake" => Ok(Self::AssuranceIntake),
            "architecture-evaluation" => Ok(Self::ArchitectureEvaluation),
            "measurement-promotion" => Ok(Self::MeasurementPromotion),
            "change-assurance" => Ok(Self::ChangeAssurance),
            _ => Err(()),
        }
    }
}

impl Serialize for WorkflowKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

/// Stable identity that binds one ix-flow run to one decision boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBinding {
    /// Stable ix-flow run identifier.
    pub run_id: String,
    /// Repository and revision identity governed by the run.
    pub repository_id: String,
    /// Canonical Engineering Assurance workflow name.
    pub workflow: String,
    /// Exact canonical workflow-definition version.
    pub workflow_version: String,
    /// Human-readable boundary governed by the terminal decision.
    pub decision_boundary: String,
    /// Named human owner allowed to make the terminal decision.
    pub decision_owner: String,
}

impl WorkflowBinding {
    /// Validate the complete binding and return its closed workflow kind.
    ///
    /// # Errors
    ///
    /// Returns [`WorkflowRequestError::InvalidBinding`] for blank fields,
    /// unsupported workflows, or a run id that cannot safely be passed to
    /// ix-flow as one argument.
    pub fn validate(&self) -> Result<WorkflowKind, WorkflowRequestError> {
        let missing = [
            ("run_id", self.run_id.as_str()),
            ("repository_id", self.repository_id.as_str()),
            ("workflow", self.workflow.as_str()),
            ("workflow_version", self.workflow_version.as_str()),
            ("decision_boundary", self.decision_boundary.as_str()),
            ("decision_owner", self.decision_owner.as_str()),
        ]
        .into_iter()
        .filter_map(|(name, value)| value.trim().is_empty().then_some(name))
        .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(WorkflowRequestError::InvalidBinding {
                detail: format!("missing binding fields: {}", missing.join(", ")),
            });
        }
        if !valid_run_id(&self.run_id) {
            return Err(WorkflowRequestError::InvalidBinding {
                detail: "run_id must match [A-Za-z0-9][A-Za-z0-9._-]{0,127}".to_owned(),
            });
        }
        for (name, value) in [
            ("repository_id", self.repository_id.as_str()),
            ("workflow", self.workflow.as_str()),
            ("workflow_version", self.workflow_version.as_str()),
            ("decision_boundary", self.decision_boundary.as_str()),
            ("decision_owner", self.decision_owner.as_str()),
        ] {
            if value.chars().any(char::is_control) {
                return Err(WorkflowRequestError::InvalidBinding {
                    detail: format!("{name} must not contain control characters"),
                });
            }
        }
        if self.decision_owner.starts_with('-') {
            return Err(WorkflowRequestError::InvalidBinding {
                detail: "decision_owner must not begin with '-'".to_owned(),
            });
        }
        WorkflowKind::from_str(&self.workflow).map_err(|()| WorkflowRequestError::InvalidBinding {
            detail: format!("unsupported workflow {:?}", self.workflow),
        })
    }
}

/// Versioned input to the binary-only workflow host.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowHostRequest {
    /// Exact request protocol discriminator.
    pub protocol: String,
    /// Lifecycle operation to perform.
    pub operation: WorkflowOperation,
    /// Absolute ix-flow state directory.
    pub state_dir: String,
    /// Absolute canonical Engineering Assurance skill directory.
    pub skill_root: String,
    /// ix-flow executable name or absolute path.
    pub ix_flow_executable: String,
    /// Stable run binding.
    pub binding: WorkflowBinding,
    /// Optional explicit terminal choice, valid only for `decide`.
    #[serde(default)]
    pub choice: Option<DecisionChoice>,
}

/// One typed ix-flow continuation action retained for display.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowNextAction {
    /// Display-only ix-flow command text.
    pub command: String,
    /// Human-readable description of the action.
    pub description: String,
    /// Optional condition the action serves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_for: Option<String>,
}

/// One open ix-flow human gate.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowGate {
    /// Opaque acknowledgement token issued by ix-flow.
    pub token: String,
    /// Canonical `from->to` transition identity.
    pub transition_key: String,
    /// Source phase.
    pub from: String,
    /// Target phase.
    pub to: String,
    /// Gate kind supplied by ix-flow.
    pub kind: String,
    /// RFC 3339 issue timestamp supplied by ix-flow.
    pub issued_at: String,
}

/// Typed resumable view returned without taking ownership of ix-flow state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSnapshot {
    /// Stable run identifier.
    pub run_id: String,
    /// Canonical workflow name.
    pub workflow: String,
    /// Exact workflow-definition version.
    pub workflow_version: String,
    /// Current ix-flow phase.
    pub phase: String,
    /// ix-flow optimistic-concurrency version.
    pub state_version: u64,
    /// Display-only continuation actions supplied by ix-flow.
    pub next_actions: Vec<WorkflowNextAction>,
    /// Current open human gates.
    pub open_gates: Vec<WorkflowGate>,
}

/// One attributed terminal decision recovered from ix-flow history.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionEvent {
    /// Stable run identifier.
    pub run_id: String,
    /// Canonical workflow name.
    pub workflow: String,
    /// Exact workflow-definition version.
    pub workflow_version: String,
    /// Bound human decision owner.
    pub owner: String,
    /// Explicit human choice.
    pub choice: DecisionChoice,
    /// Terminal phase selected by the choice.
    pub outcome: String,
    /// RFC 3339 ix-flow acknowledgement timestamp.
    pub timestamp: String,
}

/// Closed successful workflow-host payload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkflowHostOutcome {
    /// A resumable non-terminal or current-state snapshot.
    Snapshot {
        /// Typed ix-flow projection.
        snapshot: WorkflowSnapshot,
    },
    /// One idempotently recovered terminal human decision.
    Decision {
        /// Typed attributed terminal event.
        decision: DecisionEvent,
    },
}

/// Versioned successful workflow-host result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowHostResult {
    /// Exact result protocol discriminator.
    pub protocol: &'static str,
    /// Typed snapshot or decision result.
    #[serde(flatten)]
    pub outcome: WorkflowHostOutcome,
}

impl WorkflowHostResult {
    /// Construct one snapshot result.
    #[must_use]
    pub const fn snapshot(snapshot: WorkflowSnapshot) -> Self {
        Self {
            protocol: RESULT_PROTOCOL,
            outcome: WorkflowHostOutcome::Snapshot { snapshot },
        }
    }

    /// Construct one terminal-decision result.
    #[must_use]
    pub const fn decision(decision: DecisionEvent) -> Self {
        Self {
            protocol: RESULT_PROTOCOL,
            outcome: WorkflowHostOutcome::Decision { decision },
        }
    }

    /// Encode this result as one compact JSON value followed by one newline.
    ///
    /// # Errors
    ///
    /// Returns [`WorkflowRequestError::ResultSerialization`] if serialization
    /// unexpectedly fails.
    pub fn to_json_line(&self) -> Result<Vec<u8>, WorkflowRequestError> {
        let mut bytes = serde_json::to_vec(self).map_err(|error| {
            WorkflowRequestError::ResultSerialization {
                detail: error.to_string(),
            }
        })?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

/// Stable failures at the pure workflow-host request boundary.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum WorkflowRequestError {
    /// Input is not strict JSON matching [`WorkflowHostRequest`].
    #[error("invalid workflow-host request: {detail}")]
    InvalidRequest {
        /// Parser or shape error.
        detail: String,
    },
    /// Request protocol is unsupported.
    #[error("unsupported workflow-host protocol {observed:?}")]
    UnsupportedProtocol {
        /// Protocol received from the caller.
        observed: String,
    },
    /// Binding identity is incomplete or unsafe.
    #[error("invalid workflow binding: {detail}")]
    InvalidBinding {
        /// Stable validation detail.
        detail: String,
    },
    /// Operation and choice fields are inconsistent.
    #[error("invalid workflow-host operation: {detail}")]
    InvalidOperation {
        /// Stable validation detail.
        detail: &'static str,
    },
    /// A typed result could not be serialized.
    #[error("workflow-host result serialization failed: {detail}")]
    ResultSerialization {
        /// Serializer failure detail.
        detail: String,
    },
}

impl WorkflowRequestError {
    /// Stable machine-readable diagnostic code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequest { .. }
            | Self::UnsupportedProtocol { .. }
            | Self::InvalidOperation { .. } => "workflow_host_request_invalid",
            Self::InvalidBinding { .. } => "workflow_binding_invalid",
            Self::ResultSerialization { .. } => "workflow_host_result_serialization_failed",
        }
    }
}

/// Parse and validate one workflow-host request without performing I/O.
///
/// # Errors
///
/// Returns a stable [`WorkflowRequestError`] for malformed JSON, an unsupported
/// protocol, an invalid binding, or a choice supplied to `start_or_resume`.
pub fn parse_request_bytes(input: &[u8]) -> Result<WorkflowHostRequest, WorkflowRequestError> {
    let request: WorkflowHostRequest =
        serde_json::from_slice(input).map_err(|error| WorkflowRequestError::InvalidRequest {
            detail: error.to_string(),
        })?;
    if request.protocol != REQUEST_PROTOCOL {
        return Err(WorkflowRequestError::UnsupportedProtocol {
            observed: request.protocol,
        });
    }
    request.binding.validate()?;
    if request.operation == WorkflowOperation::StartOrResume && request.choice.is_some() {
        return Err(WorkflowRequestError::InvalidOperation {
            detail: "start_or_resume does not accept a terminal choice",
        });
    }
    for (name, value) in [
        ("state_dir", request.state_dir.as_str()),
        ("skill_root", request.skill_root.as_str()),
        ("ix_flow_executable", request.ix_flow_executable.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(WorkflowRequestError::InvalidRequest {
                detail: format!("{name} is blank"),
            });
        }
        if value.chars().any(char::is_control) {
            return Err(WorkflowRequestError::InvalidRequest {
                detail: format!("{name} must not contain control characters"),
            });
        }
    }
    Ok(request)
}

fn valid_run_id(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    value.len() <= 128
        && first.is_ascii_alphanumeric()
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    #[trace("TC-107", "FR-016-AC-3", "FR-016-CON-1")]
    #[test]
    fn tc_107_request_contract_rejects_unsafe_or_inconsistent_bindings() {
        let base = serde_json::json!({
            "protocol": REQUEST_PROTOCOL,
            "operation": "start_or_resume",
            "state_dir": "/tmp/state",
            "skill_root": "/tmp/skill",
            "ix_flow_executable": "ix-flow",
            "binding": {
                "run_id": "run-1",
                "repository_id": "example@revision",
                "workflow": "architecture-evaluation",
                "workflow_version": "0.1.0",
                "decision_boundary": "one boundary",
                "decision_owner": "owner"
            }
        });
        let encoded = serde_json::to_vec(&base).expect("fixture must serialize");
        assert!(parse_request_bytes(&encoded).is_ok());

        for changed in [
            serde_json::json!({"binding": {"run_id": "../escape"}}),
            serde_json::json!({"binding": {"workflow": "unknown"}}),
            serde_json::json!({"binding": {"decision_owner": "--help"}}),
            serde_json::json!({"binding": {"decision_boundary": "line\nbreak"}}),
            serde_json::json!({"state_dir": "/tmp/state\u{0000}suffix"}),
            serde_json::json!({"choice": "accept"}),
            serde_json::json!({"unexpected": true}),
        ] {
            let mut candidate = base.clone();
            merge(&mut candidate, &changed);
            assert!(
                parse_request_bytes(
                    &serde_json::to_vec(&candidate).expect("fixture must serialize")
                )
                .is_err(),
                "candidate unexpectedly passed: {candidate}"
            );
        }
    }

    fn merge(target: &mut serde_json::Value, changes: &serde_json::Value) {
        let target = target.as_object_mut().expect("target must be an object");
        for (key, value) in changes.as_object().expect("changes must be an object") {
            if key == "binding" {
                target["binding"]
                    .as_object_mut()
                    .expect("binding must be an object")
                    .extend(
                        value
                            .as_object()
                            .expect("binding changes must be an object")
                            .clone(),
                    );
            } else {
                target.insert(key.clone(), value.clone());
            }
        }
    }
}
