// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Read-only verification semantics, compatibility mappings, and fixture projections.
//!
//! This module validates references to externally authoritative records. It does
//! not execute a producer, access a filesystem, or persist evidence.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

mod fixtures;
mod pgm01;
mod report;

pub use fixtures::render_generated_fixtures;
pub use pgm01::{Pgm01Mapping, Pgm01Outcome, Pgm01Unmapped, Pgm01View, map_pgm01_bytes};
pub use report::{
    ReportClaim, ReportClaimStatus, ReportGap, ReportProjection, ReportReference, ReportRelation,
    validate_report_bytes,
};

/// Protocol discriminator for semantic-reference projections.
pub const SEMANTIC_REFERENCE_PROTOCOL: &str = "engineering-assurance.semantic-reference/v1";
/// Protocol discriminator for interoperability fixtures.
pub const SEMANTIC_FIXTURE_PROTOCOL: &str =
    "engineering-assurance.verification-semantics-fixture/v1";
/// Protocol discriminator for bounded assurance reports.
pub const REPORT_PROJECTION_PROTOCOL: &str = "engineering-assurance.assurance-report-projection/v1";
/// Protocol discriminator for read-only PGM-01 views.
pub const PGM01_MAPPING_PROTOCOL: &str = "engineering-assurance.pgm01-compatibility-view/v1";

const OWNERSHIP_BYTES: &[u8] = include_bytes!(
    "../../engineering_assurance/contracts/verification-semantics-ownership-v1.json"
);
const NON_SUCCESS_STATES_BYTES: &[u8] = include_bytes!(
    "../../engineering_assurance/fixtures/verification-semantics/non-success-states.json"
);
const CANONICAL_FIXTURE_BYTES: &[u8] = include_bytes!(
    "../../engineering_assurance/fixtures/verification-semantics/canonical-references.json"
);

/// Stable machine-readable reason for a semantic refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticErrorKind {
    /// A protocol discriminator is unsupported.
    UnsupportedProtocol,
    /// An identifier violates the bounded identifier grammar.
    InvalidIdentity,
    /// A concept is assigned to the wrong authority.
    AuthorityMismatch,
    /// A producer-owned concept lacks its producer tuple.
    MissingProducer,
    /// A required semantic relationship is absent.
    MissingRequiredLink,
    /// A semantic reference points to itself.
    SelfReference,
    /// A source field path is not an absolute JSON pointer.
    InvalidSourcePath,
    /// A producer environment has no declared entries.
    EmptyProducerEnvironment,
    /// A required textual field is empty.
    EmptyRequiredField,
    /// A digest is not a lowercase SHA-256 value.
    InvalidDigest,
    /// Two semantic references declare the same identity.
    DuplicateSemanticIdentity,
    /// A semantic relationship names an absent reference.
    MissingReference,
    /// A semantic relationship targets the wrong concept.
    ReferenceConceptMismatch,
    /// A semantic fixture lacks required content.
    EmptyFixture,
    /// A source-version premise is duplicated.
    DuplicateSourcePremise,
    /// Observed source versions differ from the declared premises.
    SourcePremiseMismatch,
    /// Encoded input cannot be decoded into the declared type.
    InvalidInputEncoding,
    /// The ownership registry violates its version or non-executing boundary.
    InvalidOwnershipBoundary,
    /// Ownership metadata is incomplete.
    IncompleteOwnershipMetadata,
    /// The ownership registry repeats a semantic concept.
    DuplicateOwnershipConcept,
    /// The ownership registry omits a required semantic concept.
    IncompleteOwnershipConceptSet,
    /// The ownership registry has an invalid result-state vocabulary.
    InvalidResultStateSet,
    /// A validated value could not be serialized.
    Serialization,
    /// A legacy PGM-01 field violates its declared shape.
    InvalidLegacyField,
    /// A legacy PGM-01 numeric field is not a non-negative integer.
    InvalidLegacyInteger,
    /// A caller-supplied expected digest is invalid.
    InvalidExpectedDigest,
    /// A generated PGM-01 view has the wrong mapping protocol.
    InvalidMappingVersion,
    /// A generated PGM-01 view has an empty source identity.
    EmptySourceIdentity,
    /// A generated PGM-01 view does not preserve its source digest.
    ChangedSourceDigest,
    /// A generated PGM-01 field mapping is invalid.
    InvalidFieldMapping,
    /// A generated PGM-01 unmapped-field record is invalid.
    InvalidUnmappedField,
    /// A generated PGM-01 limitation is empty.
    EmptyLimitation,
}

/// Stable failure at the pure semantic boundary.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("{message}")]
pub struct SemanticError {
    kind: SemanticErrorKind,
    message: String,
}

impl SemanticError {
    fn new(kind: SemanticErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Return the stable top-level machine-readable error category.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        "invalid_semantic_contract"
    }

