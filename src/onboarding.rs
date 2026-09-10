// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Pure types, parsing, rendering, and recommendation logic for onboarding.
//!
//! Repository traversal, Quire invocation, staging, synchronization, and
//! publication belong to the binary adapter. This module accepts explicit
//! bytes and typed inventory values and performs no I/O.

use std::fmt;

use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map as JsonMap, Value as JsonValue};
use thiserror::Error;
use yaml_serde::{Mapping as YamlMap, Value as YamlValue};

/// Protocol accepted by the native onboarding command.
pub const REQUEST_PROTOCOL: &str = "engineering-assurance.onboarding/v1";

/// Protocol emitted after a complete onboarding operation.
pub const RESULT_PROTOCOL: &str = "engineering-assurance.onboarding-result/v1";

/// The complete set of artifacts that onboarding may recommend or author.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub enum ArtifactType {
    /// Decision-scoped assurance policy.
    AssuranceProfile,
    /// Measurement definition and decision-use policy.
    MeasurementPlan,
    /// Architecture boundary, views, decisions, and risks.
    ArchitectureDescription,
    /// Component-local assurance obligations and controls.
    ComponentAssuranceContract,
    /// Authored assurance claim and sufficiency rationale.
    AssuranceArgument,
}

impl ArtifactType {
    /// Every supported artifact type in canonical order.
    pub const ALL: [Self; 5] = [
        Self::AssuranceProfile,
        Self::MeasurementPlan,
        Self::ArchitectureDescription,
        Self::ComponentAssuranceContract,
        Self::AssuranceArgument,
    ];

    /// Return the installed artifact-type spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AssuranceProfile => "AssuranceProfile",
            Self::MeasurementPlan => "MeasurementPlan",
            Self::ArchitectureDescription => "ArchitectureDescription",
            Self::ComponentAssuranceContract => "ComponentAssuranceContract",
            Self::AssuranceArgument => "AssuranceArgument",
        }
    }
}

impl fmt::Display for ArtifactType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::str::FromStr for ArtifactType {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|artifact_type| artifact_type.as_str() == value)
            .ok_or(())
    }
}

impl Serialize for ArtifactType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

/// Validation state for one recognized installed artifact.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactValidation {
    /// Repository-relative artifact path.
    pub path: String,
    /// Recognized artifact type.
    pub artifact_type: ArtifactType,
    /// Whether Quire accepted the artifact against the installed module.
    pub valid: bool,
    /// Ordered non-empty Quire or availability diagnostics.
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

/// Complete separated inventory used by the onboarding decision.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Inventory {
    /// Decision documents discovered in the selected root.
    pub decisions: Vec<String>,
    /// Recognized measurement plans and their validation state.
    pub measurements: Vec<ArtifactValidation>,
    /// Recognized non-measurement assurance artifacts.
    pub assurance_artifacts: Vec<ArtifactValidation>,
    /// Candidate evidence-reference files.
    pub evidence_references: Vec<String>,
    /// Candidate producer configuration files.
    pub producer_configurations: Vec<String>,
    /// Inputs deliberately not interpreted, with stable reason prefixes.
    pub unresolved_inputs: Vec<String>,
}

impl Inventory {
    /// Sort every inventory collection by its stable repository-relative key.
    pub fn sort(&mut self) {
        self.decisions.sort();
        self.measurements
            .sort_by(|left, right| left.path.cmp(&right.path));
        self.assurance_artifacts
            .sort_by(|left, right| left.path.cmp(&right.path));
        self.evidence_references.sort();
        self.producer_configurations.sort();
        self.unresolved_inputs.sort();
    }
}

