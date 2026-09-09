// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Read-only verification semantics, compatibility mappings, and fixture projections.
//!
//! This module validates references to externally authoritative records. It does
//! not execute a producer, access a filesystem, or persist evidence.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Protocol discriminator for semantic-reference projections.
pub const SEMANTIC_REFERENCE_PROTOCOL: &str = "engineering-assurance.semantic-reference/v1";
/// Protocol discriminator for interoperability fixtures.
pub const SEMANTIC_FIXTURE_PROTOCOL: &str =
    "engineering-assurance.verification-semantics-fixture/v1";
/// Protocol discriminator for bounded assurance reports.
pub const REPORT_PROJECTION_PROTOCOL: &str = "engineering-assurance.assurance-report-projection/v1";
/// Protocol discriminator for read-only PGM-01 views.
pub const PGM01_MAPPING_PROTOCOL: &str = "engineering-assurance.pgm01-compatibility-view/v1";

const OWNERSHIP_BYTES: &[u8] =
    include_bytes!("../engineering_assurance/contracts/verification-semantics-ownership-v1.json");
const NON_SUCCESS_STATES_BYTES: &[u8] = include_bytes!(
    "../engineering_assurance/fixtures/verification-semantics/non-success-states.json"
);
const CANONICAL_FIXTURE_BYTES: &[u8] = include_bytes!(
    "../engineering_assurance/fixtures/verification-semantics/canonical-references.json"
);

/// Stable failure at the pure semantic boundary.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("{message}")]
pub struct SemanticError {
    code: &'static str,
    message: String,
}

impl SemanticError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_semantic_contract",
            message: message.into(),
        }
    }

    /// Return the stable machine-readable error category.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// Return the stable validation message without parsing formatted output.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// One concept in the shared verification vocabulary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticConcept {
    /// A specification-owned verification definition.
    VerificationDefinition,
    /// A native producer execution.
    VerificationExecution,
    /// A native producer check result.
    CheckResult,
    /// A Quoin-owned retained evidence reference.
    Evidence,
    /// A Quoin-owned retained measurement reference.
    Measurement,
    /// A producer-owned diagnostic reference.
    Diagnostic,
    /// A Quoin-owned report reference.
    Report,
    /// An ix-flow-owned human decision reference.
    HumanDecision,
}

impl SemanticConcept {
    const ALL: [Self; 8] = [
        Self::VerificationDefinition,
        Self::VerificationExecution,
        Self::CheckResult,
        Self::Evidence,
        Self::Measurement,
        Self::Diagnostic,
        Self::Report,
        Self::HumanDecision,
    ];

    const fn authority(self) -> Authority {
        match self {
            Self::VerificationDefinition => Authority::Quire,
            Self::VerificationExecution | Self::CheckResult => Authority::NativeProducer,
            Self::Evidence | Self::Measurement | Self::Report => Authority::Quoin,
            Self::Diagnostic => Authority::OriginatingProducer,
            Self::HumanDecision => Authority::IxFlow,
        }
    }

    const fn requires_producer(self) -> bool {
        matches!(
            self,
            Self::VerificationExecution
                | Self::CheckResult
                | Self::Evidence
                | Self::Measurement
                | Self::Diagnostic
        )
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::VerificationDefinition => "verification_definition",
            Self::VerificationExecution => "verification_execution",
            Self::CheckResult => "check_result",
            Self::Evidence => "evidence",
            Self::Measurement => "measurement",
            Self::Diagnostic => "diagnostic",
            Self::Report => "report",
            Self::HumanDecision => "human_decision",
        }
    }
}

impl std::fmt::Display for SemanticConcept {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Authority that owns one semantic concept.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Authority {
    /// Quire specification authority.
    Quire,
    /// Native producer authority.
    NativeProducer,
    /// Quoin evidence authority.
    Quoin,
    /// Originating producer diagnostic authority.
    OriginatingProducer,
    /// ix-flow human-decision authority.
    IxFlow,
}

impl Authority {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Quire => "quire",
            Self::NativeProducer => "native_producer",
            Self::Quoin => "quoin",
            Self::OriginatingProducer => "originating_producer",
            Self::IxFlow => "ix_flow",
        }
    }
}

impl std::fmt::Display for Authority {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Closed state vocabulary retained by semantic-reference v1.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticState {
    /// Defined but not executed.
    Defined,
    /// Observed output.
    Observed,
    /// Passing result.
    Passed,
    /// Failing result.
    Failed,
    /// Producer error.
    Error,
    /// Deliberately skipped result.
    Skipped,
    /// Required producer unavailable.
    Unavailable,
    /// Applicable but not computed.
    NotComputed,
    /// Outside the selected boundary.
    NotApplicable,
    /// Insufficient information for a result.
    Inconclusive,
    /// Unsupported version or capability.
    Unsupported,
    /// Explicitly rejected input.
    Rejected,
    /// Timed-out operation.
    TimedOut,
    /// Awaiting completion or decision.
    Pending,
    /// Structurally malformed input.
    Malformed,
    /// Stale retained input.
    Stale,
    /// Suspect result.
    Suspect,
    /// Vacuous result.
    Vacuous,
    /// Digest-mismatched input.
    Tampered,
    /// Unreadable input.
    Unreadable,
}

/// Immutable source pointer carried by a semantic reference.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticSource {
    /// Versioned schema identity.
    pub schema_identity: String,
    /// Exact schema version.
    pub schema_version: String,
    /// Stable record identity.
    pub record_id: String,
    /// Optional retained-record digest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_digest: Option<String>,
    /// JSON-pointer-like path into the retained record.
    pub field_path: String,
}

/// Complete immutable producer tuple for producer-derived concepts.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticProducer {
    /// Producer identity.
    pub identity: String,
    /// Immutable producer version.
    pub version: String,
    /// Configuration digest.
    pub configuration_digest: String,
    /// Exact source revision.
    pub source_revision: String,
    /// Non-empty structured execution environment.
    pub environment: Map<String, Value>,
    /// Governing definition version.
    pub definition_version: String,
}

