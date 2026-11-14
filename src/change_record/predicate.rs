// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Closed `change-record/v1` predicate types and deterministic builders
//! (FR-072).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use thiserror::Error;

/// Predicate type URI for `change-record/v1`.
pub const PREDICATE_TYPE: &str = concat!(
    "https:",
    "//schemas.agent-ix.ai/attestation/change-record/v1"
);
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
        /// SHA-256 digest of the verified approval statement.
        statement_digest: String,
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
    /// Digest of the private `CycloneDX` agent BOM.
    pub agent_bom: String,
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
    /// Review artifact references.
    pub reviews: Vec<Review>,
    /// Ordered role and evaluation pairs from the per-repository role policy.
    pub approvals: Vec<(String, ApprovalDecision)>,
    /// Digest of the Test Result statement.
    pub test_results: String,
    /// Digest of the private BOM.
    pub agent_bom: String,
    /// Canonical private-form JSON value.
    pub private_form: Value,
    /// Other receipt failures already established by the caller.
    pub failures: Vec<Failure>,
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

/// Builds a predicate and binds it to the exact canonical private form.
///
/// # Errors
///
/// Returns [`PredicateError::Invalid`] if canonicalization or schema
/// validation fails.
pub fn build_predicate(mut input: PredicateInput) -> Result<ChangePredicate, PredicateError> {
    let mut failures = input.failures;
    input
        .approvals
        .sort_by(|(left, _), (right, _)| left.cmp(right));
    let mut approvals = Vec::with_capacity(input.approvals.len());
    let mut seen_individuals = BTreeSet::new();
    let mut seen_roles = BTreeSet::new();
    for (role, decision) in input.approvals {
        if !seen_roles.insert(role.clone()) {
            return Err(PredicateError::Invalid);
        }
        let entry = match decision {
            ApprovalDecision::Covered {
                individual,
                is_human,
                statement_digest,
            } => {
                if is_human && !individual.is_empty() && seen_individuals.insert(individual) {
                    ApprovalEntry {
                        role,
                        verdict: ApprovalVerdict::Covered,
                        statement_digest: Some(statement_digest),
                    }
                } else {
                    failures.push(Failure::ApprovalsIncomplete);
                    ApprovalEntry {
                        role,
                        verdict: ApprovalVerdict::Missing,
                        statement_digest: None,
                    }
                }
            }
            ApprovalDecision::Missing => {
                failures.push(Failure::ApprovalsIncomplete);
                ApprovalEntry {
                    role,
                    verdict: ApprovalVerdict::Missing,
                    statement_digest: None,
                }
            }
            ApprovalDecision::Unstaffed => {
                failures.push(Failure::ApprovalRoleUnstaffed);
                failures.push(Failure::ApprovalsIncomplete);
                ApprovalEntry {
                    role,
                    verdict: ApprovalVerdict::Missing,
                    statement_digest: None,
                }
            }
            ApprovalDecision::Unavailable => {
                failures.push(Failure::ApprovalsUnavailable);
                failures.push(Failure::ApprovalsIncomplete);
                ApprovalEntry {
                    role,
                    verdict: ApprovalVerdict::Missing,
                    statement_digest: None,
                }
            }
        };
        approvals.push(entry);
    }
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
    let predicate = ChangePredicate {
        record_id: input.record_id,
        status,
        ticket: input.ticket,
        criteria: input.criteria,
        spec_criteria: input.spec_criteria,
        reviews: input.reviews,
        approvals,
        test_results: input.test_results,
        agent_bom: input.agent_bom,
        private_form: canonical_digest(&input.private_form)?,
        failures,
    };
    validate_predicate(&predicate)?;
    Ok(predicate)
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
pub fn verify_private_form_digest(predicate: &ChangePredicate, private_form: &Value) -> bool {
    canonical_digest(private_form).is_ok_and(|digest| digest == predicate.private_form)
}
