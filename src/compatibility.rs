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

/// A repository-relative artifact digest recorded by the reviewed matrix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordedArtifactDigest {
    /// Repository-relative artifact path.
    pub path: String,
    /// Expected lowercase SHA-256 hex digest.
    pub sha256: String,
}

/// One caller-observed component version.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentObservation {
    /// Component identity from the reviewed matrix.
    pub component: String,
    /// Exact observed version, or `null` when the component was not observed.
    pub version: Option<String>,
}

/// Versioned request for pure compatibility classification.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
    /// An observation names no component at all.
    ///
    /// Separate from [`Self::BlankObservedVersion`] because the two are
    /// different mistakes in the caller's request and were distinguishable only
    /// by prose: a blank identity means the request does not say what was
    /// observed, while a blank version means it says what was observed and not
    /// what version it is. A caller repairing its request needs to know which.
    #[error("invalid compatibility observation: component identity is blank")]
    BlankObservedComponent,
    /// An observation names a component but leaves its version blank.
    #[error("invalid compatibility observation for {component:?}: observed version is blank")]
    BlankObservedVersion {
        /// Component identity.
        component: String,
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
            Self::BlankObservedComponent => "blank_compatibility_observation_component",
            Self::BlankObservedVersion { .. } => "blank_compatibility_observation_version",
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
    release: String,
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
    artifacts: Vec<MatrixArtifact>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MatrixArtifact {
    path: String,
    sha256: String,
    #[serde(default)]
    #[serde(rename = "note")]
    _note: Option<String>,
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

/// Return every artifact digest recorded by the embedded reviewed matrix.
///
/// This exposes matrix data only. Callers that read files and compute hashes
/// remain separate host adapters, preserving the classifier's I/O-free boundary.
///
/// # Errors
///
/// Returns [`CompatibilityError::InvalidMatrix`] if the embedded matrix is malformed.
pub fn recorded_artifact_digests() -> Result<Vec<RecordedArtifactDigest>, CompatibilityError> {
    let matrix = Matrix::parse()?;
    Ok(matrix
        .components
        .into_iter()
        .flat_map(|component| component.artifacts)
        .map(|artifact| RecordedArtifactDigest {
            path: artifact.path,
            sha256: artifact.sha256,
        })
        .collect())
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
            // A component that names no release is not pinned to anything an
            // operator can install, so `released: true` beside it is a claim
            // with no artifact behind it.
            if component.release.trim().is_empty() {
                return Err(CompatibilityError::InvalidMatrix {
                    detail: format!("component {:?} names no release", component.name),
                });
            }
            // A branch name, `latest`, or a bare `HEAD` moves underneath the
            // matrix: the same pin resolves to a different build tomorrow, so a
            // toolchain this matrix classified compatible would silently become
            // something nobody reviewed. That is the exact failure FR-012-AC-1
            // exists to prevent.
            if matches!(component.version.as_str(), "main" | "latest" | "HEAD") {
                return Err(CompatibilityError::InvalidMatrix {
                    detail: format!(
                        "component {:?} pins the moving version {:?}",
                        component.name, component.version
                    ),
                });
            }
            if component.release.to_lowercase().contains("branch") {
                return Err(CompatibilityError::InvalidMatrix {
                    detail: format!(
                        "component {:?} names a branch rather than a release: {:?}",
                        component.name, component.release
                    ),
                });
            }
            // Classification consults `incompatible`, never `incompatible_reasons`.
            // A reason recorded for a version the list omits is therefore inert:
            // that version comes back `unknown` while the matrix reads as though
            // somebody had ruled it out, which is a downgrade nobody would see.
            for version in component.incompatible_reasons.keys() {
                if !component.incompatible.iter().any(|item| item == version) {
                    return Err(CompatibilityError::InvalidMatrix {
                        detail: format!(
                            "component {:?} records a rejection reason for {version:?}, which its incompatible list does not name",
                            component.name
                        ),
                    });
                }
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

/// Whether a caller-supplied matrix carries a complete attributed acceptance.
///
/// Deliberately separate from version classification. Pinned versions are a
/// fact about a machine; acceptance is a decision somebody made. A gate needs
/// both, and answering only the first is how a correctly pinned toolchain comes
/// to report an approval nobody gave.
///
/// A `state` of `accepted` with no name, no date, or no note against it is not
/// a record; it is a claim with nobody behind it, and it withholds too.
///
/// Test-only. Production reads the embedded matrix and reaches acceptance
/// through [`Matrix::human_acceptance_recorded`]; this entry point exists so a
/// test can put a mutated matrix in front of the same rule. It was `pub` with
/// no caller outside this module's own tests, which is a test seam shipped in
/// the public surface: a published function nobody calls still has to be kept
/// working, documented and compatible, and a reader has no way to tell it apart
/// from an API somebody depends on.
///
/// # Errors
///
/// Returns [`CompatibilityError::InvalidMatrix`] when the supplied bytes do not
/// satisfy the reviewed matrix contract.
#[cfg(test)]
fn acceptance_recorded_in(matrix_bytes: &[u8]) -> Result<bool, CompatibilityError> {
    let matrix: Matrix = serde_json::from_slice(matrix_bytes).map_err(|error| {
        CompatibilityError::InvalidMatrix {
            detail: error.to_string(),
        }
    })?;
    matrix.validate()?;
    Ok(matrix.human_acceptance_recorded())
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
            return Err(CompatibilityError::BlankObservedComponent);
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
            return Err(CompatibilityError::BlankObservedVersion {
                component: observation.component,
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
    use unicode_casefold::UnicodeCaseFold;

    use super::*;

    fn matrix_value() -> serde_json::Value {
        serde_json::from_slice(MATRIX_BYTES).expect("the embedded matrix must be JSON")
    }

    /// Assert a mutated matrix is refused by the contract check and return why.
    ///
    /// Structural rules live in [`Matrix::validate`] rather than only in a test
    /// so a bad matrix is refused at runtime instead of classifying against it;
    /// this drives that path through the one public entry point that accepts
    /// caller-supplied bytes.
    fn refusal_detail(matrix: &serde_json::Value) -> String {
        let bytes = serde_json::to_vec(matrix).expect("the mutated matrix must serialize");
        let error = acceptance_recorded_in(&bytes).expect_err("the mutated matrix must be refused");
        assert_eq!(error.code(), "invalid_embedded_compatibility_matrix");
        error.to_string()
    }

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
            ("engineering-assurance", Some("0.3.1")),
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

        // quoin 0.23.0 was tagged and never reached the registry. The matrix
        // names it so nobody has to rediscover why a version that exists in git
        // cannot be installed, and this asserts the recorded reason actually
        // reaches the classification rather than sitting unread in the file.
        let mut tagged_never_published = exact_observations();
        tagged_never_published[1].1 = Some("0.23.0");
        let result = evaluate_request_bytes(&request(&tagged_never_published))
            .expect("request must evaluate");
        let quoin = result
            .components
            .iter()
            .find(|item| item.component == "quoin")
            .expect("quoin must be classified");
        assert_eq!(quoin.verdict, CompatibilityVerdict::Incompatible);
        assert!(
            quoin.reason.contains("never published"),
            "the recorded reason for 0.23.0 did not reach the classification: {:?}",
            quoin.reason
        );

        // Every other component is exactly pinned here, so this is the case
        // where a single named-incompatible version has to be enough to withhold
        // the gate on its own.
        assert!(!result.versions_compatible);
        assert!(!result.gate_satisfied);
        assert_eq!(result.outcome, CompatibilityOutcome::Withheld);
    }

    #[trace("TC-081", "FR-012-AC-3")]
    #[test]
    fn tc_081_requires_every_pinned_component() {
        let exact = evaluate_request_bytes(&request(&exact_observations()))
            .expect("exact request must evaluate");
        assert!(exact.versions_compatible);
        assert_eq!(exact.outcome, CompatibilityOutcome::Compatible);

        // One unobserved component is enough to withhold the gate, and it has to
        // be true of every component rather than only the one a slice happens to
        // drop. "Mostly pinned" is not a state the migration decision has.
        for index in 0..exact_observations().len() {
            let mut partial = exact_observations();
            partial[index].1 = None;
            let missing =
                evaluate_request_bytes(&request(&partial)).expect("partial request must evaluate");
            assert!(
                !missing.versions_compatible,
                "{} was allowed to go unobserved",
                partial[index].0
            );
            assert!(
                !missing.gate_satisfied,
                "{} opened the gate unobserved",
                partial[index].0
            );
            assert_eq!(missing.outcome, CompatibilityOutcome::Withheld);
        }

        // The gate section states in prose why an unrecognised version is not a
        // pass. Losing that sentence would leave the rule documented only in the
        // classifier, where an operator reading the matrix would never find it.
        let matrix = matrix_value();
        let gate = &matrix["gate"];
        assert!(
            gate["rule"]
                .as_str()
                .is_some_and(|rule| rule.to_lowercase().contains("compatible")),
            "the gate rule no longer states what every component must classify as"
        );
        assert!(
            gate["unknown_is_not_pass"]
                .as_str()
                .is_some_and(|note| note.to_lowercase().contains("unknown")),
            "the gate no longer says that an unknown version is not a pass"
        );
    }

    /// Assert one acceptance record has one of the two honest shapes.
    ///
    /// Acceptance is either pending with nothing filled in, or accepted with a
    /// named human and a real date behind it. The shape this refuses is the
    /// dangerous one, a state that reads as accepted while nobody is on record
    /// as having accepted it.
    fn assert_acceptance_is_honest(acceptance: &serde_json::Value) {
        let state = acceptance["state"]
            .as_str()
            .expect("acceptance must name a state");
        assert!(
            matches!(state, "pending_human_acceptance" | "accepted"),
            "acceptance is in the unrecognised state {state:?}"
        );
        let note = acceptance["note"]
            .as_str()
            .expect("acceptance must carry a note");
        assert!(
            note.to_lowercase().contains("human"),
            "the acceptance note does not say a human decided: {note:?}"
        );
        assert!(
            note.contains("agent may prepare"),
            "the acceptance note does not record that an agent may only prepare: {note:?}"
        );

        if state == "pending_human_acceptance" {
            // Pending means nobody has decided yet, so an attribution or a date
            // recorded beside it is a half-recorded acceptance dressed as an
            // honest pending state.
            assert!(
                acceptance["accepted_by"].is_null(),
                "a pending matrix already attributes an acceptance"
            );
            assert!(
                acceptance["accepted_at"].is_null(),
                "a pending matrix already dates an acceptance"
            );
            return;
        }

        let who = acceptance["accepted_by"]
            .as_str()
            .expect("an accepted matrix must name who accepted it");
        assert!(!who.trim().is_empty(), "the attribution is blank");

        // CON-2: the attribution names the human who decided, not the agent that
        // typed it. An agent may transcribe an acceptance and may not be the one
        // on record for it. Without this, "Agent IX" satisfies every other
        // assertion here and the constraint is decorative.
        let folded = who.case_fold().collect::<String>();
        for impostor in ["agent", "claude", "bot"] {
            assert!(
                !folded.contains(impostor),
                "the attribution {who:?} names {impostor:?} rather than a human"
            );
        }

        // An `accepted_at` that is not a real calendar date records nothing
        // about when the decision was made, and a length check would accept both
        // a wrong shape and an impossible day. Parsing against an exact
        // year-month-day description rejects both. The standard that defines
        // this date form is deliberately not named here: this repository's
        // publication boundary refuses external publication identifiers, and
        // the gate that enforces it reads source lines.
        let when = acceptance["accepted_at"]
            .as_str()
            .expect("an accepted matrix must date the decision");
        let iso_date = time::format_description::parse_borrowed::<2>("[year]-[month]-[day]")
            .expect("the literal date description must compile");
        time::Date::parse(when, &iso_date).unwrap_or_else(|error| {
            panic!("accepted_at {when:?} is not a year-month-day calendar date: {error}")
        });
    }

    #[trace("TC-082", "FR-012-AC-4")]
    #[test]
    fn tc_082_reports_attributed_acceptance_separately() {
        let result =
            evaluate_request_bytes(&request(&exact_observations())).expect("request must evaluate");
        assert!(result.versions_compatible);
        assert!(result.human_acceptance_recorded);
        assert!(result.gate_satisfied);

        assert_acceptance_is_honest(&matrix_value()["accepted"]);

        // The embedded matrix is currently `accepted`, so its pending branch is
        // never reached above. Exercising a pending record keeps the rule that
        // pending carries no attribution from rotting unobserved until the day
        // somebody sets the state back.
        assert_acceptance_is_honest(&serde_json::json!({
            "state": "pending_human_acceptance",
            "accepted_by": serde_json::Value::Null,
            "accepted_at": serde_json::Value::Null,
            "note": "An agent may prepare the matrix; only a human may accept it.",
        }));
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

        // These four are the components the campaign actually depends on. A
        // matrix that quietly stopped pinning one of them would still classify
        // everything it does name as compatible, so the gate would open on a
        // toolchain nobody checked.
        for name in ["quire-cli", "quoin", "ix-flow", "engineering-assurance"] {
            assert!(
                expected_component_version(name).is_ok(),
                "the matrix no longer pins {name}"
            );
        }

        // The structural half of "names its release" is enforced by
        // `Matrix::validate`, so a matrix carrying a moving pin is refused
        // rather than classified against.
        for moving in ["main", "latest", "HEAD"] {
            let mut matrix = matrix_value();
            matrix["components"][0]["version"] = serde_json::json!(moving);
            assert!(
                refusal_detail(&matrix).contains("moving version"),
                "a matrix pinning {moving:?} was not refused as a moving version"
            );
        }

        let mut branch_release = matrix_value();
        branch_release["components"][0]["release"] =
            serde_json::json!("npm dist-tag tracking the main Branch");
        assert!(
            refusal_detail(&branch_release).contains("names a branch"),
            "a release naming a branch was not refused"
        );

        let mut blank_release = matrix_value();
        blank_release["components"][0]["release"] = serde_json::json!("   ");
        assert!(
            refusal_detail(&blank_release).contains("names no release"),
            "a component with no release was not refused"
        );
    }

    #[trace("TC-084", "FR-012-AC-6")]
    #[test]
    fn tc_084_states_upgrade_and_rollback_per_component() {
        let matrix = matrix_value();

        let rollback = &matrix["rollback"];
        for name in [
            "quoin",
            "quire-cli",
            "ix-flow",
            "engineering-assurance",
            "corpus",
        ] {
            let note = rollback[name]
                .as_str()
                .unwrap_or_else(|| panic!("{name} has no rollback note"));
            assert!(!note.trim().is_empty(), "{name}'s rollback note is empty");
        }
        assert!(
            rollback["irreversible"]
                .as_str()
                .is_some_and(|note| note.contains("None of the above"))
        );

        // Rolling ix-flow back means reinstalling one exact published tarball,
        // so the matrix has to identify that artifact and not merely its version
        // number: a rebuilt or re-tagged 0.2.3 is a different set of bytes, and
        // the source revision plus the registry integrity hash are what tell the
        // two apart.
        let ix_flow = matrix["components"]
            .as_array()
            .expect("components must be an array")
            .iter()
            .find(|component| component["name"] == "ix-flow")
            .expect("the matrix must pin ix-flow");
        assert_eq!(ix_flow["version"].as_str(), Some("0.2.3"));
        assert_eq!(
            ix_flow["source_revision"].as_str(),
            Some("8b6cf8287db828b4db2df814bc7c1ef10362db24")
        );
        assert!(
            ix_flow["release_integrity"]
                .as_str()
                .is_some_and(|integrity| integrity.starts_with("sha512-")),
            "ix-flow records no sha512 release integrity"
        );

        let upgrade = &matrix["upgrade"];
        assert!(
            upgrade["order"]
                .as_str()
                .is_some_and(|order| order.contains("quire-cli"))
        );
        // The verification step has to name the command that actually
        // classifies the toolchain. A bare "compatibility" would be satisfied by
        // any sentence that merely mentions the word, including one that asks
        // the reader to check by hand.
        assert!(
            upgrade["verification"]
                .as_str()
                .is_some_and(|step| step.contains("compatibility-observe")),
            "the upgrade verification does not name `compatibility-observe`"
        );
        // The upgrade explicitly does not touch a campaign repository.
        assert!(
            upgrade["what_does_not_change"]
                .as_str()
                .is_some_and(|note| note.contains("Migrations are"))
        );

        // Publication changes no repository's CI posture.
        assert!(
            matrix["hosted_ci"]
                .as_str()
                .is_some_and(|note| note.contains("manual-dispatch only"))
        );
    }

    #[trace("TC-084", "FR-012-AC-6")]
    #[test]
    fn tc_084_pins_every_component_to_the_public_release_channel() {
        let matrix = matrix_value();

        // The release channel is the public registry, and the internal mirror
        // is ruled out by name. A pin naming npm.ix cannot install or publish
        // from CI, and the mirror lagging a real publish already produced one
        // wrong reading while this matrix was prepared.
        let registry = &matrix["registry"];
        assert_eq!(
            registry["release_channel"].as_str(),
            Some("public npm registry (registry.npmjs.org)")
        );
        for field in ["rule", "mirror_is_not_an_oracle"] {
            assert!(
                registry[field]
                    .as_str()
                    .is_some_and(|value| value.contains("npm.ix")),
                "registry.{field} does not rule out the internal mirror by name"
            );
        }
        assert!(
            registry["rule"]
                .as_str()
                .is_some_and(|rule| rule.contains("MUST NOT appear"))
        );
        for component in matrix["components"]
            .as_array()
            .expect("components must be an array")
        {
            for field in ["release", "version"] {
                assert!(
                    !component[field]
                        .as_str()
                        .unwrap_or_default()
                        .contains("npm.ix"),
                    "{} pins the internal mirror in {field}",
                    component["name"]
                );
            }
        }
    }

    #[trace("TC-095", "FR-012-AC-9")]
    #[test]
    fn tc_095_a_pinned_toolchain_does_not_open_an_unaccepted_gate() {
        let with_acceptance = |acceptance: serde_json::Value| -> Vec<u8> {
            let mut matrix: serde_json::Value =
                serde_json::from_slice(MATRIX_BYTES).expect("the embedded matrix must be JSON");
            matrix["accepted"] = acceptance;
            serde_json::to_vec(&matrix).expect("the mutated matrix must serialize")
        };

        assert!(
            acceptance_recorded_in(&with_acceptance(serde_json::json!({
                "state": "accepted",
                "accepted_by": "Fictional Owner",
                "accepted_at": "2026-09-10",
                "note": "Accepted by a named human for this fixture.",
            })))
            .expect("a complete acceptance must parse"),
            "a fully attributed acceptance was not recorded"
        );

        // Any state but `accepted` withholds, including one this module has
        // never seen, on the same reasoning that makes an unrecognised version
        // `unknown` rather than a pass.
        for state in ["pending_human_acceptance", "withdrawn", "", "ACCEPTED"] {
            assert!(
                !acceptance_recorded_in(&with_acceptance(serde_json::json!({
                    "state": state,
                    "accepted_by": "Fictional Owner",
                    "accepted_at": "2026-09-10",
                    "note": "Prepared for this fixture.",
                })))
                .expect("the mutated matrix must parse"),
                "state {state:?} opened the gate"
            );
        }

        // A state of `accepted` with nobody, no date, or no note behind it is a
        // claim, not a record, and withholds exactly as a pending state does.
        for hole in [
            serde_json::json!({"accepted_by": serde_json::Value::Null}),
            serde_json::json!({"accepted_by": "   "}),
            serde_json::json!({"accepted_at": serde_json::Value::Null}),
            serde_json::json!({"accepted_at": "   "}),
            serde_json::json!({"note": "   "}),
        ] {
            let mut acceptance = serde_json::json!({
                "state": "accepted",
                "accepted_by": "Fictional Owner",
                "accepted_at": "2026-09-10",
                "note": "Accepted by a named human for this fixture.",
            });
            for (key, value) in hole.as_object().expect("a hole must be an object") {
                acceptance[key] = value.clone();
            }
            assert!(
                !acceptance_recorded_in(&with_acceptance(acceptance.clone()))
                    .expect("the mutated matrix must parse"),
                "half-recorded acceptance {acceptance} opened the gate"
            );
        }

        // And the two conditions are genuinely independent: a fully pinned
        // toolchain still fails to open a pending matrix.
        let exact = evaluate_request_bytes(&request(&exact_observations()))
            .expect("exact request must evaluate");
        assert!(exact.versions_compatible);
        assert!(
            !acceptance_recorded_in(&with_acceptance(serde_json::json!({
                "state": "pending_human_acceptance",
                "accepted_by": serde_json::Value::Null,
                "accepted_at": serde_json::Value::Null,
                "note": "An agent may prepare the matrix and may not accept it.",
            })))
            .expect("a pending matrix must parse"),
            "a pinned toolchain opened a pending gate"
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

        // A blank identity and a blank version are separate refusals. They
        // shared one variant and one code, distinguished only by a `detail`
        // string, so nothing here could tell them apart and no assertion drove
        // either one: a caller repairing its request could not learn whether it
        // had failed to say what was observed or failed to say which version.
        assert_eq!(
            evaluate_request_bytes(&request(&[("   ", Some("0.23.1"))]))
                .expect_err("a blank component identity must fail")
                .code(),
            "blank_compatibility_observation_component"
        );
        assert_eq!(
            evaluate_request_bytes(&request(&[("quoin", Some("   "))]))
                .expect_err("a blank observed version must fail")
                .code(),
            "blank_compatibility_observation_version"
        );

        // A rejection reason recorded against a version the `incompatible` list
        // does not name never reaches a classification: the version comes back
        // `unknown`, silently downgraded from the rejection the matrix appears
        // to record, and nothing reports the discrepancy.
        let mut orphaned_reason = matrix_value();
        orphaned_reason["components"][1]["incompatible_reasons"]["0.21.0"] =
            serde_json::json!("recorded as rejected while the incompatible list omits it");
        assert!(
            refusal_detail(&orphaned_reason).contains("incompatible list does not name"),
            "an unreferenced rejection reason was not refused"
        );
    }
}