/// Strict versioned onboarding request supplied to the CLI.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OnboardingRequest {
    /// Exact request protocol discriminator.
    pub protocol: String,
    /// Caller-selected absolute repository root.
    pub repository_root: String,
    /// Caller-selected absolute installed Engineering Assurance module root.
    pub module_root: String,
    /// Quire executable name or absolute path.
    #[serde(default = "default_quire_executable")]
    pub quire_executable: String,
    /// Exact decision boundary, when supplied.
    #[serde(default)]
    pub decision_boundary: Option<String>,
    /// Named human decision owner, when supplied.
    #[serde(default)]
    pub decision_owner: Option<String>,
    /// Requested artifact type, when the decision needs one.
    #[serde(default)]
    pub requested_artifact: Option<ArtifactType>,
    /// Why a new artifact is justified, when one is requested.
    #[serde(default)]
    pub justification: Option<String>,
    /// Repository-relative publication target.
    #[serde(default)]
    pub target: Option<String>,
    /// Frontmatter replacements for a newly authored artifact.
    #[serde(default)]
    pub frontmatter: Option<JsonMap<String, JsonValue>>,
}

fn default_quire_executable() -> String {
    "quire".to_owned()
}

/// Stable result status for one complete onboarding operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OnboardingStatus {
    /// More boundary or authoring input is required.
    NeedsInput,
    /// No governed artifact work is justified.
    NoApplicableWork,
    /// Existing invalid or duplicate candidates require a human selection.
    NeedsHumanSelection,
    /// One existing valid artifact should be reused.
    Reuse,
    /// One justified artifact was validated and published.
    Authored,
}

/// Versioned complete onboarding result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OnboardingResult {
    /// Exact result protocol discriminator.
    pub protocol: &'static str,
    /// Result status.
    pub status: OnboardingStatus,
    /// Complete inventory produced before the recommendation.
    pub inventory: Inventory,
    /// Human-readable retained recommendation.
    pub recommendation: String,
    /// Existing or published repository-relative artifact path.
    pub artifact_path: Option<String>,
}

impl OnboardingResult {
    /// Encode this result as one compact JSON value followed by one newline.
    ///
    /// # Errors
    ///
    /// Returns [`OnboardingError::ResultSerialization`] if serialization fails.
    pub fn to_json_line(&self) -> Result<Vec<u8>, OnboardingError> {
        let mut bytes =
            serde_json::to_vec(self).map_err(|error| OnboardingError::ResultSerialization {
                detail: error.to_string(),
            })?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

/// A publication requested only after the inventory and recommendation gates.
#[derive(Clone, Debug, PartialEq)]
pub struct PublicationPlan {
    /// Complete pre-publication inventory.
    pub inventory: Inventory,
    /// Installed skeleton type to render.
    pub artifact_type: ArtifactType,
    /// Confined repository-relative target supplied by the caller.
    pub target: String,
    /// Caller-supplied frontmatter replacements.
    pub frontmatter: JsonMap<String, JsonValue>,
}

/// Pure decision returned before any filesystem publication is attempted.
#[derive(Clone, Debug, PartialEq)]
pub enum OnboardingPlan {
    /// No publication is required.
    Complete(OnboardingResult),
    /// The CLI must validate and atomically publish one artifact.
    Publish(PublicationPlan),
}

/// Stable failures at the onboarding request or pure rendering boundary.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum OnboardingError {
    /// Request JSON does not match the closed wire contract.
    #[error("invalid onboarding request: {detail}")]
    InvalidRequest {
        /// Parser or structural-validation detail.
        detail: String,
    },
    /// The request names an unsupported protocol.
    #[error("unsupported onboarding protocol {observed:?}")]
    UnsupportedProtocol {
        /// Protocol received from the caller.
        observed: String,
    },
    /// An installed skeleton has no exact frontmatter envelope.
    #[error("installed {artifact_type} skeleton has malformed frontmatter")]
    MalformedSkeleton {
        /// Requested artifact type.
        artifact_type: ArtifactType,
    },
    /// YAML frontmatter could not be parsed or serialized.
    #[error("frontmatter conversion failed: {detail}")]
    Frontmatter {
        /// YAML parser or serializer detail.
        detail: String,
    },
    /// A typed result could not be serialized.
    #[error("onboarding result serialization failed: {detail}")]
    ResultSerialization {
        /// Serializer detail.
        detail: String,
    },
}

impl OnboardingError {
    /// Stable machine-readable diagnostic code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequest { .. } | Self::Frontmatter { .. } => "onboarding_request_invalid",
            Self::UnsupportedProtocol { .. } => "unsupported_onboarding_protocol",
            Self::MalformedSkeleton { .. } => "onboarding_publication_failed",
            Self::ResultSerialization { .. } => "onboarding_result_serialization_failed",
        }
    }
}