/// Typed links from one semantic concept to its prerequisites.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticLinks {
    /// Verification-definition link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    /// Verification-execution link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<String>,
    /// Check-result link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// Evidence link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    /// External `MeasurementPlan` link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measurement_plan: Option<String>,
    /// Report link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<String>,
    /// Human-decision subject link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_subject: Option<String>,
}

impl SemanticLinks {
    fn entries(&self) -> impl Iterator<Item = (&'static str, &String)> {
        [
            ("definition", self.definition.as_ref()),
            ("execution", self.execution.as_ref()),
            ("result", self.result.as_ref()),
            ("evidence", self.evidence.as_ref()),
            ("measurement_plan", self.measurement_plan.as_ref()),
            ("report", self.report.as_ref()),
            ("decision_subject", self.decision_subject.as_ref()),
        ]
        .into_iter()
        .filter_map(|(name, value)| value.map(|value| (name, value)))
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.entries()
            .find_map(|(candidate, value)| (candidate == name).then_some(value.as_str()))
    }
}

/// One validated, non-persisted semantic reference.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticReference {
    /// Versioned projection discriminator.
    pub projection_type: String,
    /// Referenced concept.
    pub concept: SemanticConcept,
    /// Stable semantic identity.
    pub semantic_id: String,
    /// Authority that owns the referenced record.
    pub authority: Authority,
    /// Immutable source pointer.
    pub source: SemanticSource,
    /// Explicit state.
    pub state: SemanticState,
    /// Producer tuple when the concept is producer-derived.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub producer: Option<SemanticProducer>,
    /// Typed semantic links.
    pub links: SemanticLinks,
}

impl SemanticReference {
    /// Validate protocol, authority, producer provenance, and required links.
    ///
    /// # Errors
    ///
    /// Returns [`SemanticError`] for any malformed or ownership-confused reference.
    pub fn validate(&self) -> Result<(), SemanticError> {
        if self.projection_type != SEMANTIC_REFERENCE_PROTOCOL {
            return Err(SemanticError::new(format!(
                "unsupported semantic-reference protocol {:?}",
                self.projection_type
            )));
        }
        validate_identity(&self.semantic_id, "semantic_id")?;
        if self.authority != self.concept.authority() {
            return Err(SemanticError::new(format!(
                "{} authority must be {}, not {}",
                self.concept,
                self.concept.authority(),
                self.authority
            )));
        }
        validate_source(&self.source)?;
        if self.concept.requires_producer() {
            let producer = self.producer.as_ref().ok_or_else(|| {
                SemanticError::new(format!(
                    "{} requires the complete producer tuple",
                    self.concept
                ))
            })?;
            validate_producer(producer)?;
        } else if let Some(producer) = &self.producer {
            validate_producer(producer)?;
        }
        for required in required_links(self.concept) {
            if self.links.get(required).is_none() {
                return Err(SemanticError::new(format!(
                    "{} is missing link {required}",
                    self.concept
                )));
            }
        }
        for (name, target) in self.links.entries() {
            validate_identity(target, name)?;
            if target == &self.semantic_id {
                return Err(SemanticError::new(
                    "a semantic reference cannot link to itself",
                ));
            }
        }
        Ok(())
    }
}

fn required_links(concept: SemanticConcept) -> &'static [&'static str] {
    match concept {
        SemanticConcept::VerificationDefinition | SemanticConcept::Diagnostic => &[],
        SemanticConcept::VerificationExecution => &["definition"],
        SemanticConcept::CheckResult => &["definition", "execution"],
        SemanticConcept::Evidence => &["result"],
        SemanticConcept::Measurement => &["measurement_plan", "evidence"],
        SemanticConcept::Report => &["evidence"],
        SemanticConcept::HumanDecision => &["decision_subject"],
    }
}

fn internal_target(name: &str) -> Option<SemanticConcept> {
    match name {
        "definition" => Some(SemanticConcept::VerificationDefinition),
        "execution" => Some(SemanticConcept::VerificationExecution),
        "result" => Some(SemanticConcept::CheckResult),
        "evidence" => Some(SemanticConcept::Evidence),
        "report" | "decision_subject" => Some(SemanticConcept::Report),
        _ => None,
    }
}

fn validate_source(source: &SemanticSource) -> Result<(), SemanticError> {
    non_empty(&source.schema_identity, "source schema_identity")?;
    non_empty(&source.schema_version, "source schema_version")?;
    validate_identity(&source.record_id, "source record_id")?;
    if let Some(digest) = &source.record_digest {
        validate_digest(digest, "source record_digest")?;
    }
    if !source.field_path.starts_with('/') {
        return Err(SemanticError::new("source field_path must begin with '/'"));
    }
    Ok(())
}

fn validate_producer(producer: &SemanticProducer) -> Result<(), SemanticError> {
    non_empty(&producer.identity, "producer identity")?;
    non_empty(&producer.version, "producer version")?;
    validate_digest(
        &producer.configuration_digest,
        "producer configuration_digest",
    )?;
    non_empty(&producer.source_revision, "producer source_revision")?;
    if producer.environment.is_empty() {
        return Err(SemanticError::new("producer environment must not be empty"));
    }
    non_empty(&producer.definition_version, "producer definition_version")
}

