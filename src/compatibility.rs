// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Pure classification against the reviewed shared-assurance compatibility matrix.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Protocol accepted by [`evaluate_request_bytes`].
pub const REQUEST_PROTOCOL: &str = "engineering-assurance.compatibility-request/v1";

/// Protocol emitted for a successfully evaluated compatibility request.
pub const RESULT_PROTOCOL: &str = "engineering-assurance.compatibility-result/v1";

const MATRIX_VERSION: &str = "engineering-assurance.compatibility-matrix/v1";
const MATRIX_BYTES: &[u8] = include_bytes!("../engineering_assurance/compatibility-matrix.json");

/// One caller-observed component version.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ComponentObservation {
    /// Component identity from the reviewed matrix.
    pub component: String,
    /// Exact observed version, or `null` when the component was not observed.
    pub version: Option<String>,
}

/// Versioned request for pure compatibility classification.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityRequest {
    /// Exact request protocol discriminator.
    pub protocol: String,
    /// Component observations; each reviewed component may occur at most once.
    pub observed: Vec<ComponentObservation>,
}

/// Closed compatibility verdict for one observed component.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityVerdict {
    /// The observed version exactly equals the reviewed pin.
    Compatible,
    /// The reviewed matrix explicitly identifies the observed version as incompatible.
    Incompatible,
    /// The component was absent or its version is not classified by the matrix.
    Unknown,
}

/// One deterministic component classification.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentClassification {
    /// Component identity from the reviewed matrix.
    pub component: String,
    /// Exact observed version, or `null` when it was not observed.
    pub observed: Option<String>,
    /// Exact version selected by the reviewed matrix.
    pub expected: String,
    /// Closed classification verdict.
    pub verdict: CompatibilityVerdict,
    /// Human-readable explanation of the classification.
    pub reason: String,
}

/// Aggregate compatibility outcome without collapsing human acceptance into a tool fact.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityOutcome {
    /// Every component exactly matches the reviewed pin and human acceptance is recorded.
    Compatible,
    /// A component does not match or the matrix lacks attributed human acceptance.
    Withheld,
}

/// Deterministic result of classifying every reviewed component.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityResult {
    /// Exact result protocol discriminator.
    pub protocol: &'static str,
    /// Exact reviewed compatibility-matrix version.
    pub matrix_version: String,
    /// Aggregate machine outcome.
    pub outcome: CompatibilityOutcome,
    /// Whether every observed component exactly matches its reviewed pin.
    pub versions_compatible: bool,
    /// Whether the embedded matrix carries a complete attributed human acceptance.
    pub human_acceptance_recorded: bool,
    /// Whether compatible versions and attributed human acceptance both hold.
    pub gate_satisfied: bool,
    /// Per-component classifications in reviewed matrix order.
    pub components: Vec<ComponentClassification>,
}

