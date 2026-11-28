// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded source lookup and private-store verification for release roles
//! (FR-067).

use std::path::PathBuf;

use engineering_assurance::{
    approval_gate::{GateErrorCode, GateResult, GateStatus, RolePolicy, parse_policy},
    approval_statement::{ApprovalStatementStatus, verify_approval_statements},
    attestation_signing::{
        SigstoreBundleVerifier, SigstoreSigner, TransparencyRequirement, WorkflowIdentityDocument,
        WorkflowRole,
    },
    producer_execution::private_git_store::{
        GitPrivateStore, HttpsGitDestinationResolver, RecordPublisherIdentity,
    },
    record_sink::{
        config::{SinkConfig, Visibility, resolve_sink_for_repository},
        sink::ApprovalStatementLookup,
    },
};
use ix_trace_rs::implements;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::receipt_host;

const REQUEST_PROTOCOL: &str = "engineering-assurance.release-roles-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.release-roles-result/v1";
const MAX_REQUEST_BYTES: usize = 1024 * 1024;
const MAX_POLICY_BYTES: usize = 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    repository: PathBuf,
    github_repository: String,
    base_branch: String,
    tag: String,
    commit: String,
}

#[derive(Serialize)]
struct ResultLine {
    protocol: &'static str,
    status: &'static str,
    pull_request: String,
    commit: String,
    missing_roles: Vec<String>,
}

#[derive(Debug, Error)]
pub(crate) enum ReleaseRolesError {
    #[error("release role request is invalid")]
    Request,
    #[error("protected release role policy is unavailable or invalid")]
    Policy,
    #[error("release source lookup is unavailable")]
    SourceUnavailable,
    #[error("no merged pull request has the release commit as its merge result")]
    NoPullRequest,
    #[error("release approval statements have not been retained yet")]
    StatementsPending,
    #[error("retained release approval statements are not verifiable")]
    StatementsUnverifiable,
    #[error("the release role policy is not satisfied: {0}")]
    RolesUnsatisfied(String),
    #[error("the release role policy names an unstaffed role: {0}")]
    RoleUnstaffed(String),
    #[error("release role result cannot be encoded")]
    Encoding,
}

impl ReleaseRolesError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Request => "release_roles_request_invalid",
            Self::Policy => "release_roles_policy_unavailable",
            Self::SourceUnavailable => "release_roles_source_unavailable",
            Self::NoPullRequest => "release_roles_no_pull_request",
            Self::StatementsPending => "release_roles_statements_pending",
            Self::StatementsUnverifiable => "release_roles_statements_unverifiable",
            Self::RolesUnsatisfied(_) => "release_roles_unsatisfied",
            Self::RoleUnstaffed(_) => "approval_role_unstaffed",
            Self::Encoding => "release_roles_result_encoding_failed",
        }
    }
}

pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, ReleaseRolesError> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(ReleaseRolesError::Request);
    }
    let request: Request = serde_json::from_slice(bytes).map_err(|_| ReleaseRolesError::Request)?;
    validate_request(&request)?;
    let protected = protected_policy(&request)?;
    let pull_request = merged_pull_request_for_commit(&request)?;
    let trusted_root_json = match protected.transparency {
        TransparencyRequirement::PublicLog => {
            sigstore_verify::trust_root::SIGSTORE_PRODUCTION_TRUSTED_ROOT
        }
        TransparencyRequirement::TimestampOnly => {
            sigstore_verify::trust_root::SIGSTORE_GITHUB_TRUSTED_ROOT
        }
    };
    let trusted_root = sigstore_verify::trust_root::TrustedRoot::from_json(trusted_root_json)
        .map_err(|_| ReleaseRolesError::Policy)?;
    let verifier =
        SigstoreBundleVerifier::new(&trusted_root).map_err(|_| ReleaseRolesError::Policy)?;
    let destination = protected
        .private_store_destination
        .as_deref()
        .ok_or(ReleaseRolesError::Policy)?;
    let resolver = HttpsGitDestinationResolver::new(concat!("https:", "//github.com"))
        .map_err(|_| ReleaseRolesError::Policy)?;
    let signer = SigstoreSigner::new();
    let publisher = RecordPublisherIdentity {
        repository: protected.record_publisher.0.clone(),
        workflow_path: protected.record_publisher.1.clone(),
        reference: protected.record_publisher.2.clone(),
    };
    let store = GitPrivateStore::new(
        &resolver,
        &signer,
        &verifier,
        &protected.policy_source,
        &publisher,
        &request.github_repository,
        "git",
    )
    .map_err(|_| ReleaseRolesError::SourceUnavailable)?;
    let lookup = ApprovalStatementLookup {
        repository: &request.github_repository,
        merged_pull_request: &pull_request,
        merged_commit: &request.commit,
    };
    let verified = verify_approval_statements(
        &store,
        destination,
        lookup,
        &protected.statement_signer,
        &protected.role_policy,
        &verifier,
    );
    if let Some(error) = statement_failure(verified.status, verified.reason) {
        return Err(error);
    }
    if let Some(error) = role_failure(&verified.role_verdict) {
        return Err(error);
    }
    encode_result(pull_request, request.commit)
}

