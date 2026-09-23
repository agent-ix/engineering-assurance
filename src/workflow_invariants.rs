// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Deterministic evaluation of Engineering Assurance workflow invariants.
//!
//! This module owns only the repository-specific invariant semantics. It does
//! not load workflows, inspect a repository, read the clock, or mutate ix-flow
//! state. A caller supplies one closed projection and one evaluation instant.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    str::FromStr,
};

use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use thiserror::Error;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/// Protocol accepted by [`evaluate_request_bytes`].
pub const REQUEST_PROTOCOL: &str = "engineering-assurance.workflow-invariants/v1";

/// Protocol emitted by a successfully evaluated invariant batch.
pub const RESULT_PROTOCOL: &str = "engineering-assurance.workflow-invariants-result/v1";

/// The complete closed set of Engineering Assurance workflow invariants.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum InvariantName {
    /// Require at least one valid operator observation.
    SharedObservationReady,
    /// Require every terminal transition to remain human-gated.
    SharedTerminalGates,
    /// Require current, owned exceptions when the request needs them.
    SharedExceptionsReady,
    /// Require a complete assurance-intake boundary.
    IntakeScopeReady,
    /// Require validation for every requested assurance artifact kind.
    IntakeArtifactsReady,
    /// Require a validated architecture description and complete scenarios.
    ArchitectureScenariosReady,
    /// Require the architecture review to address the selected description.
    ArchitectureReviewReady,
    /// Require exactly one adjacent measurement-maturity promotion.
    MeasurementPromotionReady,
    /// Require a complete impact snapshot at the selected source revision.
    ChangeImpactReady,
    /// Require a current, internally consistent assurance snapshot.
    ChangeSnapshotReady,
    /// Require a code review at the selected source revision.
    ChangeReviewReady,
}

impl InvariantName {
    /// Every invariant in canonical registration order.
    pub const ALL: [Self; 11] = [
        Self::SharedObservationReady,
        Self::SharedTerminalGates,
        Self::SharedExceptionsReady,
        Self::IntakeScopeReady,
        Self::IntakeArtifactsReady,
        Self::ArchitectureScenariosReady,
        Self::ArchitectureReviewReady,
        Self::MeasurementPromotionReady,
        Self::ChangeImpactReady,
        Self::ChangeSnapshotReady,
        Self::ChangeReviewReady,
    ];

    /// Return the canonical ix-flow registration name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SharedObservationReady => "shared.observation_ready",
            Self::SharedTerminalGates => "shared.terminal_gates",
            Self::SharedExceptionsReady => "shared.exceptions_ready",
            Self::IntakeScopeReady => "intake.scope_ready",
            Self::IntakeArtifactsReady => "intake.artifacts_ready",
            Self::ArchitectureScenariosReady => "architecture.scenarios_ready",
            Self::ArchitectureReviewReady => "architecture.review_ready",
            Self::MeasurementPromotionReady => "measurement.promotion_ready",
            Self::ChangeImpactReady => "change.impact_ready",
            Self::ChangeSnapshotReady => "change.snapshot_ready",
            Self::ChangeReviewReady => "change.review_ready",
        }
    }
}

impl fmt::Display for InvariantName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for InvariantName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl FromStr for InvariantName {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == value)
            .ok_or(())
    }
}

/// Closed input projection used by the reusable invariant evaluator.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InvariantProjection {
    /// Exact ix-flow definition name.
    def_name: String,
    /// Repository-owned item kinds inspected by the invariant population.
    #[serde(default)]
    items: Items,
    /// Transition-specific gate modes selected for the run.
    #[serde(default)]
    gate_config: BTreeMap<String, String>,
}

/// A validated batch ready for deterministic evaluation.
#[derive(Clone, Debug, PartialEq)]
struct WorkflowInvariantRequest {
    invariants: Vec<InvariantName>,
    workflow: WorkflowName,
    instance: InvariantProjection,
    evaluated_at: OffsetDateTime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WorkflowName {
    AssuranceIntake,
    ArchitectureEvaluation,
    MeasurementPromotion,
    ChangeAssurance,
}

impl WorkflowName {
    const fn terminal_transitions(self) -> &'static [&'static str] {
        match self {
            Self::AssuranceIntake | Self::ArchitectureEvaluation => {
                &["decision_ready->accepted", "decision_ready->rejected"]
            }
            Self::MeasurementPromotion => {
                &["decision_ready->promoted", "decision_ready->not_promoted"]
            }
            Self::ChangeAssurance => &["decision_ready->approved", "decision_ready->rejected"],
        }
    }
}