fn non_empty(value: &str, field: &str) -> Result<(), SemanticError> {
    if value.is_empty() {
        Err(SemanticError::new(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_identity(value: &str, field: &str) -> Result<(), SemanticError> {
    let mut bytes = value.bytes();
    if value.len() > 256
        || !bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        || !bytes.all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
    {
        return Err(SemanticError::new(format!(
            "{field} is not a valid identity"
        )));
    }
    Ok(())
}

fn validate_digest(value: &str, field: &str) -> Result<(), SemanticError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(SemanticError::new(format!(
            "{field} must be a SHA-256 digest"
        )));
    }
    Ok(())
}

/// Validate cross-links in one in-memory semantic bundle.
///
/// # Errors
///
/// Returns [`SemanticError`] for an invalid reference, duplicate identity,
/// missing target, or concept-confused link.
pub fn validate_semantic_bundle(references: &[SemanticReference]) -> Result<(), SemanticError> {
    let mut known = BTreeMap::new();
    for reference in references {
        reference.validate()?;
        if known
            .insert(reference.semantic_id.as_str(), reference.concept)
            .is_some()
        {
            return Err(SemanticError::new("semantic identifiers must be distinct"));
        }
    }
    for reference in references {
        for (relationship, target) in reference.links.entries() {
            let Some(expected) = internal_target(relationship) else {
                continue;
            };
            let actual = known.get(target.as_str()).ok_or_else(|| {
                SemanticError::new(format!(
                    "{} has missing reference {target}",
                    reference.semantic_id
                ))
            })?;
            if *actual != expected {
                return Err(SemanticError::new(format!(
                    "{} link {relationship} must target {expected}, not {actual}",
                    reference.semantic_id
                )));
            }
        }
    }
    Ok(())
}

/// One source-version premise in a semantic interoperability fixture.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceVersionPremise {
    /// Semantic concept.
    pub concept: SemanticConcept,
    /// Versioned schema identity.
    pub schema_identity: String,
    /// Exact schema version.
    pub schema_version: String,
}

/// One complete semantic interoperability fixture.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticFixture {
    /// Fixture protocol discriminator.
    pub fixture_version: String,
    /// Exact source versions expected by the fixture.
    pub source_version_premises: Vec<SourceVersionPremise>,
    /// Cross-linked semantic references.
    pub references: Vec<SemanticReference>,
}

impl SemanticFixture {
    /// Validate fixture protocol, references, and exact source-version premises.
    ///
    /// # Errors
    ///
    /// Returns [`SemanticError`] if the fixture is incomplete or internally inconsistent.
    pub fn validate(&self) -> Result<(), SemanticError> {
        if self.fixture_version != SEMANTIC_FIXTURE_PROTOCOL {
            return Err(SemanticError::new(format!(
                "unsupported semantic fixture protocol {:?}",
                self.fixture_version
            )));
        }
        if self.source_version_premises.is_empty() || self.references.is_empty() {
            return Err(SemanticError::new(
                "semantic fixture premises and references must not be empty",
            ));
        }
        validate_semantic_bundle(&self.references)?;
        let premises: BTreeSet<_> = self.source_version_premises.iter().cloned().collect();
        if premises.len() != self.source_version_premises.len() {
            return Err(SemanticError::new(
                "source-version premises must be distinct",
            ));
        }
        let observed: BTreeSet<_> = self
            .references
            .iter()
            .map(|reference| SourceVersionPremise {
                concept: reference.concept,
                schema_identity: reference.source.schema_identity.clone(),
                schema_version: reference.source.schema_version.clone(),
            })
            .collect();
        if observed != premises {
            return Err(SemanticError::new("source-version premises differ"));
        }
        Ok(())
    }
}

/// Parse and validate encoded semantic-fixture JSON.
///
/// # Errors
///
/// Returns [`SemanticError`] for malformed JSON or any semantic validation failure.
pub fn validate_semantic_fixture_bytes(raw: &[u8]) -> Result<SemanticFixture, SemanticError> {
    let fixture: SemanticFixture = serde_json::from_slice(raw)
        .map_err(|error| SemanticError::new(format!("invalid semantic fixture: {error}")))?;
    fixture.validate()?;
    Ok(fixture)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnershipRegistry {
    schema_version: String,
    purpose: String,
    non_executing: bool,
    concepts: Vec<OwnershipConcept>,
    result_states: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnershipConcept {
    concept: SemanticConcept,
    authority: Authority,
    authoritative_types: Vec<String>,
    link_direction: String,
    responsibility: String,
}

/// Validate the embedded semantic-ownership registry.
///
/// # Errors
///
/// Returns [`SemanticError`] if its version, population, authority allocation,
/// or non-executing boundary differs from the reviewed contract.
pub fn validate_embedded_ownership_registry() -> Result<(), SemanticError> {
    validate_ownership_registry_bytes(OWNERSHIP_BYTES)
}

/// Validate an encoded semantic-ownership registry without loading or persisting it.
///
/// # Errors
///
/// Returns [`SemanticError`] if its version, population, authority allocation,
/// metadata, or non-executing boundary differs from the reviewed contract.
pub fn validate_ownership_registry_bytes(raw: &[u8]) -> Result<(), SemanticError> {
    let registry: OwnershipRegistry = serde_json::from_slice(raw)
        .map_err(|error| SemanticError::new(format!("invalid ownership registry: {error}")))?;
    if registry.schema_version != "engineering-assurance.verification-semantics-ownership/v1"
        || registry.purpose != "semantic_ownership_registry"
        || !registry.non_executing
    {
        return Err(SemanticError::new(
            "ownership registry version or non-executing boundary is invalid",
        ));
    }
    let mut concepts = BTreeSet::new();
    for item in &registry.concepts {
        if item.authority != item.concept.authority() {
            return Err(SemanticError::new(format!(
                "{} authority must be {}, not {}",
                item.concept,
                item.concept.authority(),
                item.authority
            )));
        }
        let authoritative_types: BTreeSet<_> = item.authoritative_types.iter().collect();
        if item.authoritative_types.is_empty()
            || authoritative_types.len() != item.authoritative_types.len()
            || item.authoritative_types.iter().any(String::is_empty)
            || item.link_direction.is_empty()
            || item.responsibility.is_empty()
        {
            return Err(SemanticError::new(
                "ownership registry concept metadata is incomplete",
            ));
        }
        if !concepts.insert(item.concept) {
            return Err(SemanticError::new(
                "ownership registry repeats a semantic concept",
            ));
        }
    }
    if concepts != SemanticConcept::ALL.into_iter().collect() {
        return Err(SemanticError::new(
            "ownership registry has an incomplete concept set",
        ));
    }
    let states: BTreeSet<_> = registry.result_states.iter().collect();
    if states.is_empty()
        || states.len() != registry.result_states.len()
        || registry
            .result_states
            .iter()
            .any(|state| !valid_state_name(state))
    {
        return Err(SemanticError::new(
            "ownership registry result states must be non-empty, distinct snake-case names",
        ));
    }
    Ok(())
}

fn valid_state_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

/// One bounded report claim.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportClaim {
    /// Stable claim identity.
    pub claim_id: String,
    /// Human-readable claim statement.
    pub statement: String,
    /// Closed claim status.
    pub status: ReportClaimStatus,
}

/// Closed bounded-report claim status.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportClaimStatus {
    /// Claim is open.
    Open,
    /// Available evidence supports the claim.
    Supported,
    /// Available counterevidence challenges the claim.
    Challenged,
}