struct ProtectedPolicy {
    policy_source: ProtectedPolicySource,
    role_policy: RolePolicy,
    private_store_destination: Option<String>,
    record_publisher: (String, String, String),
    statement_signer: engineering_assurance::attestation_signing::WorkflowIdentityPolicy,
    transparency: TransparencyRequirement,
}

struct ProtectedPolicySource {
    workflow_identities: Vec<u8>,
}

impl engineering_assurance::record_sink::config::PolicySource for ProtectedPolicySource {
    fn base_branch_protected(&self) -> Option<bool> {
        Some(true)
    }

    fn read_from_base_branch(&self, path: &str) -> Option<Vec<u8>> {
        (path == ".engineering-assurance/workflow-identities.json")
            .then(|| self.workflow_identities.clone())
    }
}

fn protected_policy(request: &Request) -> Result<ProtectedPolicy, ReleaseRolesError> {
    let repository_info =
        receipt_host::gh_api_json(&format!("repos/{}", request.github_repository))
            .map_err(|_| ReleaseRolesError::Policy)?;
    if repository_info
        .get("default_branch")
        .and_then(Value::as_str)
        != Some(request.base_branch.as_str())
    {
        return Err(ReleaseRolesError::Policy);
    }
    let private = repository_info
        .get("private")
        .and_then(Value::as_bool)
        .ok_or(ReleaseRolesError::Policy)?;
    let branch = receipt_host::gh_api_json(&format!(
        "repos/{}/branches/{}",
        request.github_repository, request.base_branch
    ))
    .map_err(|_| ReleaseRolesError::Policy)?;
    if branch.get("protected").and_then(Value::as_bool) != Some(true) {
        return Err(ReleaseRolesError::Policy);
    }
    let base_commit = branch
        .pointer("/commit/sha")
        .and_then(Value::as_str)
        .filter(|value| receipt_host::is_full_object_id(value))
        .ok_or(ReleaseRolesError::Policy)?
        .to_owned();
    let identities = receipt_host::git_show_base_file(
        &request.repository,
        &base_commit,
        ".engineering-assurance/workflow-identities.json",
    )
    .map_err(|_| ReleaseRolesError::Policy)?;
    let identity_document =
        WorkflowIdentityDocument::parse(&identities).map_err(|_| ReleaseRolesError::Policy)?;
    let role_bytes = receipt_host::git_show_base_file(
        &request.repository,
        &base_commit,
        ".engineering-assurance/approval-policy.json",
    )
    .map_err(|_| ReleaseRolesError::Policy)?;
    if role_bytes.len() > MAX_POLICY_BYTES {
        return Err(ReleaseRolesError::Policy);
    }
    let role_policy = parse_policy(&role_bytes).map_err(|_| ReleaseRolesError::Policy)?;
    let sink_bytes = receipt_host::git_show_base_file(
        &request.repository,
        &base_commit,
        ".engineering-assurance/sinks.json",
    )
    .map_err(|_| ReleaseRolesError::Policy)?;
    let sink_config = SinkConfig::parse(&sink_bytes).map_err(|_| ReleaseRolesError::Policy)?;
    let visibility = if private {
        Visibility::Private
    } else {
        Visibility::Public
    };
    let sink_selection =
        resolve_sink_for_repository(Some(&sink_config), visibility, &request.github_repository)
            .map_err(|_| ReleaseRolesError::Policy)?;
    let record_publisher = identity_document
        .sole_signer_identity(WorkflowRole::RecordPublisher)
        .ok_or(ReleaseRolesError::Policy)?;
    let (statement_repository, statement_path, statement_ref) = identity_document
        .sole_signer_identity(WorkflowRole::StatementWriter)
        .ok_or(ReleaseRolesError::Policy)?;
    let transparency = if private {
        TransparencyRequirement::TimestampOnly
    } else {
        TransparencyRequirement::PublicLog
    };
    let statement_signer = identity_document
        .signer_policy(
            WorkflowRole::StatementWriter,
            &statement_repository,
            &statement_path,
            &statement_ref,
            transparency,
        )
        .ok_or(ReleaseRolesError::Policy)?;
    Ok(ProtectedPolicy {
        policy_source: ProtectedPolicySource {
            workflow_identities: identities,
        },
        role_policy,
        private_store_destination: sink_selection.private_store_destination,
        record_publisher,
        statement_signer,
        transparency,
    })
}