/// Parse one strict versioned onboarding request.
///
/// # Errors
///
/// Returns a typed refusal for malformed JSON or an unsupported protocol.
pub fn parse_request_bytes(bytes: &[u8]) -> Result<OnboardingRequest, OnboardingError> {
    let request = serde_json::from_slice::<OnboardingRequest>(bytes).map_err(|error| {
        OnboardingError::InvalidRequest {
            detail: error.to_string(),
        }
    })?;
    if request.protocol != REQUEST_PROTOCOL {
        return Err(OnboardingError::UnsupportedProtocol {
            observed: request.protocol,
        });
    }
    Ok(request)
}

/// Produce the retained onboarding recommendation after inventory, without I/O.
#[must_use]
pub fn plan(request: &OnboardingRequest, inventory: Inventory) -> OnboardingPlan {
    if request
        .decision_boundary
        .as_deref()
        .is_none_or(str::is_empty)
        || request.decision_owner.as_deref().is_none_or(str::is_empty)
    {
        return OnboardingPlan::Complete(result(
            OnboardingStatus::NeedsInput,
            inventory,
            "Provide the exact decision boundary and human decision owner.",
            None,
        ));
    }
    let Some(artifact_type) = request.requested_artifact else {
        return OnboardingPlan::Complete(result(
            OnboardingStatus::NoApplicableWork,
            inventory,
            "No assurance artifact or governed workflow is justified by the request.",
            None,
        ));
    };
    let candidates = inventory
        .measurements
        .iter()
        .chain(&inventory.assurance_artifacts)
        .filter(|item| item.artifact_type == artifact_type)
        .collect::<Vec<_>>();
    if candidates.iter().any(|item| !item.valid) || candidates.len() > 1 {
        return OnboardingPlan::Complete(result(
            OnboardingStatus::NeedsHumanSelection,
            inventory,
            "Applicable artifacts are malformed or conflicting; preserve them and select or correct one.",
            None,
        ));
    }
    if let [candidate] = candidates.as_slice() {
        let artifact_path = candidate.path.clone();
        return OnboardingPlan::Complete(result(
            OnboardingStatus::Reuse,
            inventory,
            format!("Reuse the applicable validated {artifact_type}."),
            Some(artifact_path),
        ));
    }
    if request.justification.as_deref().is_none_or(str::is_empty) {
        return OnboardingPlan::Complete(result(
            OnboardingStatus::NoApplicableWork,
            inventory,
            format!("No justification was supplied for a new {artifact_type}."),
            None,
        ));
    }
    let (Some(target), Some(frontmatter)) = (&request.target, &request.frontmatter) else {
        return OnboardingPlan::Complete(result(
            OnboardingStatus::NeedsInput,
            inventory,
            "Provide a confined target and artifact frontmatter before authoring.",
            None,
        ));
    };
    OnboardingPlan::Publish(PublicationPlan {
        inventory,
        artifact_type,
        target: target.clone(),
        frontmatter: frontmatter.clone(),
    })
}

fn result(
    status: OnboardingStatus,
    inventory: Inventory,
    recommendation: impl Into<String>,
    artifact_path: Option<String>,
) -> OnboardingResult {
    OnboardingResult {
        protocol: RESULT_PROTOCOL,
        status,
        inventory,
        recommendation: recommendation.into(),
        artifact_path,
    }
}