impl std::fmt::Display for ReportClaimStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Open => "open",
            Self::Supported => "supported",
            Self::Challenged => "challenged",
        })
    }
}

/// One bounded report relationship.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportReference {
    /// Semantic reference identity.
    pub semantic_ref: String,
    /// Closed relationship kind.
    pub relation: ReportRelation,
}

/// Closed bounded-report relationship kind.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportRelation {
    /// Reference supports the claim set.
    Supports,
    /// Reference challenges the claim set.
    Challenges,
    /// Reference supplies context without deciding a claim.
    Context,
}

impl std::fmt::Display for ReportRelation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Supports => "supports",
            Self::Challenges => "challenges",
            Self::Context => "context",
        })
    }
}

/// One explicit assurance gap.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportGap {
    /// Stable gap identity.
    pub gap_id: String,
    /// Human-readable gap summary.
    pub summary: String,
    /// Named gap owner.
    pub owner: String,
    /// Required next action.
    pub action: String,
}

/// Read-only bounded report projection with no aggregate verdict field.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportProjection {
    /// Projection protocol discriminator.
    pub projection_type: String,
    /// Stable report identity.
    pub report_id: String,
    /// Subject being reviewed.
    pub subject: String,
    /// Explicit claims.
    pub claims: Vec<ReportClaim>,
    /// Supporting evidence references.
    pub evidence: Vec<ReportReference>,
    /// Challenging evidence references.
    pub counterevidence: Vec<ReportReference>,
    /// Explicit gaps.
    pub gaps: Vec<ReportGap>,
    /// Named report owner.
    pub owner: String,
    /// Explicit next actions.
    pub actions: Vec<String>,
    /// Optional externally authoritative human-decision reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_ref: Option<String>,
}

impl ReportProjection {
    /// Validate the bounded report vocabulary without inferring a verdict.
    ///
    /// # Errors
    ///
    /// Returns [`SemanticError`] for an unsupported protocol or malformed field.
    pub fn validate(&self) -> Result<(), SemanticError> {
        if self.projection_type != REPORT_PROJECTION_PROTOCOL {
            return Err(SemanticError::new(format!(
                "unsupported report projection protocol {:?}",
                self.projection_type
            )));
        }
        validate_identity(&self.report_id, "report_id")?;
        non_empty(&self.subject, "report subject")?;
        non_empty(&self.owner, "report owner")?;
        for claim in &self.claims {
            validate_identity(&claim.claim_id, "claim_id")?;
            non_empty(&claim.statement, "claim statement")?;
        }
        for reference in self.evidence.iter().chain(&self.counterevidence) {
            validate_identity(&reference.semantic_ref, "semantic_ref")?;
        }
        for gap in &self.gaps {
            validate_identity(&gap.gap_id, "gap_id")?;
            non_empty(&gap.summary, "gap summary")?;
            non_empty(&gap.owner, "gap owner")?;
            non_empty(&gap.action, "gap action")?;
        }
        for action in &self.actions {
            non_empty(action, "report action")?;
        }
        if let Some(reference) = &self.decision_ref {
            validate_identity(reference, "decision_ref")?;
        }
        Ok(())
    }

    /// Render canonical compact JSON followed by one newline.
    ///
    /// # Errors
    ///
    /// Returns [`SemanticError`] for invalid input or serialization failure.
    pub fn render_json(&self) -> Result<String, SemanticError> {
        self.validate()?;
        let value = serde_json::to_value(self)
            .map_err(|error| SemanticError::new(format!("report serialization failed: {error}")))?;
        let mut encoded = serde_json::to_string(&value)
            .map_err(|error| SemanticError::new(format!("report serialization failed: {error}")))?;
        encoded.push('\n');
        Ok(encoded)
    }

    /// Render the retained bounded Markdown view.
    ///
    /// # Errors
    ///
    /// Returns [`SemanticError`] for invalid input.
    pub fn render_markdown(&self) -> Result<String, SemanticError> {
        self.validate()?;
        let mut lines = vec![
            format!("# Assurance report: {}", markdown_text(&self.subject)),
            String::new(),
            "## Claims".to_owned(),
            String::new(),
        ];
        if self.claims.is_empty() {
            lines.push("- No claims declared.".to_owned());
        } else {
            lines.extend(self.claims.iter().map(|item| {
                format!(
                    "- `{}` [{}]: {}",
                    item.claim_id,
                    item.status,
                    markdown_text(&item.statement)
                )
            }));
        }
        for (heading, entries, empty) in [
            ("Evidence", &self.evidence, "- No evidence declared."),
            (
                "Counterevidence",
                &self.counterevidence,
                "- No counterevidence declared.",
            ),
        ] {
            lines.extend([String::new(), format!("## {heading}"), String::new()]);
            if entries.is_empty() {
                lines.push(empty.to_owned());
            } else {
                lines.extend(
                    entries
                        .iter()
                        .map(|entry| format!("- `{}` ({})", entry.semantic_ref, entry.relation)),
                );
            }
        }
        lines.extend([String::new(), "## Gaps".to_owned(), String::new()]);
        if self.gaps.is_empty() {
            lines.push("- No gaps declared.".to_owned());
        } else {
            lines.extend(self.gaps.iter().map(|gap| {
                format!(
                    "- `{}`: {} (owner: {}; action: {})",
                    gap.gap_id,
                    markdown_text(&gap.summary),
                    markdown_text(&gap.owner),
                    markdown_text(&gap.action)
                )
            }));
        }
        lines.extend([
            String::new(),
            "## Owner".to_owned(),
            String::new(),
            markdown_text(&self.owner),
            String::new(),
            "## Actions".to_owned(),
            String::new(),
        ]);
        if self.actions.is_empty() {
            lines.push("- No actions declared.".to_owned());
        } else {
            lines.extend(
                self.actions
                    .iter()
                    .map(|action| format!("- {}", markdown_text(action))),
            );
        }
        lines.extend([
            String::new(),
            "## Human decision reference".to_owned(),
            String::new(),
            markdown_text(
                self.decision_ref
                    .as_deref()
                    .unwrap_or("No decision recorded."),
            ),
            String::new(),
        ]);
        Ok(lines.join("\n"))
    }
}