fn merged_pull_request_for_commit(request: &Request) -> Result<String, ReleaseRolesError> {
    let rows = receipt_host::gh_api_json(&format!(
        "repos/{}/commits/{}/pulls?per_page=100",
        request.github_repository, request.commit
    ))
    .map_err(|_| ReleaseRolesError::SourceUnavailable)?;
    select_merged_pull_request(&rows, &request.github_repository, &request.commit)
}

/// Select the one source-reported merged pull request whose merge result is
/// exactly the tag's peeled commit. Head commits and non-tip rebases do not
/// establish that binding.
#[implements("FR-067-AC-6", "FR-067-AC-8")]
fn select_merged_pull_request(
    rows: &Value,
    repository: &str,
    commit: &str,
) -> Result<String, ReleaseRolesError> {
    let rows = rows
        .as_array()
        .ok_or(ReleaseRolesError::SourceUnavailable)?;
    if rows.len() >= 100 {
        return Err(ReleaseRolesError::SourceUnavailable);
    }
    let mut matches = rows.iter().filter(|pull_request| {
        pull_request.get("merge_commit_sha").and_then(Value::as_str) == Some(commit)
            && pull_request
                .get("merged_at")
                .is_some_and(|value| !value.is_null())
    });
    let number = matches
        .next()
        .and_then(|pull_request| pull_request.get("number"))
        .and_then(Value::as_u64)
        .ok_or(ReleaseRolesError::NoPullRequest)?;
    if matches.next().is_some() {
        return Err(ReleaseRolesError::SourceUnavailable);
    }
    Ok(format!("{repository}#{number}"))
}

#[implements("FR-067-AC-5")]
fn statement_failure(
    status: ApprovalStatementStatus,
    reason: Option<engineering_assurance::approval_statement::ApprovalStatementReason>,
) -> Option<ReleaseRolesError> {
    match (status, reason) {
        (ApprovalStatementStatus::Verified, _) => None,
        (
            ApprovalStatementStatus::Unverifiable,
            Some(engineering_assurance::approval_statement::ApprovalStatementReason::StatementsNotRetained),
        ) => Some(ReleaseRolesError::StatementsPending),
        _ => Some(ReleaseRolesError::StatementsUnverifiable),
    }
}

