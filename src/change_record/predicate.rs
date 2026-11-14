// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Closed `change-record/v1` predicate types and deterministic builders
//! (FR-072).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

use crate::record_sink::form::{GitObjectId, PrivateRecord, Reference};

/// Predicate type URI for `change-record/v1`.
pub const PREDICATE_TYPE: &str = concat!(
    "https:",
    "//schemas.agent-ix.ai/attestation/change-record/v1"
);
/// In-toto Statement v1 type URI.
pub const STATEMENT_TYPE: &str = concat!("https:", "//in-toto.io/Statement/v1");
const PREDICATE_SCHEMA: &str =
    include_str!("../change_record_schemas/change-record-v1.schema.json");

/// Errors returned when building or validating a predicate.
#[derive(Debug, Error)]
pub enum PredicateError {
    /// JSON serialization, canonicalization, or schema validation failed.
    #[error("change predicate is invalid")]
    Invalid,
}

/// Ticket reference kind; repository tickets are deliberately not accepted.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TicketKind {
    /// A Linear issue reference.
    #[serde(rename = "linear_issue")]
    LinearIssue,
}

/// Ticket reference without private title or body text.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ticket {
    /// The only supported ticket kind.
    pub kind: TicketKind,
    /// Linear issue identifier such as `EA-107`.
    pub id: String,
}

/// Receipt outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// No failures were recorded.
    Passed,
    /// One or more failures were recorded.
    Failed,
}

/// Review artifact category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewKind {
    /// Spec review.
    SpecReview,
    /// General code review.
    CodeReview,
    /// Rust-specific code review.
    RustReview,
    /// Gap analysis.
    GapAnalysis,
}

/// Review result.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdict {
    /// Review passed.
    Pass,
    /// Review reported findings.
    Findings,
    /// Review failed.
    Fail,
}

/// Public review reference; raw review content remains private.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    /// Review category.
    pub kind: ReviewKind,
    /// Recorded verdict.
    pub verdict: ReviewVerdict,
    /// SHA-256 of the raw review artifact bytes.
    pub artifact_digest: String,
}

/// Review data supplied to the builder; artifact bytes are hashed as-is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewInput {
    /// Review category.
    pub kind: ReviewKind,
    /// Recorded verdict.
    pub verdict: ReviewVerdict,
    /// Raw Markdown bytes of the review artifact.
    pub artifact_bytes: Vec<u8>,
}

/// One role's approval evaluation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApprovalDecision {
    /// A distinct eligible person supplied a verifiable statement.
    Covered {
        /// Stable policy individual identity. It is used only to enforce
        /// distinct-human coverage and is never serialized into the predicate.
        individual: String,
        /// Whether the source classified the approver as a human.
        is_human: bool,
        /// Verified approval statement JSON, hashed by the builder.
        statement: Value,
    },
    /// No valid approval statement exists for this role.
    Missing,
    /// No eligible human can staff this required role.
    Unstaffed,
    /// The role evaluation or statement verification was unavailable.
    Unavailable,
}

/// Public role approval result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalEntry {
    /// Required role name from the repository policy.
    pub role: String,
    /// Whether this role has a verified statement.
    pub verdict: ApprovalVerdict,
    /// Present exactly when the verdict is `covered`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_digest: Option<String>,
}

/// Public approval verdict.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalVerdict {
    /// Role was covered by a verified statement.
    Covered,
    /// Required role was not covered.
    Missing,
}

/// Closed failure reason.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    /// A required gate failed.
    GateFailed,
    /// At least one required role was not covered.
    ApprovalsIncomplete,
    /// A required role has no eligible person.
    ApprovalRoleUnstaffed,
    /// Role evaluation or statement verification was unavailable.
    ApprovalsUnavailable,
    /// Required change contract was absent.
    ContractMissing,
    /// Required reviews were absent.
    ReviewsMissing,
    /// Publication leak check refused the public record.
    LeakCheckRefused,
}

/// The complete public predicate with no prose-bearing fields.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangePredicate {
    /// Lowercase UUID for this record.
    pub record_id: String,
    /// Predicate status.
    pub status: Status,
    /// Ticket reference; omitted when the contract is missing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticket: Option<Ticket>,
    /// Ticket acceptance criterion identifiers.
    pub criteria: Vec<String>,
    /// Traced specification criterion identifiers.
    pub spec_criteria: Vec<String>,
    /// Review artifact references.
    pub reviews: Vec<Review>,
    /// Exactly one approval entry for each configured role.
    pub approvals: Vec<ApprovalEntry>,
    /// Digest of the Test Result statement.
    pub test_results: String,
    /// Digest of the private `CycloneDX` agent BOM, omitted when no session
    /// descriptor was supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_bom: Option<String>,
    /// Digest of the private form.
    pub private_form: String,
    /// Closed failure list.
    pub failures: Vec<Failure>,
}