/// Parse and validate an encoded bounded report.
///
/// # Errors
///
/// Returns [`SemanticError`] for malformed JSON or invalid report fields.
pub fn validate_report_bytes(raw: &[u8]) -> Result<ReportProjection, SemanticError> {
    let report: ReportProjection = serde_json::from_slice(raw)
        .map_err(|error| SemanticError::new(format!("invalid report projection: {error}")))?;
    report.validate()?;
    Ok(report)
}

fn markdown_text(value: &str) -> String {
    value
        .lines()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "\\|")
}

/// One traceable field mapping from a historical PGM-01 record.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pgm01Mapping {
    /// Source JSON path.
    pub source_path: String,
    /// Target semantic concept.
    pub target_concept: SemanticConcept,
    /// Target field within that concept.
    pub target_field: String,
    /// Preserved source value.
    pub value: Value,
}

/// One deliberately unmapped historical field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pgm01Unmapped {
    /// Source JSON path.
    pub source_path: String,
    /// Reason the field cannot be mapped without inventing semantics.
    pub reason: String,
}

/// Classification of a read-only PGM-01 mapping.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Pgm01Outcome {
    /// Fully compatible mapping.
    Compatible,
    /// Readable mapping with explicit limitations.
    Lossy,
    /// Unsupported or identity-mismatched input.
    Incompatible,
    /// Malformed or unreadable input.
    Unreadable,
}

/// Read-only compatibility view over immutable PGM-01 bytes.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pgm01View {
    /// Mapping protocol discriminator.
    pub mapping_version: String,
    /// Source schema version or explicit unknown value.
    pub source_schema_version: String,
    /// Source record identity or explicit placeholder.
    pub source_record_id: String,
    /// SHA-256 digest of the exact source bytes.
    pub source_digest: String,
    /// Mapping outcome.
    pub outcome: Pgm01Outcome,
    /// Preserved field mappings.
    pub mappings: Vec<Pgm01Mapping>,
    /// Explicitly unmapped fields.
    pub unmapped_fields: Vec<Pgm01Unmapped>,
    /// Declared compatibility limitations.
    pub limitations: Vec<String>,
}

impl Pgm01View {
    fn base(raw: &[u8], schema_version: impl Into<String>, record_id: impl Into<String>) -> Self {
        Self {
            mapping_version: PGM01_MAPPING_PROTOCOL.to_owned(),
            source_schema_version: schema_version.into(),
            source_record_id: record_id.into(),
            source_digest: sha256_hex(raw),
            outcome: Pgm01Outcome::Lossy,
            mappings: Vec::new(),
            unmapped_fields: Vec::new(),
            limitations: Vec::new(),
        }
    }
}

fn mapping(
    path: impl Into<String>,
    concept: SemanticConcept,
    field: &str,
    value: Value,
) -> Pgm01Mapping {
    Pgm01Mapping {
        source_path: path.into(),
        target_concept: concept,
        target_field: field.to_owned(),
        value,
    }
}

fn unmapped(path: &str, reason: impl Into<String>) -> Pgm01Unmapped {
    Pgm01Unmapped {
        source_path: path.to_owned(),
        reason: reason.into(),
    }
}

fn required_object<'a>(
    record: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a Map<String, Value>, SemanticError> {
    record
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| SemanticError::new(format!("legacy field /{key} must be an object")))
}

fn required_array<'a>(
    record: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a [Value], SemanticError> {
    record
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| SemanticError::new(format!("legacy field /{key} must be an array")))
}

fn required_string<'a>(
    record: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a str, SemanticError> {
    record
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| SemanticError::new(format!("legacy field /{key} must be a string")))
}

fn required_digest<'a>(
    record: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a str, SemanticError> {
    let value = required_string(record, key)?;
    validate_digest(value, &format!("legacy field /{key}"))?;
    Ok(value)
}

fn required_non_negative_integer(
    record: &Map<String, Value>,
    key: &str,
) -> Result<Value, SemanticError> {
    let token = record
        .get(key)
        .and_then(Value::as_number)
        .map(ToString::to_string);
    match token.as_deref() {
        Some("-0") => Ok(Value::from(0)),
        Some(token) if token.bytes().all(|byte| byte.is_ascii_digit()) => Ok(record[key].clone()),
        _ => Err(SemanticError::new(format!(
            "legacy field /{key} must be a non-negative integer"
        ))),
    }
}

fn state_mapping(value: &str) -> Option<&'static str> {
    match value {
        "pass" | "passed" => Some("passed"),
        "fail" | "failed" => Some("failed"),
        "error" => Some("error"),
        "skipped" => Some("skipped"),
        "inconclusive" => Some("inconclusive"),
        "unavailable" => Some("unavailable"),
        _ => None,
    }
}