impl FromStr for WorkflowName {
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

/// One stable invariant-failure code.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InvariantFailureCode {
    /// No complete non-negative observation exists.
    OperatorObservationMissingOrInvalid,
    /// A terminal transition is not human-gated.
    TerminalGateOverride,
    /// A required exception is missing, incomplete, invalid, or expired.
    OwnedCurrentExceptionRequired,
    /// The intake request lacks its decision boundary.
    IntakeScopeIncomplete,
    /// The intake request does not carry an artifact-kind list.
    RequestedArtifactsMissing,
    /// A requested artifact kind lacks successful validation.
    ArtifactValidationMissing,
    /// The architecture description or scenario population is incomplete.
    ArchitectureContextIncomplete,
    /// No matching architecture review exists.
    ArchitectureReviewMissing,
    /// A measurement promotion skips or repeats a maturity stage.
    PromotionMustAdvanceOneStage,
    /// The selected promotion lacks complete matching evidence.
    PromotionEvidenceIncomplete,
    /// A required checker result for the promoted plan is absent.
    PromotionCheckerMissing,
    /// Checker results exist for the plan, but none matches the evidence's
    /// definition version and candidate collection.
    PromotionCheckerMismatch,
    /// The matching checker result did not accept the candidate.
    PromotionCheckerNotAccepted,
    /// The matching checker result accepted over an unattested intake order.
    PromotionCheckerOrderUnattested,
    /// The impact snapshot is absent or incomplete.
    ImpactSnapshotIncomplete,
    /// The assurance snapshot is absent, stale, or malformed.
    AssuranceSnapshotInvalid,
    /// No matching code review exists.
    CodeReviewMissing,
}

/// Structured details carried by the failure families that need them.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvariantFailureDetails {
    /// Terminal transition keys whose gate mode is not `hitl`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub transitions: Vec<String>,
    /// Requested artifact kinds without a successful validation.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub artifact_types: Vec<String>,
    /// What a measurement promotion found about its checker result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checker: Option<CheckerReport>,
}

/// The measurement-policy mode in force for one promotion.
///
/// The `AssuranceProfile` `measurement_policy` names the stages its mode
/// governs; every other stage, and a run with no policy, is `recommend`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementPolicyMode {
    /// The checker result never refuses a promotion.
    Recommend,
    /// The promotion needs an accepted, attested checker result or a current
    /// owned exception.
    Require,
}

/// The verdict a `quoin.measurement-verdict.v1` document reports.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckerVerdict {
    /// The checker accepted the candidate collection.
    Accept,
    /// The checker rejected the candidate collection.
    Reject,
    /// The checker could not decide.
    Inconclusive,
}

/// What a promotion found about the checker result bound to its evidence.
///
/// Variants are ordered from most to least favourable; when several results
/// match, the greatest status is reported.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckerStatus {
    /// A matching result accepted the candidate over an attested order.
    Accepted,
    /// The matching result accepted over an order the producer could choose.
    OrderUnattested,
    /// The matching result rejected the candidate or was inconclusive.
    NotAccepted,
    /// Results exist for the plan, but none for this definition version and
    /// candidate collection.
    Mismatch,
    /// No result exists for the plan.
    Missing,
}

impl CheckerStatus {
    /// The failure code this status carries when the policy requires the
    /// checker, or `None` when it satisfies the requirement.
    #[must_use]
    pub const fn failure_code(self) -> Option<InvariantFailureCode> {
        match self {
            Self::Accepted => None,
            Self::OrderUnattested => Some(InvariantFailureCode::PromotionCheckerOrderUnattested),
            Self::NotAccepted => Some(InvariantFailureCode::PromotionCheckerNotAccepted),
            Self::Mismatch => Some(InvariantFailureCode::PromotionCheckerMismatch),
            Self::Missing => Some(InvariantFailureCode::PromotionCheckerMissing),
        }
    }
}

/// The checker result a refused `require` promotion reports in its details.
///
/// A passing outcome carries no details: ix-flow's external-provider result
/// contract admits only `invariant` and `status` on a passed outcome. The
/// checker's verdict stays visible through the recorded `measurement_verdict`
/// item, and an owner override through the recorded `exception` item.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CheckerReport {
    /// What the promotion found.
    pub status: CheckerStatus,
    /// The matching result's verdict; `null` when no result matches.
    pub verdict: Option<CheckerVerdict>,
    /// The matching result's reason codes; empty when no result matches.
    pub reasons: Vec<String>,
}

/// The verdict for one requested invariant.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum InvariantVerdict {
    /// The invariant holds.
    Passed,
    /// The invariant does not hold.
    Failed {
        /// Stable machine-readable reason.
        code: InvariantFailureCode,
        /// Typed failure details; empty for scalar failure families.
        details: InvariantFailureDetails,
    },
}

/// One result in the same order as the request's invariant-name list.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvariantOutcome {
    /// Evaluated canonical invariant.
    pub invariant: InvariantName,
    /// Passed or failed result.
    #[serde(flatten)]
    pub verdict: InvariantVerdict,
}

/// Versioned deterministic result for one invariant batch.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowInvariantResult {
    /// Exact result protocol discriminator.
    pub protocol: &'static str,
    /// One outcome per requested name, in request order.
    pub outcomes: Vec<InvariantOutcome>,
}