#[implements("FR-067-AC-4")]
fn role_failure(verdict: &GateResult) -> Option<ReleaseRolesError> {
    if verdict.status == GateStatus::Unavailable {
        return Some(ReleaseRolesError::SourceUnavailable);
    }
    if verdict.error == Some(GateErrorCode::ApprovalRoleUnstaffed) {
        let role = verdict
            .missing
            .first()
            .map_or_else(|| "unknown".to_owned(), |missing| missing.role.clone());
        return Some(ReleaseRolesError::RoleUnstaffed(role));
    }
    if verdict.status != GateStatus::Satisfied {
        let missing = verdict
            .missing
            .iter()
            .map(|missing| missing.role.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Some(ReleaseRolesError::RolesUnsatisfied(missing));
    }
    None
}

fn validate_request(request: &Request) -> Result<(), ReleaseRolesError> {
    if request.protocol != REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !request.repository.is_dir()
        || !receipt_host::valid_github_repository(&request.github_repository)
        || !receipt_host::valid_branch(&request.base_branch)
        || request.tag.is_empty()
        || request.tag.len() > 256
        || request.tag.starts_with('-')
        || request.tag.contains(['\n', '\r', '\0'])
        || !receipt_host::is_full_object_id(&request.commit)
    {
        return Err(ReleaseRolesError::Request);
    }
    Ok(())
}

fn encode_result(pull_request: String, commit: String) -> Result<Vec<u8>, ReleaseRolesError> {
    let mut bytes = serde_json::to_vec(&ResultLine {
        protocol: RESULT_PROTOCOL,
        status: "satisfied",
        pull_request,
        commit,
        missing_roles: Vec::new(),
    })
    .map_err(|_| ReleaseRolesError::Encoding)?;
    bytes.push(b'\n');
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use engineering_assurance::approval_gate::{Assignment, MissingReason, MissingRole};
    use ix_trace_rs::trace;

    const RELEASE_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn pull_request(merge_commit: &str, head: &str) -> Value {
        serde_json::json!({
            "number": 27,
            "merged_at": "2026-11-27T00:00:00Z",
            "merge_commit_sha": merge_commit,
            "head": {"sha": head}
        })
    }

    #[trace("TC-1212", "FR-067-AC-6", "FR-067-AC-8")]
    #[test]
    fn release_role_lookup_requires_one_exact_merged_commit() {
        let matching = pull_request(RELEASE_COMMIT, "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        assert_eq!(
            select_merged_pull_request(
                &serde_json::json!([matching]),
                "fictional/product",
                RELEASE_COMMIT,
            )
            .expect("exact merge result resolves"),
            "fictional/product#27"
        );

        let head_only = pull_request("cccccccccccccccccccccccccccccccccccccccc", RELEASE_COMMIT);
        assert!(matches!(
            select_merged_pull_request(
                &serde_json::json!([head_only]),
                "fictional/product",
                RELEASE_COMMIT,
            ),
            Err(ReleaseRolesError::NoPullRequest)
        ));
        assert!(matches!(
            select_merged_pull_request(&serde_json::json!([]), "fictional/product", RELEASE_COMMIT),
            Err(ReleaseRolesError::NoPullRequest)
        ));
    }

    #[trace("TC-1213", "FR-067-AC-4")]
    #[test]
    fn role_policy_failure_names_missing_or_unstaffed_roles() {
        let missing = GateResult {
            protocol: engineering_assurance::approval_gate::RESULT_PROTOCOL,
            status: GateStatus::Unsatisfied,
            required_roles: vec!["security".to_owned(), "owner".to_owned()],
            assignments: vec![Assignment {
                role: "owner".to_owned(),
                individual: "alice".to_owned(),
                account: engineering_assurance::approval_gate::AccountRef {
                    source: "github".to_owned(),
                    id: "1".to_owned(),
                },
            }],
            missing: vec![MissingRole {
                role: "security".to_owned(),
                reason: MissingReason::NoApproval,
            }],
            error: None,
            unavailable_accounts: vec![],
        };
        assert!(matches!(
            role_failure(&missing),
            Some(ReleaseRolesError::RolesUnsatisfied(roles)) if roles == "security"
        ));

        let unstaffed = GateResult {
            error: Some(GateErrorCode::ApprovalRoleUnstaffed),
            ..missing
        };
        assert!(matches!(
            role_failure(&unstaffed),
            Some(ReleaseRolesError::RoleUnstaffed(role)) if role == "security"
        ));
    }

    #[trace("TC-1214", "FR-067-AC-5")]
    #[test]
    fn no_retained_statements_is_retryable_but_other_unverifiable_results_are_not_pending() {
        use engineering_assurance::approval_statement::ApprovalStatementReason;

        assert!(matches!(
            statement_failure(
                ApprovalStatementStatus::Unverifiable,
                Some(ApprovalStatementReason::StatementsNotRetained),
            ),
            Some(ReleaseRolesError::StatementsPending)
        ));
        assert!(matches!(
            statement_failure(
                ApprovalStatementStatus::Unverifiable,
                Some(ApprovalStatementReason::InputUnreadable),
            ),
            Some(ReleaseRolesError::StatementsUnverifiable)
        ));
        assert!(statement_failure(ApprovalStatementStatus::Verified, None).is_none());
    }
}