fn map_pgm01_v1(raw: &[u8], record: &Map<String, Value>) -> Result<Pgm01View, SemanticError> {
    let record_id = required_string(record, "recordId")?;
    let mut view = Pgm01View::base(raw, "quire.pgm01-evidence/v1", record_id);
    let collector = required_object(record, "collector")?;
    let environment = required_object(record, "environment")?;
    let checks = required_array(record, "checks")?;
    let outputs = required_array(record, "outputs")?;
    let limitations = required_array(record, "limitations")?;
    view.mappings.extend([
        mapping(
            "/recordId",
            SemanticConcept::Report,
            "legacy_record_id",
            Value::String(record_id.to_owned()),
        ),
        mapping(
            "/subjectRevision",
            SemanticConcept::VerificationExecution,
            "source_revision",
            Value::String(required_string(record, "subjectRevision")?.to_owned()),
        ),
        mapping(
            "/repository",
            SemanticConcept::VerificationExecution,
            "repository",
            Value::String(required_string(record, "repository")?.to_owned()),
        ),
        mapping(
            "/collector/implementation",
            SemanticConcept::VerificationExecution,
            "producer.identity",
            Value::String(required_string(collector, "implementation")?.to_owned()),
        ),
        mapping(
            "/collector/implementationRevision",
            SemanticConcept::VerificationExecution,
            "producer.source_revision",
            Value::String(required_string(collector, "implementationRevision")?.to_owned()),
        ),
        mapping(
            "/environment",
            SemanticConcept::VerificationExecution,
            "producer.environment",
            Value::Object(environment.clone()),
        ),
    ]);
    for (index, check) in checks.iter().enumerate() {
        let check = check.as_object().ok_or_else(|| {
            SemanticError::new(format!("legacy field /checks/{index} must be an object"))
        })?;
        let status = check
            .get("status")
            .and_then(Value::as_str)
            .and_then(state_mapping)
            .ok_or_else(|| {
                SemanticError::new(format!("legacy field /checks/{index}/status is unknown"))
            })?;
        view.mappings.push(mapping(
            format!("/checks/{index}/status"),
            SemanticConcept::CheckResult,
            "state",
            Value::String(status.to_owned()),
        ));
    }
    for (index, output) in outputs.iter().enumerate() {
        let output = output
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                SemanticError::new(format!("legacy field /outputs/{index} must be a string"))
            })?;
        view.mappings.push(mapping(
            format!("/outputs/{index}"),
            SemanticConcept::Evidence,
            "retained_output.path",
            Value::String(output.to_owned()),
        ));
    }
    for (index, limitation) in limitations.iter().enumerate() {
        let limitation = limitation
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                SemanticError::new(format!(
                    "legacy field /limitations/{index} must be a string"
                ))
            })?;
        view.mappings.push(mapping(
            format!("/limitations/{index}"),
            SemanticConcept::Report,
            "limitation",
            Value::String(limitation.to_owned()),
        ));
    }
    finish_pgm01_v1(&mut view);
    Ok(view)
}

fn finish_pgm01_v1(view: &mut Pgm01View) {
    view.unmapped_fields.extend([
        unmapped(
            "/collector/version",
            "PGM-01 v1 did not record a distinct producer version",
        ),
        unmapped(
            "/configurationDigest",
            "PGM-01 v1 did not record a configuration digest",
        ),
        unmapped(
            "/definitionVersion",
            "PGM-01 v1 did not record a governing definition version",
        ),
        unmapped(
            "/outputs/*/digest",
            "output digests live in a separate checksum file, not this manifest",
        ),
        unmapped(
            "/decision",
            "the legacy merge-readiness label is not an ix-flow human decision event",
        ),
    ]);
    view.limitations.push(
        "PGM-01 v1 is readable only as a lossy view; missing identities remain missing.".to_owned(),
    );
}

fn map_pgm01_v2(raw: &[u8], record: &Map<String, Value>) -> Result<Pgm01View, SemanticError> {
    let record_id = required_string(record, "recordId")?;
    let mut view = Pgm01View::base(raw, "quire.pgm01-evidence/v2", record_id);
    let collector = required_object(record, "collector")?;
    let parameters = required_object(record, "parameters")?;
    let profile = required_object(record, "profile")?;
    let commands = required_array(record, "commands")?;
    let limitations = required_array(record, "limitations")?;
    view.mappings.extend([
        mapping(
            "/recordId",
            SemanticConcept::Report,
            "legacy_record_id",
            Value::String(record_id.to_owned()),
        ),
        mapping(
            "/subjectRevision",
            SemanticConcept::VerificationExecution,
            "source_revision",
            Value::String(required_string(record, "subjectRevision")?.to_owned()),
        ),
        mapping(
            "/repository",
            SemanticConcept::VerificationExecution,
            "repository",
            Value::String(required_string(record, "repository")?.to_owned()),
        ),
        mapping(
            "/collector/id",
            SemanticConcept::VerificationExecution,
            "producer.identity",
            Value::String(required_string(collector, "id")?.to_owned()),
        ),
        mapping(
            "/collector/version",
            SemanticConcept::VerificationExecution,
            "producer.version",
            Value::String(required_string(collector, "version")?.to_owned()),
        ),
        mapping(
            "/collector/sha256",
            SemanticConcept::VerificationExecution,
            "producer.executable_digest",
            Value::String(required_digest(collector, "sha256")?.to_owned()),
        ),
        mapping(
            "/parameters/sha256",
            SemanticConcept::VerificationExecution,
            "producer.configuration_digest",
            Value::String(required_digest(parameters, "sha256")?.to_owned()),
        ),
        mapping(
            "/profile/sha256",
            SemanticConcept::VerificationDefinition,
            "definition_version",
            Value::String(required_digest(profile, "sha256")?.to_owned()),
        ),
    ]);
    let overall = required_string(record, "overallStatus")?;
    let overall = state_mapping(overall)
        .ok_or_else(|| SemanticError::new("legacy field /overallStatus is unknown"))?;
    view.mappings.push(mapping(
        "/overallStatus",
        SemanticConcept::CheckResult,
        "state",
        Value::String(overall.to_owned()),
    ));
    map_pgm01_v2_commands(&mut view, commands)?;
    for (index, limitation) in limitations.iter().enumerate() {
        let limitation = limitation
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                SemanticError::new(format!(
                    "legacy field /limitations/{index} must be a string"
                ))
            })?;
        view.mappings.push(mapping(
            format!("/limitations/{index}"),
            SemanticConcept::Report,
            "limitation",
            Value::String(limitation.to_owned()),
        ));
    }
    match required_string(record, "historicalDisposition")? {
        "active" => {}
        "retracted" => view.mappings.push(mapping(
            "/historicalDisposition",
            SemanticConcept::Evidence,
            "state",
            Value::String("stale".to_owned()),
        )),
        _ => {
            return Err(SemanticError::new(
                "legacy historicalDisposition is unknown",
            ));
        }
    }
    finish_pgm01_v2(&mut view);
    Ok(view)
}