/// Inputs that are safe to publish after policy evaluation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateInput {
    /// Lowercase record UUID.
    pub record_id: String,
    /// Optional Linear ticket reference.
    pub ticket: Option<Ticket>,
    /// Ticket acceptance criterion identifiers.
    pub criteria: Vec<String>,
    /// Specification criterion identifiers.
    pub spec_criteria: Vec<String>,
    /// Review artifact bytes and results.
    pub reviews: Vec<ReviewInput>,
    /// Ordered role and evaluation pairs from the per-repository role policy.
    pub approvals: Vec<(String, ApprovalDecision)>,
    /// Test Result statement JSON, hashed by the builder.
    pub test_results: Value,
    /// Subject repository and merged Git objects.
    pub subject: ChangeSubjectInput,
    /// Session descriptor; absence produces a failed receipt with no BOM.
    pub session: Option<super::bom::AgentSession>,
    /// Private form shell. It must not already contain a statement or BOM;
    /// the builder fills both from the inputs above.
    pub private_form: PrivateRecord,
    /// Other receipt failures already established by the caller.
    pub failures: Vec<Failure>,
}

/// Repository and squash-merge subject inputs for the in-toto statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeSubjectInput {
    /// Repository name in `owner/name` form.
    pub repository: String,
    /// Squash-merged commit object id.
    pub merged_commit: String,
    /// Tree object id of the merged commit.
    pub merged_tree: String,
}

/// One in-toto statement subject.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatementSubject {
    /// Stable repository name.
    pub name: String,
    /// Commit and tree object ids.
    pub digest: BTreeMap<String, String>,
}

/// Change-record statement binding the predicate to the merged commit and tree.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeStatement {
    /// In-toto Statement v1 type URI.
    #[serde(rename = "_type")]
    pub type_: String,
    /// The merged repository, commit and tree.
    pub subject: Vec<StatementSubject>,
    /// Change-record predicate type URI.
    #[serde(rename = "predicateType")]
    pub predicate_type: String,
    /// Closed change-record predicate.
    pub predicate: ChangePredicate,
}

/// A built record keeps the private form beside the public statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuiltChangeRecord {
    /// Public in-toto statement suitable for signing and note storage.
    pub statement: ChangeStatement,
    /// Private record carrying the text-bearing change record and BOM under id.
    pub private_form: PrivateRecord,
}