impl WorkflowInvariantResult {
    /// Encode this result as one compact JSON value followed by one newline.
    ///
    /// # Errors
    ///
    /// Returns [`WorkflowInvariantError::ResultSerialization`] if serialization
    /// unexpectedly fails.
    pub fn to_json_line(&self) -> Result<Vec<u8>, WorkflowInvariantError> {
        let mut bytes = serde_json::to_vec(self).map_err(|error| {
            WorkflowInvariantError::ResultSerialization {
                detail: error.to_string(),
            }
        })?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

/// Stable failures at the workflow-invariant request boundary.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum WorkflowInvariantError {
    /// The request is not strict JSON matching the closed wire projection.
    #[error("invalid workflow invariant request: {detail}")]
    InvalidRequest {
        /// Parser or structural-validation detail.
        detail: String,
    },
    /// The request names a protocol this implementation does not support.
    #[error("unsupported workflow invariant protocol {observed:?}")]
    UnsupportedProtocol {
        /// Protocol received from the caller.
        observed: String,
    },
    /// The requested invariant list is empty.
    #[error("workflow invariant request contains no invariant names")]
    EmptyInvariantSet,
    /// One invariant name occurs more than once.
    #[error("duplicate workflow invariant {name:?}")]
    DuplicateInvariant {
        /// Duplicated canonical name.
        name: String,
    },
    /// The request names no registered invariant.
    #[error("unknown workflow invariant {name:?}")]
    UnknownInvariant {
        /// Unknown requested name.
        name: String,
    },
    /// The request's evaluation instant is not canonical RFC 3339.
    #[error("invalid workflow invariant evaluation instant {observed:?}")]
    InvalidEvaluationInstant {
        /// Invalid supplied instant.
        observed: String,
    },
    /// The projection names no canonical Engineering Assurance workflow.
    #[error("unknown Engineering Assurance workflow {name:?}")]
    UnknownWorkflow {
        /// Unknown workflow definition name.
        name: String,
    },
    /// A typed result could not be serialized.
    #[error("workflow invariant result serialization failed: {detail}")]
    ResultSerialization {
        /// Serializer failure detail.
        detail: String,
    },
}

impl WorkflowInvariantError {
    /// Stable machine-readable diagnostic code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedProtocol { .. } => "unsupported_workflow_invariant_protocol",
            Self::UnknownInvariant { .. } => "workflow_invariant_unknown",
            Self::UnknownWorkflow { .. } => "workflow_binding_invalid",
            Self::InvalidRequest { .. }
            | Self::EmptyInvariantSet
            | Self::DuplicateInvariant { .. }
            | Self::InvalidEvaluationInstant { .. } => "workflow_invariant_request_invalid",
            Self::ResultSerialization { .. } => "workflow_invariant_result_serialization_failed",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRequest {
    protocol: String,
    invariants: Vec<String>,
    instance: InvariantProjection,
    evaluated_at: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct Items {
    #[serde(rename = "run_binding")]
    _run_binding: Vec<Value>,
    intake_request: Vec<IntakeRequest>,
    architecture_request: Vec<ArchitectureRequest>,
    promotion_request: Vec<PromotionRequest>,
    change_request: Vec<ChangeRequest>,
    artifact_validation: Vec<ArtifactValidation>,
    architecture_scenario: Vec<ArchitectureScenario>,
    review_validation: Vec<ReviewValidation>,
    promotion_evidence: Vec<PromotionEvidence>,
    measurement_policy: Vec<MeasurementPolicy>,
    measurement_verdict: Vec<VerdictItem>,
    impact_snapshot: Vec<ImpactSnapshot>,
    assurance_snapshot: Vec<AssuranceSnapshot>,
    exception: Vec<ExceptionItem>,
    operator_observation: Vec<OperatorObservation>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct IntakeRequest {
    #[serde(rename = "id")]
    _id: Option<String>,
    #[serde(rename = "interviewId")]
    interview_id: Option<String>,
    scope: Option<String>,
    boundary: Option<String>,
    impact_scenario: Option<String>,
    owner: Option<String>,
    requested_artifacts: Option<Vec<String>>,
    exceptions_expected: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct ArchitectureRequest {
    #[serde(rename = "id")]
    _id: Option<String>,
    #[serde(rename = "interviewId")]
    interview_id: Option<String>,
    #[serde(rename = "scope")]
    _scope: Option<String>,
    description_path: Option<String>,
    #[serde(rename = "concerns")]
    _concerns: Option<Vec<String>>,
    #[serde(rename = "owner")]
    _owner: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct PromotionRequest {
    #[serde(rename = "id")]
    _id: Option<String>,
    #[serde(rename = "interviewId")]
    interview_id: Option<String>,
    plan_path: Option<String>,
    definition_version: Option<String>,
    prior_stage: Option<String>,
    proposed_stage: Option<String>,
    #[serde(rename = "decision_use")]
    _decision_use: Option<String>,
    #[serde(rename = "owner")]
    _owner: Option<String>,
    exceptions_expected: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct ChangeRequest {
    #[serde(rename = "id")]
    _id: Option<String>,
    #[serde(rename = "interviewId")]
    interview_id: Option<String>,
    #[serde(rename = "change_ref")]
    _change_ref: Option<String>,
    #[serde(rename = "scope")]
    _scope: Option<String>,
    profile_path: Option<String>,
    baseline_id: Option<String>,
    source_revision: Option<String>,
    #[serde(rename = "owner")]
    _owner: Option<String>,
    exceptions_expected: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct ArtifactValidation {
    #[serde(rename = "id")]
    _id: Option<String>,
    artifact_type: Option<String>,
    path: Option<String>,
    valid: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct ArchitectureScenario {
    #[serde(rename = "id")]
    _id: Option<String>,
    concern: Option<String>,
    stimulus: Option<String>,
    environment: Option<String>,
    response: Option<String>,
    measure: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct ReviewValidation {
    #[serde(rename = "id")]
    _id: Option<String>,
    artifact_type: Option<String>,
    analysis: Option<String>,
    path: Option<String>,
    subject_path: Option<String>,
    source_revision: Option<String>,
    valid: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct PromotionEvidence {
    #[serde(rename = "id")]
    _id: Option<String>,
    plan_path: Option<String>,
    definition_version: Option<String>,
    prior_stage: Option<String>,
    proposed_stage: Option<String>,
    stability: Option<String>,
    decision_yield: Option<String>,
    limitations: Option<String>,
    owner: Option<String>,
    plan_id: Option<String>,
    candidate: Option<String>,
}

/// The `AssuranceProfile` `measurement_policy`, recorded into the run.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct MeasurementPolicy {
    #[serde(rename = "id")]
    _id: Option<String>,
    #[serde(rename = "profile_path")]
    _profile_path: Option<String>,
    mode: Option<MeasurementPolicyMode>,
    stages: Option<Vec<String>>,
}

/// The `schema` a checker result must carry to be read.
const VERDICT_SCHEMA: &str = "quoin.measurement-verdict.v1";

/// The only order source a producer cannot choose after the fact.
const ATTESTED_ORDER_SOURCE: &str = "git-first-parent-add";

/// One recorded `measurement_verdict` item.
///
/// Only a document carrying [`VERDICT_SCHEMA`] is read, and it is read
/// strictly. A document of any other schema is kept opaque and never read, so
/// a later checker schema neither refuses the request nor blocks a
/// `recommend` promotion.
/// `None` holds a document of another schema.
#[derive(Clone, Debug, PartialEq)]
struct VerdictItem(Option<MeasurementVerdict>);

impl<'de> Deserialize<'de> for VerdictItem {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let value = Value::deserialize(deserializer)?;
        let Some(object) = value.as_object() else {
            return Err(D::Error::custom(
                "a measurement_verdict item must be an object",
            ));
        };
        if object.get("schema").and_then(Value::as_str) != Some(VERDICT_SCHEMA) {
            return Ok(Self(None));
        }
        MeasurementVerdict::deserialize(value)
            .map(|verdict| Self(Some(verdict)))
            .map_err(D::Error::custom)
    }
}

/// One `quoin.measurement-verdict.v1` document recorded as a run item.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
struct MeasurementVerdict {
    #[serde(rename = "id")]
    _id: Option<String>,
    #[serde(rename = "schema")]
    _schema: Option<String>,
    plan_id: Option<String>,
    definition_version: Option<String>,
    verdict: Option<CheckerVerdict>,
    reasons: Option<Vec<String>>,
    #[serde(rename = "claimed")]
    _claimed: Option<CheckerVerdict>,
    candidate: Option<String>,
    #[serde(rename = "decisions")]
    _decisions: Option<Vec<Value>>,
    #[serde(rename = "findings")]
    _findings: Option<Vec<Value>>,
    #[serde(rename = "regressedRuns")]
    _regressed_runs: Option<Vec<String>>,
    order_source: Option<String>,
    #[serde(rename = "counts")]
    _counts: Option<serde_json::Map<String, Value>>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct ImpactSnapshot {
    #[serde(rename = "id")]
    _id: Option<String>,
    source_revision: Option<String>,
    profile_path: Option<String>,
    baseline_id: Option<String>,
    changed_nodes: Option<Vec<Value>>,
    impacted_nodes: Option<Vec<Value>>,
    missing_edges: Option<Vec<Value>>,
    stale_evidence: Option<Vec<Value>>,
    suspect_evidence: Option<Vec<Value>>,
    unknowns: Option<Vec<Value>>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct AssuranceSnapshot {
    #[serde(rename = "id")]
    _id: Option<String>,
    source_revision: Option<String>,
    discharge_schema: Option<String>,
    discharge_digest: Option<String>,
    binding_open: Option<i64>,
    applicability_unresolved: Option<i64>,
    argument_schema: Option<String>,
    argument_id: Option<String>,
    argument_digest: Option<String>,
    top_claim_status: Option<String>,
    evidence_record_ids: Option<Vec<String>>,
    verified_at: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct ExceptionItem {
    #[serde(rename = "id")]
    _id: Option<String>,
    owner: Option<String>,
    expires_at: Option<String>,
    rationale: Option<String>,
    impact: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct OperatorObservation {
    #[serde(rename = "id")]
    _id: Option<String>,
    elapsed_minutes: Option<i64>,
    command_count: Option<i64>,
}

/// Parse and evaluate one strict versioned request.
///
/// # Errors
///
/// Returns a typed refusal for malformed JSON, an unsupported protocol, an
/// empty/duplicate/unknown invariant population, or an invalid evaluation
/// instant.
pub fn evaluate_request_bytes(
    bytes: &[u8],
) -> Result<WorkflowInvariantResult, WorkflowInvariantError> {
    let wire: WireRequest =
        serde_json::from_slice(bytes).map_err(|error| WorkflowInvariantError::InvalidRequest {
            detail: error.to_string(),
        })?;
    if wire.protocol != REQUEST_PROTOCOL {
        return Err(WorkflowInvariantError::UnsupportedProtocol {
            observed: wire.protocol,
        });
    }
    if wire.invariants.is_empty() {
        return Err(WorkflowInvariantError::EmptyInvariantSet);
    }
    let workflow = wire
        .instance
        .def_name
        .parse::<WorkflowName>()
        .map_err(|()| WorkflowInvariantError::UnknownWorkflow {
            name: wire.instance.def_name.clone(),
        })?;
    let mut names = Vec::with_capacity(wire.invariants.len());
    let mut unique_names = BTreeSet::new();
    for name in wire.invariants {
        let parsed = name
            .parse::<InvariantName>()
            .map_err(|()| WorkflowInvariantError::UnknownInvariant { name: name.clone() })?;
        if !unique_names.insert(parsed) {
            return Err(WorkflowInvariantError::DuplicateInvariant { name });
        }
        names.push(parsed);
    }
    let evaluated_at = OffsetDateTime::parse(&wire.evaluated_at, &Rfc3339).map_err(|_| {
        WorkflowInvariantError::InvalidEvaluationInstant {
            observed: wire.evaluated_at,
        }
    })?;
    Ok(evaluate(&WorkflowInvariantRequest {
        invariants: names,
        workflow,
        instance: wire.instance,
        evaluated_at,
    }))
}

/// Evaluate a validated batch without filesystem, process, network, environment,
/// or clock access.
#[must_use]
fn evaluate(request: &WorkflowInvariantRequest) -> WorkflowInvariantResult {
    let outcomes = request
        .invariants
        .iter()
        .copied()
        .map(|invariant| InvariantOutcome {
            invariant,
            verdict: evaluate_one(
                invariant,
                request.workflow,
                &request.instance,
                request.evaluated_at,
            ),
        })
        .collect();
    WorkflowInvariantResult {
        protocol: RESULT_PROTOCOL,
        outcomes,
    }
}

fn evaluate_one(
    invariant: InvariantName,
    workflow: WorkflowName,
    instance: &InvariantProjection,
    evaluated_at: OffsetDateTime,
) -> InvariantVerdict {
    let failure = match invariant {
        InvariantName::SharedObservationReady => observation_ready(instance),
        InvariantName::SharedTerminalGates => terminal_gates(workflow, instance),
        InvariantName::SharedExceptionsReady => exceptions_ready(instance, evaluated_at),
        InvariantName::IntakeScopeReady => intake_scope_ready(instance),
        InvariantName::IntakeArtifactsReady => intake_artifacts_ready(instance),
        InvariantName::ArchitectureScenariosReady => architecture_scenarios_ready(instance),
        InvariantName::ArchitectureReviewReady => architecture_review_ready(instance),
        InvariantName::MeasurementPromotionReady => promotion_ready(instance, evaluated_at),
        InvariantName::ChangeImpactReady => change_impact_ready(instance),
        InvariantName::ChangeSnapshotReady => change_snapshot_ready(instance, evaluated_at),
        InvariantName::ChangeReviewReady => change_review_ready(instance),
    };
    match failure {
        None => InvariantVerdict::Passed,
        Some((code, details)) => InvariantVerdict::Failed { code, details },
    }
}

type Failure = (InvariantFailureCode, InvariantFailureDetails);

fn failure(code: InvariantFailureCode) -> Failure {
    (code, InvariantFailureDetails::default())
}

fn nonempty(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

fn observation_ready(instance: &InvariantProjection) -> Option<Failure> {
    let observations = &instance.items.operator_observation;
    let valid = !observations.is_empty()
        && observations.iter().all(|item| {
            item.elapsed_minutes.is_some_and(|value| value >= 0)
                && item.command_count.is_some_and(|value| value >= 1)
        });
    (!valid).then_some((
        InvariantFailureCode::OperatorObservationMissingOrInvalid,
        InvariantFailureDetails::default(),
    ))
}

fn terminal_gates(workflow: WorkflowName, instance: &InvariantProjection) -> Option<Failure> {
    let changed = workflow
        .terminal_transitions()
        .iter()
        .filter(|key| instance.gate_config.get(**key).map(String::as_str) != Some("hitl"))
        .map(|key| (*key).to_owned())
        .collect::<Vec<_>>();
    (!changed.is_empty()).then_some((
        InvariantFailureCode::TerminalGateOverride,
        InvariantFailureDetails {
            transitions: changed,
            artifact_types: Vec::new(),
            checker: None,
        },
    ))
}

fn exceptions_ready(
    instance: &InvariantProjection,
    evaluated_at: OffsetDateTime,
) -> Option<Failure> {
    let required = instance
        .items
        .intake_request
        .iter()
        .find(|item| nonempty(item.interview_id.as_deref()))
        .map(|item| item.exceptions_expected == Some(true))
        .or_else(|| {
            instance
                .items
                .promotion_request
                .iter()
                .find(|item| nonempty(item.interview_id.as_deref()))
                .map(|item| item.exceptions_expected == Some(true))
        })
        .or_else(|| {
            instance
                .items
                .change_request
                .iter()
                .find(|item| nonempty(item.interview_id.as_deref()))
                .map(|item| item.exceptions_expected == Some(true))
        })
        .unwrap_or(false);
    let exceptions = &instance.items.exception;
    let invalid = exceptions
        .iter()
        .any(|item| !exception_is_current(item, evaluated_at));
    (invalid || (required && exceptions.is_empty())).then_some((
        InvariantFailureCode::OwnedCurrentExceptionRequired,
        InvariantFailureDetails::default(),
    ))
}

/// Whether an exception is complete, owned, and unexpired at `evaluated_at`.
fn exception_is_current(item: &ExceptionItem, evaluated_at: OffsetDateTime) -> bool {
    nonempty(item.owner.as_deref())
        && nonempty(item.rationale.as_deref())
        && nonempty(item.impact.as_deref())
        && item
            .expires_at
            .as_deref()
            .and_then(parse_instant)
            .is_some_and(|expires_at| expires_at > evaluated_at)
}

fn intake_scope_ready(instance: &InvariantProjection) -> Option<Failure> {
    let valid = instance
        .items
        .intake_request
        .iter()
        .find(|item| nonempty(item.interview_id.as_deref()))
        .is_some_and(|item| {
            nonempty(item.scope.as_deref())
                && nonempty(item.boundary.as_deref())
                && nonempty(item.impact_scenario.as_deref())
                && nonempty(item.owner.as_deref())
        });
    (!valid).then_some((
        InvariantFailureCode::IntakeScopeIncomplete,
        InvariantFailureDetails::default(),
    ))
}

fn intake_artifacts_ready(instance: &InvariantProjection) -> Option<Failure> {
    let request = instance
        .items
        .intake_request
        .iter()
        .find(|item| nonempty(item.interview_id.as_deref()));
    let Some(requested) = request.and_then(|item| item.requested_artifacts.as_ref()) else {
        return Some(failure(InvariantFailureCode::RequestedArtifactsMissing));
    };
    let valid = instance
        .items
        .artifact_validation
        .iter()
        .filter(|item| item.valid == Some(true))
        .filter_map(|item| item.artifact_type.as_deref())
        .collect::<BTreeSet<_>>();
    let absent = requested
        .iter()
        .filter(|kind| !valid.contains(kind.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    (!absent.is_empty()).then_some((
        InvariantFailureCode::ArtifactValidationMissing,
        InvariantFailureDetails {
            transitions: Vec::new(),
            artifact_types: absent,
            checker: None,
        },
    ))
}

fn architecture_scenarios_ready(instance: &InvariantProjection) -> Option<Failure> {
    let description_path = instance
        .items
        .architecture_request
        .iter()
        .find(|item| nonempty(item.interview_id.as_deref()))
        .and_then(|item| item.description_path.as_deref())
        .filter(|path| !path.trim().is_empty());
    let Some(description_path) = description_path else {
        return Some(failure(InvariantFailureCode::ArchitectureContextIncomplete));
    };
    let description = instance.items.artifact_validation.iter().any(|item| {
        item.valid == Some(true)
            && item.artifact_type.as_deref() == Some("ArchitectureDescription")
            && item.path.as_deref() == Some(description_path)
    });
    let scenarios = &instance.items.architecture_scenario;
    let complete = !scenarios.is_empty()
        && scenarios.iter().all(|item| {
            nonempty(item.concern.as_deref())
                && nonempty(item.stimulus.as_deref())
                && nonempty(item.environment.as_deref())
                && nonempty(item.response.as_deref())
                && nonempty(item.measure.as_deref())
        });
    (!(description && complete)).then_some((
        InvariantFailureCode::ArchitectureContextIncomplete,
        InvariantFailureDetails::default(),
    ))
}

fn architecture_review_ready(instance: &InvariantProjection) -> Option<Failure> {
    let description_path = instance
        .items
        .architecture_request
        .iter()
        .find(|item| nonempty(item.interview_id.as_deref()))
        .and_then(|item| item.description_path.as_deref())
        .filter(|path| !path.trim().is_empty());
    let Some(description_path) = description_path else {
        return Some(failure(InvariantFailureCode::ArchitectureReviewMissing));
    };
    let valid = instance.items.review_validation.iter().any(|item| {
        item.valid == Some(true)
            && item.artifact_type.as_deref() == Some("SpecReview")
            && item.analysis.as_deref() == Some("architecture-evaluation")
            && item.subject_path.as_deref() == Some(description_path)
            && nonempty(item.path.as_deref())
    });
    (!valid).then_some((
        InvariantFailureCode::ArchitectureReviewMissing,
        InvariantFailureDetails::default(),
    ))
}

fn promotion_ready(
    instance: &InvariantProjection,
    evaluated_at: OffsetDateTime,
) -> Option<Failure> {
    let request = instance
        .items
        .promotion_request
        .iter()
        .find(|item| nonempty(item.interview_id.as_deref()));
    let Some(request) = request else {
        return Some(failure(InvariantFailureCode::PromotionMustAdvanceOneStage));
    };
    let prior = request.prior_stage.as_deref().and_then(stage_index);
    let proposed = request.proposed_stage.as_deref().and_then(stage_index);
    if prior
        .zip(proposed)
        .is_none_or(|(prior, proposed)| prior.checked_add(1) != Some(proposed))
    {
        return Some(failure(InvariantFailureCode::PromotionMustAdvanceOneStage));
    }
    let evidence = instance.items.promotion_evidence.iter().find(|item| {
        item.plan_path == request.plan_path
            && item.definition_version == request.definition_version
            && item.prior_stage == request.prior_stage
            && item.proposed_stage == request.proposed_stage
    });
    let Some(evidence) = evidence.filter(|item| {
        nonempty(item.plan_path.as_deref())
            && nonempty(item.definition_version.as_deref())
            && nonempty(item.prior_stage.as_deref())
            && nonempty(item.proposed_stage.as_deref())
            && nonempty(item.stability.as_deref())
            && nonempty(item.decision_yield.as_deref())
            && nonempty(item.limitations.as_deref())
            && nonempty(item.owner.as_deref())
    }) else {
        return Some(failure(InvariantFailureCode::PromotionEvidenceIncomplete));
    };
    // `recommend` never refuses: the checker result is not even read.
    if policy_mode(instance, request.proposed_stage.as_deref()) == MeasurementPolicyMode::Recommend
    {
        return None;
    }
    let (status, selected) = checker_status(instance, evidence);
    let code = status.failure_code()?;
    // An owner override passes; the `exception` item stays in the run.
    if instance
        .items
        .exception
        .iter()
        .any(|item| exception_is_current(item, evaluated_at))
    {
        return None;
    }
    let checker = CheckerReport {
        status,
        verdict: selected.and_then(|item| item.verdict),
        reasons: selected
            .and_then(|item| item.reasons.clone())
            .unwrap_or_default(),
    };
    Some((
        code,
        InvariantFailureDetails {
            checker: Some(checker),
            ..InvariantFailureDetails::default()
        },
    ))
}

/// Position of a measurement-maturity stage, in `MeasurementPlan` `stage` order.
fn stage_index(stage: &str) -> Option<usize> {
    const STAGES: [&str; 7] = [
        "observe",
        "baseline",
        "branch-comparison",
        "trend",
        "ratchet",
        "target",
        "gate",
    ];
    STAGES.iter().position(|candidate| *candidate == stage)
}

/// The mode in force for `stage`: `require` when any recorded policy requires
/// that stage, otherwise `recommend`, including when no policy is recorded.
fn policy_mode(instance: &InvariantProjection, stage: Option<&str>) -> MeasurementPolicyMode {
    let required = stage.is_some_and(|stage| {
        instance.items.measurement_policy.iter().any(|policy| {
            policy.mode == Some(MeasurementPolicyMode::Require)
                && policy
                    .stages
                    .as_ref()
                    .is_some_and(|stages| stages.iter().any(|listed| listed == stage))
        })
    });
    if required {
        MeasurementPolicyMode::Require
    } else {
        MeasurementPolicyMode::Recommend
    }
}

/// Select the checker result bound to `evidence` and classify it.
///
/// A result is read only when it carries the verdict schema and the
/// evidence's plan id. It matches when its definition version and candidate
/// collection also equal the evidence's. When several results match, the
/// least favourable status wins, the earliest result on a tie.
fn checker_status<'a>(
    instance: &'a InvariantProjection,
    evidence: &PromotionEvidence,
) -> (CheckerStatus, Option<&'a MeasurementVerdict>) {
    let Some(plan_id) = evidence
        .plan_id
        .as_deref()
        .filter(|id| !id.trim().is_empty())
    else {
        return (CheckerStatus::Missing, None);
    };
    let mut for_plan = instance
        .items
        .measurement_verdict
        .iter()
        .filter_map(|item| item.0.as_ref())
        .filter(|item| item.plan_id.as_deref() == Some(plan_id))
        .peekable();
    if for_plan.peek().is_none() {
        return (CheckerStatus::Missing, None);
    }
    let candidate = evidence
        .candidate
        .as_deref()
        .filter(|candidate| !candidate.trim().is_empty());
    for_plan
        .filter(|item| {
            candidate.is_some()
                && item.definition_version == evidence.definition_version
                && item.candidate.as_deref() == candidate
        })
        .map(|item| {
            let status = match item.verdict {
                Some(CheckerVerdict::Accept)
                    if item.order_source.as_deref() == Some(ATTESTED_ORDER_SOURCE) =>
                {
                    CheckerStatus::Accepted
                }
                Some(CheckerVerdict::Accept) => CheckerStatus::OrderUnattested,
                Some(CheckerVerdict::Reject | CheckerVerdict::Inconclusive) | None => {
                    CheckerStatus::NotAccepted
                }
            };
            (status, Some(item))
        })
        .reduce(|worst, next| if next.0 > worst.0 { next } else { worst })
        .unwrap_or((CheckerStatus::Mismatch, None))
}

fn change_impact_ready(instance: &InvariantProjection) -> Option<Failure> {
    let request = instance
        .items
        .change_request
        .iter()
        .find(|item| nonempty(item.interview_id.as_deref()));
    let Some(request) = request else {
        return Some(failure(InvariantFailureCode::ImpactSnapshotIncomplete));
    };
    let exact_revision = request
        .source_revision
        .as_deref()
        .is_some_and(is_lower_hex_revision);
    let snapshot = instance.items.impact_snapshot.iter().find(|item| {
        item.source_revision == request.source_revision
            && item.profile_path == request.profile_path
            && item.baseline_id == request.baseline_id
    });
    let complete = snapshot.is_some_and(|item| {
        item.changed_nodes.is_some()
            && item.impacted_nodes.is_some()
            && item.missing_edges.is_some()
            && item.stale_evidence.is_some()
            && item.suspect_evidence.is_some()
            && item.unknowns.is_some()
    });
    (!(exact_revision
        && nonempty(request.profile_path.as_deref())
        && nonempty(request.baseline_id.as_deref())
        && complete))
        .then_some((
            InvariantFailureCode::ImpactSnapshotIncomplete,
            InvariantFailureDetails::default(),
        ))
}

fn change_snapshot_ready(
    instance: &InvariantProjection,
    evaluated_at: OffsetDateTime,
) -> Option<Failure> {
    let source_revision = instance
        .items
        .change_request
        .iter()
        .find(|item| nonempty(item.interview_id.as_deref()))
        .and_then(|item| item.source_revision.as_deref())
        .filter(|revision| !revision.trim().is_empty());
    let Some(source_revision) = source_revision else {
        return Some(failure(InvariantFailureCode::AssuranceSnapshotInvalid));
    };
    let snapshot = instance
        .items
        .assurance_snapshot
        .iter()
        .find(|item| item.source_revision.as_deref() == Some(source_revision));
    let valid = snapshot.is_some_and(|item| {
        item.discharge_schema.as_deref() == Some("clause-discharge-v1")
            && item.argument_schema.as_deref() == Some("authored-assurance-view-v1")
            && nonempty(item.argument_id.as_deref())
            && item.discharge_digest.as_deref().is_some_and(is_digest)
            && item.argument_digest.as_deref().is_some_and(is_digest)
            && item.binding_open.is_some_and(|value| value >= 0)
            && item
                .applicability_unresolved
                .is_some_and(|value| value >= 0)
            && item.top_claim_status.as_deref().is_some_and(|status| {
                matches!(status, "supported" | "open" | "challenged" | "rejected")
            })
            && item
                .evidence_record_ids
                .as_ref()
                .is_some_and(|ids| !ids.is_empty() && ids.iter().all(|id| is_digest(id)))
            && item
                .verified_at
                .as_deref()
                .and_then(parse_instant)
                .is_some_and(|verified_at| verified_at <= evaluated_at)
    });
    (!valid).then_some((
        InvariantFailureCode::AssuranceSnapshotInvalid,
        InvariantFailureDetails::default(),
    ))
}

fn change_review_ready(instance: &InvariantProjection) -> Option<Failure> {
    let source_revision = instance
        .items
        .change_request
        .iter()
        .find(|item| nonempty(item.interview_id.as_deref()))
        .and_then(|item| item.source_revision.as_deref())
        .filter(|revision| !revision.trim().is_empty());
    let Some(source_revision) = source_revision else {
        return Some(failure(InvariantFailureCode::CodeReviewMissing));
    };
    let valid = instance.items.review_validation.iter().any(|item| {
        item.valid == Some(true)
            && item.artifact_type.as_deref() == Some("SpecReview")
            && item.analysis.as_deref() == Some("code-review")
            && item.source_revision.as_deref() == Some(source_revision)
            && nonempty(item.path.as_deref())
    });
    (!valid).then_some((
        InvariantFailureCode::CodeReviewMissing,
        InvariantFailureDetails::default(),
    ))
}

fn parse_instant(value: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(value, &Rfc3339).ok()
}

fn is_lower_hex_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}