fn map_pgm01_v2_commands(view: &mut Pgm01View, commands: &[Value]) -> Result<(), SemanticError> {
    for (index, command) in commands.iter().enumerate() {
        let command = command.as_object().ok_or_else(|| {
            SemanticError::new(format!("legacy field /commands/{index} must be an object"))
        })?;
        let status = command
            .get("status")
            .and_then(Value::as_str)
            .and_then(state_mapping)
            .ok_or_else(|| {
                SemanticError::new(format!("legacy field /commands/{index}/status is unknown"))
            })?;
        view.mappings.push(mapping(
            format!("/commands/{index}/status"),
            SemanticConcept::CheckResult,
            "state",
            Value::String(status.to_owned()),
        ));
        for stream in ["stdout", "stderr"] {
            let retained = command
                .get(stream)
                .and_then(Value::as_object)
                .ok_or_else(|| {
                    SemanticError::new(format!(
                        "legacy field /commands/{index}/{stream} must be an object"
                    ))
                })?;
            for (field, value) in [
                (
                    "path",
                    Value::String(required_string(retained, "path")?.to_owned()),
                ),
                (
                    "sha256",
                    Value::String(required_digest(retained, "sha256")?.to_owned()),
                ),
                ("bytes", required_non_negative_integer(retained, "bytes")?),
            ] {
                view.mappings.push(mapping(
                    format!("/commands/{index}/{stream}/{field}"),
                    SemanticConcept::Evidence,
                    &format!("retained_output.{field}"),
                    value,
                ));
            }
        }
    }
    Ok(())
}

fn finish_pgm01_v2(view: &mut Pgm01View) {
    view.unmapped_fields.extend([
        unmapped(
            "/environment",
            "PGM-01 v2 did not record a complete execution environment",
        ),
        unmapped(
            "/commands/*/corroboration",
            "generic transcript corroboration is not imported as a verdict",
        ),
        unmapped(
            "/quoin/status",
            "legacy intake status is not evidence sufficiency or a check result",
        ),
    ]);
    view.limitations.push("PGM-01 v2 is readable only as a lossy view; corroboration and intake status do not establish success.".to_owned());
}