/// Complete a successful publication plan with the retained authored result.
#[must_use]
pub fn authored_result(plan: PublicationPlan) -> OnboardingResult {
    let artifact_path = plan.target;
    result(
        OnboardingStatus::Authored,
        plan.inventory,
        format!(
            "Authored and validated one justified {}.",
            plan.artifact_type
        ),
        Some(artifact_path),
    )
}

/// Parse a Markdown frontmatter envelope and return its string `type`, if any.
///
/// # Errors
///
/// Returns [`OnboardingError::Frontmatter`] when an envelope exists but is not
/// a YAML mapping, cannot be parsed, or carries ambiguous merge-key identity.
pub fn frontmatter_type(text: &str) -> Result<Option<String>, OnboardingError> {
    let Some((frontmatter, _)) = split_frontmatter(text) else {
        return Ok(None);
    };
    let mapping = parse_frontmatter_mapping(frontmatter)?;
    Ok(mapping
        .get(YamlValue::String("type".to_owned()))
        .and_then(YamlValue::as_str)
        .map(str::to_owned))
}

/// Render one installed skeleton with caller-supplied frontmatter replacements.
///
/// # Errors
///
/// Returns a typed failure when the skeleton envelope or YAML is malformed or
/// carries ambiguous merge-key identity.
pub fn render_from_skeleton(
    artifact_type: ArtifactType,
    skeleton: &str,
    replacements: &JsonMap<String, JsonValue>,
) -> Result<String, OnboardingError> {
    let Some((frontmatter, body)) = split_frontmatter(skeleton) else {
        return Err(OnboardingError::MalformedSkeleton { artifact_type });
    };
    let mut mapping = parse_frontmatter_mapping(frontmatter)?;
    for (key, value) in replacements {
        let value = yaml_serde::to_value(value).map_err(|error| OnboardingError::Frontmatter {
            detail: error.to_string(),
        })?;
        mapping.insert(YamlValue::String(key.clone()), value);
    }
    mapping.insert(
        YamlValue::String("type".to_owned()),
        YamlValue::String(artifact_type.as_str().to_owned()),
    );
    let title = mapping
        .get(YamlValue::String("title".to_owned()))
        .and_then(YamlValue::as_str)
        .filter(|title| !title.trim().is_empty());
    let body = title.map_or_else(|| body.to_owned(), |title| replace_title(body, title));
    let frontmatter =
        yaml_serde::to_string(&mapping).map_err(|error| OnboardingError::Frontmatter {
            detail: error.to_string(),
        })?;
    Ok(format!(
        "---\n{}\n---\n{body}",
        frontmatter.trim_end_matches('\n')
    ))
}

fn parse_frontmatter_mapping(frontmatter: &str) -> Result<YamlMap, OnboardingError> {
    let mapping = yaml_serde::from_str::<YamlMap>(frontmatter).map_err(|error| {
        OnboardingError::Frontmatter {
            detail: error.to_string(),
        }
    })?;
    if mapping.contains_key(YamlValue::String("<<".to_owned())) {
        return Err(OnboardingError::Frontmatter {
            detail: "YAML merge keys cannot supply artifact identity".to_owned(),
        });
    }
    Ok(mapping)
}

fn split_frontmatter(text: &str) -> Option<(&str, &str)> {
    let remainder = text.strip_prefix("---\n")?;
    if let Some(index) = remainder.find("\n---\n") {
        return Some((&remainder[..index], &remainder[index + 5..]));
    }
    remainder
        .strip_suffix("\n---")
        .map(|frontmatter| (frontmatter, ""))
}

fn replace_title(body: &str, title: &str) -> String {
    let (prefix, rest) = body
        .strip_prefix("\n# ")
        .map_or(("", body), |rest| ("\n", rest));
    let Some(end) = rest.find('\n') else {
        return body.to_owned();
    };
    format!("{prefix}# {title}{}", &rest[end..])
}