    /// Return the closed machine-readable refusal reason.
    #[must_use]
    pub const fn kind(&self) -> SemanticErrorKind {
        self.kind
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
            return Err(SemanticError::new(
                SemanticErrorKind::UnsupportedProtocol,
                format!(
                    "unsupported semantic-reference protocol {:?}",
                    self.projection_type
                ),
            ));
        }
        validate_identity(&self.semantic_id, "semantic_id")?;
        if self.authority != self.concept.authority() {
            return Err(SemanticError::new(
                SemanticErrorKind::AuthorityMismatch,
                format!(
                    "{} authority must be {}, not {}",
                    self.concept,
                    self.concept.authority(),
                    self.authority
                ),
            ));
        }
        validate_source(&self.source)?;
        if self.concept.requires_producer() {
            let producer = self.producer.as_ref().ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::MissingProducer,
                    format!("{} requires the complete producer tuple", self.concept),
                )
            })?;
            validate_producer(producer)?;
        } else if let Some(producer) = &self.producer {
            validate_producer(producer)?;
        }
        for required in required_links(self.concept) {
            if self.links.get(required).is_none() {
                return Err(SemanticError::new(
                    SemanticErrorKind::MissingRequiredLink,
                    format!("{} is missing link {required}", self.concept),
                ));
            }
        }
        for (name, target) in self.links.entries() {
            validate_identity(target, name)?;
            if target == &self.semantic_id {
                return Err(SemanticError::new(
                    SemanticErrorKind::SelfReference,
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
        return Err(SemanticError::new(
            SemanticErrorKind::InvalidSourcePath,
            "source field_path must begin with '/'",
        ));
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
        return Err(SemanticError::new(
            SemanticErrorKind::EmptyProducerEnvironment,
            "producer environment must not be empty",
        ));
    }
    non_empty(&producer.definition_version, "producer definition_version")
}

fn non_empty(value: &str, field: &str) -> Result<(), SemanticError> {
    if value.is_empty() {
        Err(SemanticError::new(
            SemanticErrorKind::EmptyRequiredField,
            format!("{field} must not be empty"),
        ))
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
        return Err(SemanticError::new(
            SemanticErrorKind::InvalidIdentity,
            format!("{field} is not a valid identity"),
        ));
    }
    Ok(())
}

fn validate_digest(value: &str, field: &str) -> Result<(), SemanticError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(SemanticError::new(
            SemanticErrorKind::InvalidDigest,
            format!("{field} must be a SHA-256 digest"),
        ));
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
            return Err(SemanticError::new(
                SemanticErrorKind::DuplicateSemanticIdentity,
                "semantic identifiers must be distinct",
            ));
        }
    }
    for reference in references {
        for (relationship, target) in reference.links.entries() {
            let Some(expected) = internal_target(relationship) else {
                continue;
            };
            let actual = known.get(target.as_str()).ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::MissingReference,
                    format!("{} has missing reference {target}", reference.semantic_id),
                )
            })?;
            if *actual != expected {
                return Err(SemanticError::new(
                    SemanticErrorKind::ReferenceConceptMismatch,
                    format!(
                        "{} link {relationship} must target {expected}, not {actual}",
                        reference.semantic_id
                    ),
                ));
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
            return Err(SemanticError::new(
                SemanticErrorKind::UnsupportedProtocol,
                format!(
                    "unsupported semantic fixture protocol {:?}",
                    self.fixture_version
                ),
            ));
        }
        if self.source_version_premises.is_empty() || self.references.is_empty() {
            return Err(SemanticError::new(
                SemanticErrorKind::EmptyFixture,
                "semantic fixture premises and references must not be empty",
            ));
        }
        validate_semantic_bundle(&self.references)?;
        let premises: BTreeSet<_> = self.source_version_premises.iter().cloned().collect();
        if premises.len() != self.source_version_premises.len() {
            return Err(SemanticError::new(
                SemanticErrorKind::DuplicateSourcePremise,
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
            return Err(SemanticError::new(
                SemanticErrorKind::SourcePremiseMismatch,
                "source-version premises differ",
            ));
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
    let fixture: SemanticFixture = serde_json::from_slice(raw).map_err(|error| {
        SemanticError::new(
            SemanticErrorKind::InvalidInputEncoding,
            format!("invalid semantic fixture: {error}"),
        )
    })?;
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
    let registry: OwnershipRegistry = serde_json::from_slice(raw).map_err(|error| {
        SemanticError::new(
            SemanticErrorKind::InvalidInputEncoding,
            format!("invalid ownership registry: {error}"),
        )
    })?;
    if registry.schema_version != "engineering-assurance.verification-semantics-ownership/v1"
        || registry.purpose != "semantic_ownership_registry"
        || !registry.non_executing
    {
        return Err(SemanticError::new(
            SemanticErrorKind::InvalidOwnershipBoundary,
            "ownership registry version or non-executing boundary is invalid",
        ));
    }
    let mut concepts = BTreeSet::new();
    for item in &registry.concepts {
        if item.authority != item.concept.authority() {
            return Err(SemanticError::new(
                SemanticErrorKind::AuthorityMismatch,
                format!(
                    "{} authority must be {}, not {}",
                    item.concept,
                    item.concept.authority(),
                    item.authority
                ),
            ));
        }
        let authoritative_types: BTreeSet<_> = item.authoritative_types.iter().collect();
        if item.authoritative_types.is_empty()
            || authoritative_types.len() != item.authoritative_types.len()
            || item.authoritative_types.iter().any(String::is_empty)
            || item.link_direction.is_empty()
            || item.responsibility.is_empty()
        {
            return Err(SemanticError::new(
                SemanticErrorKind::IncompleteOwnershipMetadata,
                "ownership registry concept metadata is incomplete",
            ));
        }
        if !concepts.insert(item.concept) {
            return Err(SemanticError::new(
                SemanticErrorKind::DuplicateOwnershipConcept,
                "ownership registry repeats a semantic concept",
            ));
        }
    }
    if concepts != SemanticConcept::ALL.into_iter().collect() {
        return Err(SemanticError::new(
            SemanticErrorKind::IncompleteOwnershipConceptSet,
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
            SemanticErrorKind::InvalidResultStateSet,
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