/// Computes `sha256:` over RFC 8785 canonical JSON bytes.
///
/// # Errors
///
/// Returns [`PredicateError::Invalid`] when canonical serialization fails.
pub fn canonical_digest(value: &Value) -> Result<String, PredicateError> {
    let bytes = serde_json_canonicalizer::to_vec(value).map_err(|_| PredicateError::Invalid)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

/// Builds the private form, public predicate, and commit-bound statement.
///
/// # Errors
///
/// Returns [`PredicateError::Invalid`] if the record id, subject, private form,
/// BOM, canonicalization, or predicate schema is invalid.
pub fn build_change_record(mut input: PredicateInput) -> Result<BuiltChangeRecord, PredicateError> {
    validate_private_shell(&input)?;
    let (repository, commit, tree) = validated_subject(&input.subject)?;
    let bom_value = build_private_bom(input.session.as_ref(), &mut input.private_form)?;
    let mut failures = input.failures;
    if bom_value.is_none() && !failures.contains(&Failure::ContractMissing) {
        failures.push(Failure::ContractMissing);
    }
    let approvals = build_approvals(
        input.approvals,
        &mut failures,
        &mut input.private_form.statement,
    )?;
    if input.ticket.is_none() && !failures.contains(&Failure::ContractMissing) {
        failures.push(Failure::ContractMissing);
    }
    failures.sort_by_key(|failure| *failure as u8);
    failures.dedup();
    let status = if failures.is_empty() {
        Status::Passed
    } else {
        Status::Failed
    };
    let reviews = build_reviews(input.reviews);
    let private_value =
        serde_json::to_value(&input.private_form).map_err(|_| PredicateError::Invalid)?;
    let predicate = ChangePredicate {
        record_id: input.record_id,
        status,
        ticket: input.ticket,
        criteria: input.criteria,
        spec_criteria: input.spec_criteria,
        reviews,
        approvals,
        test_results: canonical_digest(&input.test_results)?,
        agent_bom: bom_value.as_ref().map(canonical_digest).transpose()?,
        private_form: canonical_digest(&private_value)?,
        failures,
    };
    validate_predicate(&predicate)?;
    let statement = ChangeStatement {
        type_: STATEMENT_TYPE.to_owned(),
        subject: vec![StatementSubject {
            name: repository.id().to_owned(),
            digest: BTreeMap::from([
                ("gitCommit".to_owned(), commit.into()),
                ("gitTree".to_owned(), tree.into()),
            ]),
        }],
        predicate_type: PREDICATE_TYPE.to_owned(),
        predicate,
    };
    Ok(BuiltChangeRecord {
        statement,
        private_form: input.private_form,
    })
}

fn validate_private_shell(input: &PredicateInput) -> Result<(), PredicateError> {
    if input.private_form.record_id.as_str() != input.record_id
        || input.private_form.statement.is_some()
        || input.private_form.agent_bom.is_some()
        || input
            .private_form
            .change_record
            .as_ref()
            .and_then(Value::as_object)
            .is_none_or(|record| record.contains_key("private_form"))
    {
        return Err(PredicateError::Invalid);
    }
    Ok(())
}

fn validated_subject(
    subject: &ChangeSubjectInput,
) -> Result<(Reference, GitObjectId, GitObjectId), PredicateError> {
    let repository = Reference::repo(&subject.repository).map_err(|_| PredicateError::Invalid)?;
    let commit = GitObjectId::try_from(subject.merged_commit.as_str())
        .map_err(|_| PredicateError::Invalid)?;
    let tree =
        GitObjectId::try_from(subject.merged_tree.as_str()).map_err(|_| PredicateError::Invalid)?;
    Ok((repository, commit, tree))
}

fn build_private_bom(
    session: Option<&super::bom::AgentSession>,
    private_form: &mut PrivateRecord,
) -> Result<Option<Value>, PredicateError> {
    let Some(session) = session else {
        return Ok(None);
    };
    let bom = super::bom::build_bom(session).map_err(|_| PredicateError::Invalid)?;
    let value = serde_json::to_value(bom).map_err(|_| PredicateError::Invalid)?;
    super::bom::validate_bom_value(&value).map_err(|_| PredicateError::Invalid)?;
    private_form.agent_bom = Some(value.clone());
    private_form.model_names = Some(vec![session.model.clone()]);
    Ok(Some(value))
}

fn build_approvals(
    mut decisions: Vec<(String, ApprovalDecision)>,
    failures: &mut Vec<Failure>,
    private_statements: &mut Option<Value>,
) -> Result<Vec<ApprovalEntry>, PredicateError> {
    decisions.sort_by(|(left, _), (right, _)| left.cmp(right));
    let mut entries = Vec::with_capacity(decisions.len());
    let mut seen_individuals = BTreeSet::new();
    let mut seen_roles = BTreeSet::new();
    let mut statements = Vec::new();
    for (role, decision) in decisions {
        if !seen_roles.insert(role.clone()) {
            return Err(PredicateError::Invalid);
        }
        entries.push(approval_entry(
            role,
            decision,
            failures,
            &mut seen_individuals,
            &mut statements,
        )?);
    }
    if !statements.is_empty() {
        *private_statements = Some(Value::Array(statements));
    }
    Ok(entries)
}

fn approval_entry(
    role: String,
    decision: ApprovalDecision,
    failures: &mut Vec<Failure>,
    seen_individuals: &mut BTreeSet<String>,
    statements: &mut Vec<Value>,
) -> Result<ApprovalEntry, PredicateError> {
    let (verdict, statement_digest) = match decision {
        ApprovalDecision::Covered {
            individual,
            is_human,
            statement,
        } if is_human && !individual.is_empty() && seen_individuals.insert(individual.clone()) => {
            let digest = canonical_digest(&statement)?;
            statements.push(statement);
            (ApprovalVerdict::Covered, Some(digest))
        }
        ApprovalDecision::Unstaffed => {
            failures.push(Failure::ApprovalRoleUnstaffed);
            failures.push(Failure::ApprovalsIncomplete);
            (ApprovalVerdict::Missing, None)
        }
        ApprovalDecision::Unavailable => {
            failures.push(Failure::ApprovalsUnavailable);
            failures.push(Failure::ApprovalsIncomplete);
            (ApprovalVerdict::Missing, None)
        }
        ApprovalDecision::Missing | ApprovalDecision::Covered { .. } => {
            failures.push(Failure::ApprovalsIncomplete);
            (ApprovalVerdict::Missing, None)
        }
    };
    Ok(ApprovalEntry {
        role,
        verdict,
        statement_digest,
    })
}

fn build_reviews(reviews: Vec<ReviewInput>) -> Vec<Review> {
    reviews
        .into_iter()
        .map(|review| Review {
            kind: review.kind,
            verdict: review.verdict,
            artifact_digest: raw_digest(&review.artifact_bytes),
        })
        .collect()
}

fn raw_digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// Validates the exact serialized predicate against its closed JSON Schema.
///
/// # Errors
///
/// Returns [`PredicateError::Invalid`] when serialization or schema
/// validation fails.
pub fn validate_predicate(predicate: &ChangePredicate) -> Result<(), PredicateError> {
    let schema: Value =
        serde_json::from_str(PREDICATE_SCHEMA).map_err(|_| PredicateError::Invalid)?;
    let compiled = jsonschema::validator_for(&schema).map_err(|_| PredicateError::Invalid)?;
    let value = serde_json::to_value(predicate).map_err(|_| PredicateError::Invalid)?;
    if compiled.is_valid(&value) {
        Ok(())
    } else {
        Err(PredicateError::Invalid)
    }
}

/// Returns whether the private form still matches the predicate's binding.
#[must_use]
pub fn verify_private_form_digest(
    predicate: &ChangePredicate,
    private_form: &PrivateRecord,
) -> bool {
    serde_json::to_value(private_form)
        .ok()
        .and_then(|value| canonical_digest(&value).ok())
        .is_some_and(|digest| digest == predicate.private_form)
}