/// Map immutable PGM-01 v1/v2 bytes without writing or synthesizing fields.
///
/// # Errors
///
/// Returns [`SemanticError`] only when the caller supplies an invalid expected
/// digest. Malformed, unknown, and tampered source bytes receive explicit views.
pub fn map_pgm01_bytes(
    raw: &[u8],
    expected_digest: Option<&str>,
) -> Result<Pgm01View, SemanticError> {
    let digest = sha256_hex(raw);
    if let Some(expected) = expected_digest {
        if validate_digest(expected, "expected digest").is_err() {
            return Err(SemanticError::new(
                "expected digest must be a SHA-256 digest",
            ));
        }
        if digest != expected {
            let mut view = Pgm01View::base(raw, "unknown", "tampered-source");
            view.outcome = Pgm01Outcome::Incompatible;
            view.unmapped_fields.push(unmapped(
                "/",
                "tampered source digest differs from expected identity",
            ));
            view.limitations
                .push("No field from the altered source was interpreted.".to_owned());
            return Ok(view);
        }
    }
    let decoded: Value = match serde_json::from_slice(raw) {
        Ok(decoded) => decoded,
        Err(error) => {
            let mut view = Pgm01View::base(raw, "unknown", "unreadable-source");
            view.outcome = Pgm01Outcome::Unreadable;
            view.unmapped_fields
                .push(unmapped("/", format!("invalid JSON: {error}")));
            view.limitations
                .push("No legacy field was interpreted.".to_owned());
            return Ok(view);
        }
    };
    let Some(record) = decoded.as_object() else {
        let mut view = Pgm01View::base(raw, "unknown", "unreadable-source");
        view.outcome = Pgm01Outcome::Unreadable;
        view.unmapped_fields
            .push(unmapped("/", "legacy record must be a JSON object"));
        view.limitations
            .push("No legacy field was interpreted.".to_owned());
        return Ok(view);
    };
    let schema_version = record.get("schemaVersion").and_then(Value::as_str);
    if !matches!(
        schema_version,
        Some("quire.pgm01-evidence/v1" | "quire.pgm01-evidence/v2")
    ) {
        let source_schema = schema_version.unwrap_or("unknown");
        let source_id = record
            .get("recordId")
            .and_then(Value::as_str)
            .unwrap_or("incompatible-source");
        let mut view = Pgm01View::base(raw, source_schema, source_id);
        view.outcome = Pgm01Outcome::Incompatible;
        view.unmapped_fields
            .push(unmapped("/schemaVersion", "unknown PGM-01 schema version"));
        view.limitations
            .push("No unknown schema was treated as empty or current.".to_owned());
        return Ok(view);
    }
    let mapped = if schema_version == Some("quire.pgm01-evidence/v1") {
        map_pgm01_v1(raw, record)
    } else {
        map_pgm01_v2(raw, record)
    };
    Ok(match mapped {
        Ok(view) => view,
        Err(error) => {
            let source_id = record
                .get("recordId")
                .and_then(Value::as_str)
                .unwrap_or("unreadable-source");
            let mut view = Pgm01View::base(raw, schema_version.unwrap_or("unknown"), source_id);
            view.outcome = Pgm01Outcome::Unreadable;
            view.unmapped_fields.push(unmapped("/", error.to_string()));
            view.limitations
                .push("The malformed legacy record was not accepted.".to_owned());
            view
        }
    })
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

#[derive(Debug, Deserialize)]
struct CompatibilityCorpus {
    cases: Vec<CompatibilityCase>,
}

#[derive(Debug, Deserialize)]
struct CompatibilityCase {
    id: String,
    kind: String,
    family: String,
    retained_path: String,
    retained_sha256: String,
    derivation: Option<Value>,
    expected: CompatibilityExpected,
}

#[derive(Debug, Deserialize)]
struct CompatibilityExpected {
    outcome: String,
}

/// Render all inert cross-language fixture samples from canonical semantic data.
///
/// `compatibility_corpus_bytes` are supplied explicitly so the reusable
/// library does not discover a workstation checkout or embed qualification
/// corpus bytes in its production artifact.
///
/// # Errors
///
/// Returns [`SemanticError`] when a canonical semantic source is malformed.
pub fn render_generated_fixtures(
    compatibility_corpus_bytes: &[u8],
) -> Result<BTreeMap<String, String>, SemanticError> {
    let states: Vec<String> =
        serde_json::from_slice(NON_SUCCESS_STATES_BYTES).map_err(|error| {
            SemanticError::new(format!("invalid non-success-state source: {error}"))
        })?;
    if states.is_empty() || states.iter().any(String::is_empty) {
        return Err(SemanticError::new(
            "non-success-state source must contain non-empty states",
        ));
    }
    let canonical: Value = serde_json::from_slice(CANONICAL_FIXTURE_BYTES).map_err(|error| {
        SemanticError::new(format!("invalid canonical fixture source: {error}"))
    })?;
    validate_semantic_fixture_bytes(CANONICAL_FIXTURE_BYTES)?;
    let canonical_json = serde_json::to_string(&canonical).map_err(|error| {
        SemanticError::new(format!("canonical fixture serialization failed: {error}"))
    })?;
    let quoted_canonical = serde_json::to_string(&canonical_json).map_err(|error| {
        SemanticError::new(format!("canonical fixture quoting failed: {error}"))
    })?;

    let corpus: CompatibilityCorpus = serde_json::from_slice(compatibility_corpus_bytes)
        .map_err(|error| SemanticError::new(format!("invalid compatibility corpus: {error}")))?;
    let cases: Vec<Value> = corpus
        .cases
        .into_iter()
        .map(|case| {
            serde_json::json!({
                "constructed": case.derivation.is_some(),
                "expected_outcome": case.expected.outcome,
                "family": case.family,
                "id": case.id,
                "kind": case.kind,
                "retained_path": case.retained_path,
                "retained_sha256": case.retained_sha256,
            })
        })
        .collect();
    let cases_json = serde_json::to_string(&cases).map_err(|error| {
        SemanticError::new(format!(
            "compatibility fixture serialization failed: {error}"
        ))
    })?;
    let quoted_cases = serde_json::to_string(&cases_json).map_err(|error| {
        SemanticError::new(format!("compatibility fixture quoting failed: {error}"))
    })?;

    let mut generated = BTreeMap::new();
    insert_state_fixtures(&mut generated, &states);
    insert_canonical_fixtures(&mut generated, &canonical_json, &quoted_canonical);
    insert_compatibility_fixtures(&mut generated, &cases_json, &quoted_cases);
    Ok(generated)
}

fn insert_state_fixtures(generated: &mut BTreeMap<String, String>, states: &[String]) {
    generated.insert(
        "non_success_states.py".to_owned(),
        format!(
            "\"\"\"Generated fixture; semantic source is non-success-states.json.\"\"\"\n\nNON_SUCCESS_STATES = (\n{}\n)\n",
            states.iter().map(|state| format!("    \"{state}\",")).collect::<Vec<_>>().join("\n")
        ),
    );
    generated.insert(
        "non_success_states.ts".to_owned(),
        format!(
            "// Generated fixture; semantic source is non-success-states.json.\nexport const NON_SUCCESS_STATES = [\n{}\n] as const;\n",
            states.iter().map(|state| format!("  \"{state}\",")).collect::<Vec<_>>().join("\n")
        ),
    );
    generated.insert(
        "non_success_states.rs".to_owned(),
        format!(
            "// Generated fixture; semantic source is non-success-states.json.\npub const NON_SUCCESS_STATES: &[&str] = &[\n{}\n];\n",
            states.iter().map(|state| format!("    \"{state}\",")).collect::<Vec<_>>().join("\n")
        ),
    );
}

fn insert_canonical_fixtures(
    generated: &mut BTreeMap<String, String>,
    canonical_json: &str,
    quoted_canonical: &str,
) {
    for (name, prefix, suffix) in [
        (
            "canonical_references.py",
            "\"\"\"Generated fixture; semantic source is canonical-references.json.\"\"\"\nCANONICAL_FIXTURE_JSON = ",
            "\n",
        ),
        (
            "canonical_references.ts",
            "// Generated fixture; semantic source is canonical-references.json.\nexport const CANONICAL_FIXTURE_JSON = ",
            ";\n",
        ),
    ] {
        generated.insert(
            name.to_owned(),
            format!("{prefix}{quoted_canonical}{suffix}"),
        );
    }
    generated.insert(
        "canonical_references.rs".to_owned(),
        format!("// Generated fixture; semantic source is canonical-references.json.\npub const CANONICAL_FIXTURE_JSON: &str = r#\"{canonical_json}\"#;\n"),
    );
}

fn insert_compatibility_fixtures(
    generated: &mut BTreeMap<String, String>,
    cases_json: &str,
    quoted_cases: &str,
) {
    for (name, prefix, suffix) in [
        (
            "compatibility_cases.py",
            "\"\"\"Generated fixture; semantic source is compatibility-corpus/corpus.json.\"\"\"\nCOMPATIBILITY_CASES_JSON = ",
            "\n",
        ),
        (
            "compatibility_cases.ts",
            "// Generated fixture; semantic source is compatibility-corpus/corpus.json.\nexport const COMPATIBILITY_CASES_JSON = ",
            ";\n",
        ),
    ] {
        generated.insert(name.to_owned(), format!("{prefix}{quoted_cases}{suffix}"));
    }
    generated.insert(
        "compatibility_cases.rs".to_owned(),
        format!("// Generated fixture; semantic source is compatibility-corpus/corpus.json.\npub const COMPATIBILITY_CASES_JSON: &str = r#\"{cases_json}\"#;\n"),
    );
}