impl CompatibilityResult {
    /// Encode the result as one compact JSON value followed by one newline.
    ///
    /// # Errors
    ///
    /// Returns [`CompatibilityError::ResultSerialization`] if the typed result
    /// cannot be represented as JSON.
    pub fn to_json_line(&self) -> Result<Vec<u8>, CompatibilityError> {
        let mut bytes =
            serde_json::to_vec(self).map_err(|error| CompatibilityError::ResultSerialization {
                detail: error.to_string(),
            })?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

/// Stable failures at the compatibility request boundary.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CompatibilityError {
    /// The request is not valid strict JSON for [`CompatibilityRequest`].
    #[error("invalid compatibility request: {detail}")]
    InvalidRequest {
        /// Parser or structural-validation detail.
        detail: String,
    },
    /// The request names a protocol this implementation does not support.
    #[error("unsupported compatibility protocol {observed:?}")]
    UnsupportedProtocol {
        /// Protocol received from the caller.
        observed: String,
    },
    /// One component occurs more than once in the observation list.
    #[error("duplicate compatibility observation for {component:?}")]
    DuplicateObservation {
        /// Duplicated component identity.
        component: String,
    },
    /// An observation names no component in the reviewed matrix.
    #[error("unknown compatibility component {component:?}")]
    UnknownComponent {
        /// Unknown component identity.
        component: String,
    },
    /// An observation contains a blank identity or version.
    #[error("invalid compatibility observation for {component:?}: {detail}")]
    InvalidObservation {
        /// Component identity, possibly blank.
        component: String,
        /// Stable explanation of the invalid value.
        detail: &'static str,
    },
    /// The embedded reviewed matrix violates its required contract.
    #[error("embedded compatibility matrix is invalid: {detail}")]
    InvalidMatrix {
        /// Structural or semantic failure detail.
        detail: String,
    },
    /// A typed result could not be serialized.
    #[error("compatibility result serialization failed: {detail}")]
    ResultSerialization {
        /// Serializer failure detail.
        detail: String,
    },
}

impl CompatibilityError {
    /// Stable machine-readable diagnostic code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequest { .. } => "invalid_compatibility_request",
            Self::UnsupportedProtocol { .. } => "unsupported_compatibility_protocol",
            Self::DuplicateObservation { .. } => "duplicate_compatibility_observation",
            Self::UnknownComponent { .. } => "unknown_compatibility_component",
            Self::InvalidObservation { .. } => "invalid_compatibility_observation",
            Self::InvalidMatrix { .. } => "invalid_embedded_compatibility_matrix",
            Self::ResultSerialization { .. } => "compatibility_result_serialization_failed",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Matrix {
    #[serde(rename = "matrix_version")]
    version: String,
    #[serde(rename = "purpose")]
    _purpose: String,
    accepted: Acceptance,
    #[serde(rename = "gate")]
    _gate: Gate,
    components: Vec<MatrixComponent>,
    #[serde(rename = "fixtures")]
    _fixtures: serde_json::Value,
    #[serde(rename = "rollback")]
    _rollback: serde_json::Value,
    #[serde(rename = "upgrade")]
    _upgrade: serde_json::Value,
    #[serde(rename = "hosted_ci")]
    _hosted_ci: String,
    #[serde(rename = "registry")]
    _registry: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Acceptance {
    state: String,
    accepted_by: Option<String>,
    accepted_at: Option<String>,
    note: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Gate {
    #[serde(rename = "rule")]
    _rule: String,
    #[serde(rename = "unknown_is_not_pass")]
    _unknown_is_not_pass: String,
    #[serde(rename = "absent_is_not_pass")]
    _absent_is_not_pass: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MatrixComponent {
    name: String,
    version: String,
    released: bool,
    #[serde(rename = "release")]
    _release: String,
    #[serde(default)]
    #[serde(rename = "source_revision")]
    _source_revision: Option<String>,
    #[serde(default)]
    #[serde(rename = "release_integrity")]
    _release_integrity: Option<String>,
    #[serde(default)]
    #[serde(rename = "engine")]
    _engine: Option<serde_json::Value>,
    #[serde(rename = "provides")]
    _provides: String,
    #[serde(rename = "observe")]
    _observe: String,
    incompatible: Vec<String>,
    incompatible_reasons: BTreeMap<String, String>,
    #[serde(rename = "artifacts")]
    _artifacts: Vec<serde_json::Value>,
}

/// Return one exact component pin from the embedded reviewed matrix.
///
/// # Errors
///
/// Returns [`CompatibilityError::InvalidMatrix`] if the embedded matrix is
/// malformed, and [`CompatibilityError::UnknownComponent`] if `name` is not a
/// reviewed component identity.
pub fn expected_component_version(name: &str) -> Result<String, CompatibilityError> {
    let matrix = Matrix::parse()?;
    matrix
        .components
        .into_iter()
        .find(|component| component.name == name)
        .map(|component| component.version)
        .ok_or_else(|| CompatibilityError::UnknownComponent {
            component: name.to_owned(),
        })
}

impl Matrix {
    fn parse() -> Result<Self, CompatibilityError> {
        let matrix: Self = serde_json::from_slice(MATRIX_BYTES).map_err(|error| {
            CompatibilityError::InvalidMatrix {
                detail: error.to_string(),
            }
        })?;
        matrix.validate()?;
        Ok(matrix)
    }

    fn validate(&self) -> Result<(), CompatibilityError> {
        if self.version != MATRIX_VERSION {
            return Err(CompatibilityError::InvalidMatrix {
                detail: format!("unsupported matrix version {:?}", self.version),
            });
        }
        if self.components.is_empty() {
            return Err(CompatibilityError::InvalidMatrix {
                detail: "matrix pins no components".to_owned(),
            });
        }
        let mut names = BTreeSet::new();
        for component in &self.components {
            if component.name.trim().is_empty() || component.version.trim().is_empty() {
                return Err(CompatibilityError::InvalidMatrix {
                    detail: "matrix contains a blank component identity or version".to_owned(),
                });
            }
            if !component.released {
                return Err(CompatibilityError::InvalidMatrix {
                    detail: format!("component {:?} is not released", component.name),
                });
            }
            if !names.insert(component.name.as_str()) {
                return Err(CompatibilityError::InvalidMatrix {
                    detail: format!("duplicate component {:?}", component.name),
                });
            }
            for incompatible in &component.incompatible {
                if incompatible == &component.version {
                    return Err(CompatibilityError::InvalidMatrix {
                        detail: format!(
                            "component {:?} marks its pin incompatible",
                            component.name
                        ),
                    });
                }
            }
        }
        Ok(())
    }

    fn human_acceptance_recorded(&self) -> bool {
        self.accepted.state == "accepted"
            && self
                .accepted
                .accepted_by
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            && self
                .accepted
                .accepted_at
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            && !self.accepted.note.trim().is_empty()
    }
}

/// Parse and evaluate one compatibility request without performing I/O.
///
/// # Errors
///
/// Returns a stable [`CompatibilityError`] for syntax, protocol, observation,
/// or embedded-matrix failures.
pub fn evaluate_request_bytes(input: &[u8]) -> Result<CompatibilityResult, CompatibilityError> {
    let request: CompatibilityRequest =
        serde_json::from_slice(input).map_err(|error| CompatibilityError::InvalidRequest {
            detail: error.to_string(),
        })?;
    if request.protocol != REQUEST_PROTOCOL {
        return Err(CompatibilityError::UnsupportedProtocol {
            observed: request.protocol,
        });
    }

    let matrix = Matrix::parse()?;
    let known: BTreeSet<&str> = matrix
        .components
        .iter()
        .map(|component| component.name.as_str())
        .collect();
    let mut observed = BTreeMap::new();
    for observation in request.observed {
        if observation.component.trim().is_empty() {
            return Err(CompatibilityError::InvalidObservation {
                component: observation.component,
                detail: "component identity is blank",
            });
        }
        if !known.contains(observation.component.as_str()) {
            return Err(CompatibilityError::UnknownComponent {
                component: observation.component,
            });
        }
        if observation
            .version
            .as_deref()
            .is_some_and(|version| version.trim().is_empty())
        {
            return Err(CompatibilityError::InvalidObservation {
                component: observation.component,
                detail: "observed version is blank",
            });
        }
        let component = observation.component;
        if observed
            .insert(component.clone(), observation.version)
            .is_some()
        {
            return Err(CompatibilityError::DuplicateObservation { component });
        }
    }

    let components: Vec<_> = matrix
        .components
        .iter()
        .map(|component| classify(component, observed.get(&component.name).cloned().flatten()))
        .collect();
    let versions_compatible = components
        .iter()
        .all(|item| item.verdict == CompatibilityVerdict::Compatible);
    let human_acceptance_recorded = matrix.human_acceptance_recorded();
    let gate_satisfied = versions_compatible && human_acceptance_recorded;
    Ok(CompatibilityResult {
        protocol: RESULT_PROTOCOL,
        matrix_version: matrix.version,
        outcome: if gate_satisfied {
            CompatibilityOutcome::Compatible
        } else {
            CompatibilityOutcome::Withheld
        },
        versions_compatible,
        human_acceptance_recorded,
        gate_satisfied,
        components,
    })
}

fn classify(component: &MatrixComponent, observed: Option<String>) -> ComponentClassification {
    let (verdict, reason) = match observed.as_deref() {
        None => (
            CompatibilityVerdict::Unknown,
            format!(
                "{} was not observed; nothing was checked against the pin",
                component.name
            ),
        ),
        Some(version) if version == component.version => (
            CompatibilityVerdict::Compatible,
            format!("{} {version} is the pinned version", component.name),
        ),
        Some(version) if component.incompatible.iter().any(|item| item == version) => (
            CompatibilityVerdict::Incompatible,
            component
                .incompatible_reasons
                .get(version)
                .cloned()
                .unwrap_or_else(|| {
                    format!(
                        "{} {version} is named incompatible by this matrix",
                        component.name
                    )
                }),
        ),
        Some(version) => (
            CompatibilityVerdict::Unknown,
            format!(
                "{} {version} is not the pinned {} and this matrix has never seen it; it is untested, not approved and not rejected",
                component.name, component.version
            ),
        ),
    };
    ComponentClassification {
        component: component.name.clone(),
        observed,
        expected: component.version.clone(),
        verdict,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    fn request(observed: &[(&str, Option<&str>)]) -> Vec<u8> {
        let observed = observed
            .iter()
            .map(|(component, version)| {
                serde_json::json!({"component": component, "version": version})
            })
            .collect::<Vec<_>>();
        serde_json::to_vec(&serde_json::json!({
            "protocol": REQUEST_PROTOCOL,
            "observed": observed,
        }))
        .expect("test request must serialize")
    }

    fn exact_observations() -> Vec<(&'static str, Option<&'static str>)> {
        vec![
            ("quire-cli", Some("0.31.0")),
            ("quoin", Some("0.23.1")),
            ("ix-flow", Some("0.2.3")),
            ("engineering-assurance", Some("0.2.1")),
        ]
    }

    #[trace("TC-080", "FR-012-AC-2")]
    #[test]
    fn tc_080_keeps_compatible_incompatible_and_unknown_distinct() {
        let mut cases = exact_observations();
        cases[1].1 = Some("0.22.5");
        cases[2].1 = Some("99.0.0");
        cases[3].1 = None;
        let result = evaluate_request_bytes(&request(&cases)).expect("request must evaluate");
        assert_eq!(result.outcome, CompatibilityOutcome::Withheld);
        assert_eq!(
            result
                .components
                .iter()
                .map(|item| item.verdict)
                .collect::<Vec<_>>(),
            vec![
                CompatibilityVerdict::Compatible,
                CompatibilityVerdict::Incompatible,
                CompatibilityVerdict::Unknown,
                CompatibilityVerdict::Unknown,
            ]
        );
    }

    #[trace("TC-081", "FR-012-AC-3")]
    #[test]
    fn tc_081_requires_every_pinned_component() {
        let exact = evaluate_request_bytes(&request(&exact_observations()))
            .expect("exact request must evaluate");
        assert!(exact.versions_compatible);
        assert_eq!(exact.outcome, CompatibilityOutcome::Compatible);

        let missing = evaluate_request_bytes(&request(&exact_observations()[..3]))
            .expect("partial request must evaluate");
        assert!(!missing.versions_compatible);
        assert_eq!(missing.outcome, CompatibilityOutcome::Withheld);
    }

    #[trace("TC-082", "FR-012-AC-4")]
    #[test]
    fn tc_082_reports_attributed_acceptance_separately() {
        let result =
            evaluate_request_bytes(&request(&exact_observations())).expect("request must evaluate");
        assert!(result.versions_compatible);
        assert!(result.human_acceptance_recorded);
        assert!(result.gate_satisfied);
    }

    #[trace("TC-079", "FR-012-AC-1")]
    #[test]
    fn tc_079_exposes_the_exact_embedded_component_pin() {
        assert_eq!(
            expected_component_version("ix-flow").expect("ix-flow pin must exist"),
            "0.2.3"
        );
        assert_eq!(
            expected_component_version("unknown")
                .expect_err("unknown component must fail")
                .code(),
            "unknown_compatibility_component"
        );
    }

    #[trace("TC-085", "FR-012-AC-7")]
    #[test]
    fn tc_085_refuses_unknown_protocol_component_and_duplicates() {
        let foreign = br#"{"protocol":"foreign/v1","observed":[]}"#;
        assert_eq!(
            evaluate_request_bytes(foreign)
                .expect_err("foreign protocol must fail")
                .code(),
            "unsupported_compatibility_protocol"
        );
        assert_eq!(
            evaluate_request_bytes(&request(&[("foreign", Some("1.0.0"))]))
                .expect_err("unknown component must fail")
                .code(),
            "unknown_compatibility_component"
        );
        assert_eq!(
            evaluate_request_bytes(&request(&[
                ("quoin", Some("0.23.1")),
                ("quoin", Some("0.23.1")),
            ]))
            .expect_err("duplicate component must fail")
            .code(),
            "duplicate_compatibility_observation"
        );
    }
}
