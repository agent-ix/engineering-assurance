// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded command boundary for `attestation-verify` (FR-068).

use std::{
    ffi::{OsStr, OsString},
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

use engineering_assurance::{
    approval_gate::{GateErrorCode, GateStatus, RolePolicy, parse_policy},
    approval_statement::{
        ApprovalStatementLookup, ApprovalStatementReason, ApprovalStatementResult,
        ApprovalStatementStatus, verify_approval_statements,
    },
    attestation_signing::{
        SigstoreBundleVerifier, SigstoreSigner, SourceWorkflowPolicy, TransparencyRequirement,
        WorkflowIdentityDocument, WorkflowIdentityPolicy, WorkflowRole,
    },
    attestation_verify::{
        ArtifactSubject, ExpectedRelease, ReleaseInputs, SignedStatement, VerificationConditions,
        canonical_digest, verify_release_attestation_set,
    },
    producer_execution::private_git_store::{
        GitPrivateStore, HttpsGitDestinationResolver, RecordPublisherIdentity,
    },
    record_sink::config::{PolicySource, SinkConfig, Visibility, resolve_sink_for_repository},
};
use ix_trace_rs::implements;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::process_host::{self, ProcessLimits};

const REQUEST_PROTOCOL: &str = "engineering-assurance.attestation-verify-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.attestation-verify-result/v1";
const ROLE_REQUEST_PROTOCOL: &str = "engineering-assurance.release-roles-verify-request/v1";
const ROLE_RESULT_PROTOCOL: &str = "engineering-assurance.release-roles-verify-result/v1";
const MAX_DOCUMENT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ARTIFACTS: usize = 64;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(30);
const REMOVED_GIT_ENVIRONMENT: [&str; 12] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_EXTERNAL_DIFF",
    "GIT_DIFF_OPTS",
    "GIT_NAMESPACE",
    "GIT_COMMON_DIR",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_REPLACE_REF_BASE",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    github_repository: String,
    repository: PathBuf,
    tag: String,
    record_path: String,
    allowed_signers_path: String,
    base_branch: String,
    signer_repository: String,
    signer_workflow_path: String,
    signer_ref: String,
    approval_signer_repository: String,
    approval_signer_workflow_path: String,
    approval_signer_ref: String,
    record_publisher_repository: String,
    record_publisher_workflow_path: String,
    record_publisher_ref: String,
    transparency: String,
    artifacts: Vec<ArtifactRequest>,
    statements: StatementSetRequest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RoleRequest {
    protocol: String,
    github_repository: String,
    repository: PathBuf,
    tag: String,
    record_path: String,
    allowed_signers_path: String,
    base_branch: String,
    signer_repository: String,
    signer_workflow_path: String,
    signer_ref: String,
    approval_signer_repository: String,
    approval_signer_workflow_path: String,
    approval_signer_ref: String,
    record_publisher_repository: String,
    record_publisher_workflow_path: String,
    record_publisher_ref: String,
    transparency: String,
    release_artifacts: PathBuf,
    execution_provenance_bundle: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactRequest {
    name: String,
    path: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatementSetRequest {
    execution_provenance: SignedStatementRequest,
    provenance: SignedStatementRequest,
    sbom: SignedStatementRequest,
    test_result: SignedStatementRequest,
    summary: SignedStatementRequest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedStatementRequest {
    payload: PathBuf,
    bundle: PathBuf,
}

#[derive(Serialize)]
struct ResultLine {
    protocol: &'static str,
    status: &'static str,
    tag: String,
    commit: String,
}

#[derive(Serialize)]
struct RoleResultLine {
    protocol: &'static str,
    status: &'static str,
    tag: String,
    commit: String,
}

#[derive(Debug, Error)]
pub(crate) enum AttestationVerifyError {
    #[error("attestation-verify request is not readable JSON")]
    Unreadable,
    #[error("attestation-verify request is not an exact valid request")]
    Invalid,
    #[error("attestation-verify policy is not a valid protected policy document")]
    Policy,
    #[error("attestation-verify approval-statement source is unavailable")]
    RoleSourceUnavailable,
    #[error("attestation-verify release commit has no merged pull request")]
    NoPullRequest,
    #[error("attestation-verify approval statements are not retained yet")]
    StatementsPending,
    #[error("attestation-verify approval policy has an unstaffed required role")]
    RoleUnstaffed,
    #[error("attestation-verify policy base branch is not protected")]
    Unprotected,
    #[error("attestation-verify release tag is not verified")]
    Release,
    #[error("attestation-verify statement set is invalid")]
    Statement,
    #[error("attestation-verify result cannot be encoded")]
    Encoding,
}

impl AttestationVerifyError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Unreadable => "attestation_verify_request_unreadable",
            Self::Invalid => "attestation_verify_request_invalid",
            Self::Policy => "attestation_verify_policy_invalid",
            Self::RoleSourceUnavailable => "release_roles_source_unavailable",
            Self::NoPullRequest => "release_roles_no_pull_request",
            Self::StatementsPending => "release_roles_statements_pending",
            Self::RoleUnstaffed => "approval_role_unstaffed",
            Self::Unprotected => "policy_branch_unprotected",
            Self::Release => "release_not_verified",
            Self::Statement => "attestation_set_invalid",
            Self::Encoding => "attestation_verify_result_encoding_failed",
        }
    }
}

/// Verify one release-attestation set and the signed release tag it describes.
pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, AttestationVerifyError> {
    let request = parse_request(bytes)?;
    validate_request(&request)?;
    if !checkout_matches_repository(&request.repository, &request.github_repository)? {
        return Err(AttestationVerifyError::Invalid);
    }
    let base_commit = protected_base_commit(
        &request.github_repository,
        &request.repository,
        &request.base_branch,
    )?;
    let policies = load_verification_policies(&request, &base_commit)?;
    let commit = verify_release_tag(&request, policies.allowed_signers.path())?;
    verify_role_statements(&request, &commit, &policies)?;
    let attestations = read_attestations(&request)?;
    let expected = expected_release(
        &request,
        &commit,
        attestations.artifacts.clone(),
        attestations.input_digests.clone(),
    );
    verify_attestations(&policies, &expected, &attestations)?;
    encode_result(request.tag, commit)
}

/// Verify the signed release tag and exact retained approval roles before the
/// reusable workflow creates any release attestations.
#[implements("FR-069-AC-5", "FR-069-AC-6")]
pub(crate) fn execute_roles(bytes: &[u8]) -> Result<Vec<u8>, AttestationVerifyError> {
    let source: RoleRequest =
        serde_json::from_slice(bytes).map_err(|_| AttestationVerifyError::Invalid)?;
    if source.protocol != ROLE_REQUEST_PROTOCOL {
        return Err(AttestationVerifyError::Invalid);
    }
    let execution_provenance_bundle = source.execution_provenance_bundle;
    let request = Request {
        protocol: REQUEST_PROTOCOL.to_owned(),
        github_repository: source.github_repository,
        repository: source.repository,
        tag: source.tag,
        record_path: source.record_path,
        allowed_signers_path: source.allowed_signers_path,
        base_branch: source.base_branch,
        signer_repository: source.signer_repository,
        signer_workflow_path: source.signer_workflow_path,
        signer_ref: source.signer_ref,
        approval_signer_repository: source.approval_signer_repository,
        approval_signer_workflow_path: source.approval_signer_workflow_path,
        approval_signer_ref: source.approval_signer_ref,
        record_publisher_repository: source.record_publisher_repository,
        record_publisher_workflow_path: source.record_publisher_workflow_path,
        record_publisher_ref: source.record_publisher_ref,
        transparency: source.transparency,
        artifacts: release_artifacts(&source.release_artifacts)?,
        statements: StatementSetRequest {
            execution_provenance: SignedStatementRequest {
                payload: PathBuf::from("/dev/null"),
                bundle: execution_provenance_bundle,
            },
            provenance: SignedStatementRequest {
                payload: PathBuf::from("/dev/null"),
                bundle: PathBuf::from("/dev/null"),
            },
            sbom: SignedStatementRequest {
                payload: PathBuf::from("/dev/null"),
                bundle: PathBuf::from("/dev/null"),
            },
            test_result: SignedStatementRequest {
                payload: PathBuf::from("/dev/null"),
                bundle: PathBuf::from("/dev/null"),
            },
            summary: SignedStatementRequest {
                payload: PathBuf::from("/dev/null"),
                bundle: PathBuf::from("/dev/null"),
            },
        },
    };
    validate_request(&request)?;
    if !checkout_matches_repository(&request.repository, &request.github_repository)? {
        return Err(AttestationVerifyError::Invalid);
    }
    let base_commit = protected_base_commit(
        &request.github_repository,
        &request.repository,
        &request.base_branch,
    )?;
    let policies = load_verification_policies(&request, &base_commit)?;
    let commit = verify_release_tag(&request, policies.allowed_signers.path())?;
    verify_execution_provenance_before_export(&request, &commit, &policies)?;
    verify_role_statements(&request, &commit, &policies)?;
    let mut result = serde_json::to_vec(&RoleResultLine {
        protocol: ROLE_RESULT_PROTOCOL,
        status: "satisfied",
        tag: request.tag,
        commit,
    })
    .map_err(|_| AttestationVerifyError::Encoding)?;
    result.push(b'\n');
    Ok(result)
}

#[implements("FR-069-AC-5", "FR-069-AC-6")]
fn verify_execution_provenance_before_export(
    request: &Request,
    commit: &str,
    policies: &VerificationPolicies,
) -> Result<(), AttestationVerifyError> {
    let bundle_bytes = read_bounded(
        &request.statements.execution_provenance.bundle,
        MAX_DOCUMENT_BYTES,
    )
    .map_err(|()| AttestationVerifyError::Statement)?;
    let bundle_json =
        std::str::from_utf8(&bundle_bytes).map_err(|_| AttestationVerifyError::Statement)?;
    let bundle = sigstore_types::Bundle::from_json(bundle_json)
        .map_err(|_| AttestationVerifyError::Statement)?;
    let sigstore_types::SignatureContent::DsseEnvelope(envelope) = bundle.content else {
        return Err(AttestationVerifyError::Statement);
    };
    if envelope.payload_type != "application/vnd.in-toto+json" {
        return Err(AttestationVerifyError::Statement);
    }
    let artifacts = request
        .artifacts
        .iter()
        .map(|artifact| {
            let path = absolute_file(&artifact.path, MAX_ARTIFACT_BYTES)
                .map_err(|()| AttestationVerifyError::Invalid)?;
            Ok(ArtifactSubject {
                name: artifact.name.clone(),
                sha256: sha256_file(&path).map_err(|()| AttestationVerifyError::Statement)?,
            })
        })
        .collect::<Result<Vec<_>, AttestationVerifyError>>()?;
    let trusted_root_json = match policies.transparency {
        TransparencyRequirement::PublicLog => {
            sigstore_verify::trust_root::SIGSTORE_PRODUCTION_TRUSTED_ROOT
        }
        TransparencyRequirement::TimestampOnly => {
            sigstore_verify::trust_root::SIGSTORE_GITHUB_TRUSTED_ROOT
        }
    };
    let trusted_root = sigstore_verify::trust_root::TrustedRoot::from_json(trusted_root_json)
        .map_err(|_| AttestationVerifyError::Policy)?;
    let verifier =
        SigstoreBundleVerifier::new(&trusted_root).map_err(|_| AttestationVerifyError::Policy)?;
    engineering_assurance::attestation_verify::verify_execution_provenance(
        &verifier,
        &policies.signer,
        &policies.source,
        commit,
        &artifacts,
        envelope.payload.as_ref(),
        bundle_json,
    )
    .map_err(|_| AttestationVerifyError::Statement)?;
    Ok(())
}

fn release_artifacts(root: &Path) -> Result<Vec<ArtifactRequest>, AttestationVerifyError> {
    if !root.is_absolute() || !root.is_dir() {
        return Err(AttestationVerifyError::Invalid);
    }
    let mut pending = vec![root.to_path_buf()];
    let mut artifacts = Vec::new();
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory).map_err(|_| AttestationVerifyError::Invalid)?;
        for entry in entries {
            let entry = entry.map_err(|_| AttestationVerifyError::Invalid)?;
            let path = entry.path();
            let metadata =
                fs::symlink_metadata(&path).map_err(|_| AttestationVerifyError::Invalid)?;
            if metadata.file_type().is_symlink() {
                return Err(AttestationVerifyError::Invalid);
            }
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() && artifacts.len() < MAX_ARTIFACTS {
                let name = path
                    .strip_prefix(root)
                    .map_err(|_| AttestationVerifyError::Invalid)?
                    .to_str()
                    .ok_or(AttestationVerifyError::Invalid)?
                    .replace('\\', "/");
                artifacts.push(ArtifactRequest { name, path });
            } else {
                return Err(AttestationVerifyError::Invalid);
            }
        }
    }
    artifacts.sort_by(|left, right| left.name.cmp(&right.name));
    if artifacts.is_empty() {
        return Err(AttestationVerifyError::Invalid);
    }
    Ok(artifacts)
}

struct VerificationPolicies {
    base_commit: String,
    allowed_signers: tempfile::NamedTempFile,
    signer: WorkflowIdentityPolicy,
    source: SourceWorkflowPolicy,
    transparency: TransparencyRequirement,
    role_policy: RolePolicy,
    identities: WorkflowIdentityDocument,
    private_store_destination: String,
}

struct AttestationSet {
    artifacts: Vec<ArtifactSubject>,
    input_digests: [String; 3],
    execution_provenance: ReadStatement,
    provenance: ReadStatement,
    sbom: ReadStatement,
    test_result: ReadStatement,
    summary: ReadStatement,
}

fn parse_request(bytes: &[u8]) -> Result<Request, AttestationVerifyError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| AttestationVerifyError::Unreadable)?;
    serde_json::from_value(value).map_err(|_| AttestationVerifyError::Invalid)
}

fn load_verification_policies(
    request: &Request,
    base_commit: &str,
) -> Result<VerificationPolicies, AttestationVerifyError> {
    let allowed_signers_bytes = git_show_base_file(
        &request.repository,
        base_commit,
        &request.allowed_signers_path,
        1024 * 1024,
    )
    .map_err(|()| AttestationVerifyError::Policy)?;
    let mut allowed_signers =
        tempfile::NamedTempFile::new().map_err(|_| AttestationVerifyError::Policy)?;
    std::io::Write::write_all(&mut allowed_signers, &allowed_signers_bytes)
        .map_err(|_| AttestationVerifyError::Policy)?;
    let identity_bytes = git_show_base_file(
        &request.repository,
        base_commit,
        ".engineering-assurance/workflow-identities.json",
        MAX_DOCUMENT_BYTES,
    )
    .map_err(|()| AttestationVerifyError::Policy)?;
    let identities = WorkflowIdentityDocument::parse(&identity_bytes)
        .map_err(|_| AttestationVerifyError::Policy)?;
    let role_bytes = git_show_base_file(
        &request.repository,
        base_commit,
        ".engineering-assurance/approval-policy.json",
        MAX_DOCUMENT_BYTES,
    )
    .map_err(|()| AttestationVerifyError::Policy)?;
    let role_policy = parse_policy(&role_bytes).map_err(|_| AttestationVerifyError::Policy)?;
    let sink_bytes = git_show_base_file(
        &request.repository,
        base_commit,
        ".engineering-assurance/sinks.json",
        MAX_DOCUMENT_BYTES,
    )
    .map_err(|()| AttestationVerifyError::Policy)?;
    let sink_config = SinkConfig::parse(&sink_bytes).map_err(|_| AttestationVerifyError::Policy)?;
    let sink = resolve_sink_for_repository(
        Some(&sink_config),
        Visibility::Private,
        &request.github_repository,
    )
    .map_err(|_| AttestationVerifyError::Policy)?;
    let private_store_destination = sink
        .private_store_destination
        .filter(|destination| !destination.is_empty())
        .ok_or(AttestationVerifyError::Policy)?;

    let transparency = match request.transparency.as_str() {
        "public_log" => TransparencyRequirement::PublicLog,
        "timestamp_only" => TransparencyRequirement::TimestampOnly,
        _ => return Err(AttestationVerifyError::Invalid),
    };
    let signer = identities
        .signer_policy(
            WorkflowRole::ReleaseAttester,
            &request.signer_repository,
            &request.signer_workflow_path,
            &request.signer_ref,
            transparency,
        )
        .ok_or(AttestationVerifyError::Policy)?;
    let source = identities
        .source_policy(
            WorkflowRole::ReleaseAttester,
            &request.signer_repository,
            &request.signer_workflow_path,
            &request.signer_ref,
        )
        .ok_or(AttestationVerifyError::Policy)?;
    Ok(VerificationPolicies {
        base_commit: base_commit.to_owned(),
        allowed_signers,
        signer,
        source,
        transparency,
        role_policy,
        identities,
        private_store_destination,
    })
}

fn verify_release_tag(
    request: &Request,
    allowed_signers: &Path,
) -> Result<String, AttestationVerifyError> {
    let release_request = serde_json::json!({
        "protocol": "engineering-assurance.release-verify-request/v1",
        "repository": &request.repository,
        "tag": &request.tag,
        "record_path": &request.record_path,
        "allowed_signers": allowed_signers.to_string_lossy(),
    });
    let release_output = crate::release_host::execute(release_request.to_string().as_bytes())
        .map_err(|_| AttestationVerifyError::Release)?;
    let release_value: serde_json::Value =
        serde_json::from_slice(&release_output).map_err(|_| AttestationVerifyError::Release)?;
    if release_value
        .get("status")
        .and_then(serde_json::Value::as_str)
        != Some("released")
    {
        return Err(AttestationVerifyError::Release);
    }
    resolve_tag_commit(&request.repository, &request.tag)
}

fn read_attestations(request: &Request) -> Result<AttestationSet, AttestationVerifyError> {
    let artifacts = request
        .artifacts
        .iter()
        .map(|artifact| {
            let path = absolute_file(&artifact.path, MAX_ARTIFACT_BYTES)
                .map_err(|()| AttestationVerifyError::Invalid)?;
            Ok(ArtifactSubject {
                name: artifact.name.clone(),
                sha256: sha256_file(&path).map_err(|()| AttestationVerifyError::Statement)?,
            })
        })
        .collect::<Result<Vec<_>, AttestationVerifyError>>()?;
    let execution_provenance = read_signed_statement(&request.statements.execution_provenance)?;
    let provenance = read_signed_statement(&request.statements.provenance)?;
    let sbom = read_signed_statement(&request.statements.sbom)?;
    let test_result = read_signed_statement(&request.statements.test_result)?;
    let summary = read_signed_statement(&request.statements.summary)?;
    let input_digests = [
        canonical_digest(&provenance.payload, sha256_canonical_bytes)
            .map_err(|_| AttestationVerifyError::Statement)?,
        canonical_digest(&sbom.payload, sha256_canonical_bytes)
            .map_err(|_| AttestationVerifyError::Statement)?,
        canonical_digest(&test_result.payload, sha256_canonical_bytes)
            .map_err(|_| AttestationVerifyError::Statement)?,
    ];
    Ok(AttestationSet {
        artifacts,
        input_digests,
        execution_provenance,
        provenance,
        sbom,
        test_result,
        summary,
    })
}

fn expected_release(
    request: &Request,
    commit: &str,
    artifacts: Vec<ArtifactSubject>,
    input_digests: [String; 3],
) -> ExpectedRelease {
    let repository = request.github_repository.clone();
    let verifier_id = format!(
        "{}{}github.com/{}/{}@{}",
        "https:", "//", request.signer_repository, request.signer_workflow_path, request.signer_ref
    );
    let policy_uri = format!(
        "{}{}github.com/{}/blob/{}/.engineering-assurance/approval-policy.json",
        "https:", "//", repository, request.base_branch
    );
    ExpectedRelease {
        repository,
        tag: request.tag.clone(),
        commit: commit.to_owned(),
        policy_uri,
        verifier_id,
        artifacts,
        input_digests,
    }
}

fn verify_attestations(
    policies: &VerificationPolicies,
    expected: &ExpectedRelease,
    attestations: &AttestationSet,
) -> Result<(), AttestationVerifyError> {
    let trusted_root_json = match policies.transparency {
        TransparencyRequirement::PublicLog => {
            sigstore_verify::trust_root::SIGSTORE_PRODUCTION_TRUSTED_ROOT
        }
        TransparencyRequirement::TimestampOnly => {
            sigstore_verify::trust_root::SIGSTORE_GITHUB_TRUSTED_ROOT
        }
    };
    let trusted_root = sigstore_verify::trust_root::TrustedRoot::from_json(trusted_root_json)
        .map_err(|_| AttestationVerifyError::Policy)?;
    let verifier =
        SigstoreBundleVerifier::new(&trusted_root).map_err(|_| AttestationVerifyError::Policy)?;
    engineering_assurance::attestation_verify::verify_execution_provenance(
        &verifier,
        &policies.signer,
        &policies.source,
        &expected.commit,
        &attestations.artifacts,
        &attestations.execution_provenance.payload,
        &attestations.execution_provenance.bundle,
    )
    .map_err(|_| AttestationVerifyError::Statement)?;
    verify_release_attestation_set(
        &verifier,
        &policies.signer,
        &policies.source,
        expected,
        VerificationConditions {
            release_verified: true,
            roles_satisfied: true,
            inputs_verified: true,
        },
        ReleaseInputs {
            provenance: SignedStatement {
                payload: &attestations.provenance.payload,
                bundle_json: &attestations.provenance.bundle,
            },
            sbom: SignedStatement {
                payload: &attestations.sbom.payload,
                bundle_json: &attestations.sbom.bundle,
            },
            test_result: SignedStatement {
                payload: &attestations.test_result.payload,
                bundle_json: &attestations.test_result.bundle,
            },
        },
        SignedStatement {
            payload: &attestations.summary.payload,
            bundle_json: &attestations.summary.bundle,
        },
    )
    .map_err(|_| AttestationVerifyError::Statement)?;
    Ok(())
}

fn encode_result(tag: String, commit: String) -> Result<Vec<u8>, AttestationVerifyError> {
    let mut result = serde_json::to_vec(&ResultLine {
        protocol: RESULT_PROTOCOL,
        status: "verified",
        tag,
        commit,
    })
    .map_err(|_| AttestationVerifyError::Encoding)?;
    result.push(b'\n');
    Ok(result)
}

/// Resolve the merged PR from the authenticated source before consulting the
/// retained approval statements. A caller-provided role verdict is never used.
#[implements("FR-067-AC-6")]
fn verify_role_statements(
    request: &Request,
    release_commit: &str,
    policies: &VerificationPolicies,
) -> Result<(), AttestationVerifyError> {
    let endpoint = format!(
        "repos/{}/commits/{release_commit}/pulls?per_page=100",
        request.github_repository
    );
    let arguments = [OsStr::new("api"), OsStr::new(&endpoint)];
    let response = process_host::run(
        OsStr::new("gh"),
        &arguments,
        ProcessLimits {
            timeout: PROCESS_TIMEOUT,
            max_output_bytes: usize::try_from(MAX_DOCUMENT_BYTES).unwrap_or(usize::MAX),
        },
    )
    .map_err(|_| AttestationVerifyError::RoleSourceUnavailable)?;
    if !response.status.success() {
        return Err(AttestationVerifyError::RoleSourceUnavailable);
    }
    let value: serde_json::Value = serde_json::from_slice(&response.stdout)
        .map_err(|_| AttestationVerifyError::RoleSourceUnavailable)?;
    let pull_request = select_merged_pull_request(&value, release_commit)?;
    let workflow_signer = policies
        .identities
        .signer_policy(
            WorkflowRole::StatementWriter,
            &request.approval_signer_repository,
            &request.approval_signer_workflow_path,
            &request.approval_signer_ref,
            policies.transparency,
        )
        .ok_or(AttestationVerifyError::Policy)?;
    let publisher = RecordPublisherIdentity {
        repository: request.record_publisher_repository.clone(),
        workflow_path: request.record_publisher_workflow_path.clone(),
        reference: request.record_publisher_ref.clone(),
    };
    let protected_source = ProtectedBasePolicySource {
        repository: request.repository.clone(),
        base_commit: policies.base_commit.clone(),
    };
    let trusted_root_json = match policies.transparency {
        TransparencyRequirement::PublicLog => {
            sigstore_verify::trust_root::SIGSTORE_PRODUCTION_TRUSTED_ROOT
        }
        TransparencyRequirement::TimestampOnly => {
            sigstore_verify::trust_root::SIGSTORE_GITHUB_TRUSTED_ROOT
        }
    };
    let trusted_root = sigstore_verify::trust_root::TrustedRoot::from_json(trusted_root_json)
        .map_err(|_| AttestationVerifyError::Policy)?;
    let verifier =
        SigstoreBundleVerifier::new(&trusted_root).map_err(|_| AttestationVerifyError::Policy)?;
    let resolver = HttpsGitDestinationResolver::new(concat!("https:", "//github.com"))
        .map_err(|_| AttestationVerifyError::Policy)?;
    let signer = SigstoreSigner::new();
    let reader = GitPrivateStore::new(
        &resolver,
        &signer,
        &verifier,
        &protected_source,
        &publisher,
        &request.github_repository,
        "git",
    )
    .map_err(|_| AttestationVerifyError::RoleSourceUnavailable)?;
    let pull_request_reference = format!("{}#{}", request.github_repository, pull_request.number);
    let result = verify_approval_statements(
        &reader,
        &policies.private_store_destination,
        ApprovalStatementLookup {
            repository: &request.github_repository,
            merged_pull_request: &pull_request_reference,
            merged_commit: release_commit,
        },
        &workflow_signer,
        &policies.role_policy,
        &verifier,
    );
    classify_approval_result(&result)
}

fn classify_approval_result(
    result: &ApprovalStatementResult,
) -> Result<(), AttestationVerifyError> {
    if result.role_verdict.error == Some(GateErrorCode::ApprovalRoleUnstaffed) {
        return Err(AttestationVerifyError::RoleUnstaffed);
    }
    match result.status {
        ApprovalStatementStatus::Verified
            if result.role_verdict.status == GateStatus::Satisfied =>
        {
            Ok(())
        }
        ApprovalStatementStatus::Unverifiable
            if result.reason == Some(ApprovalStatementReason::StatementsNotRetained) =>
        {
            Err(AttestationVerifyError::StatementsPending)
        }
        ApprovalStatementStatus::Unverifiable => Err(AttestationVerifyError::RoleSourceUnavailable),
        _ => Err(AttestationVerifyError::Statement),
    }
}

struct ProtectedBasePolicySource {
    repository: PathBuf,
    base_commit: String,
}

impl PolicySource for ProtectedBasePolicySource {
    fn base_branch_protected(&self) -> Option<bool> {
        Some(true)
    }

    fn read_from_base_branch(&self, path: &str) -> Option<Vec<u8>> {
        git_show_base_file(
            &self.repository,
            &self.base_commit,
            path,
            MAX_DOCUMENT_BYTES,
        )
        .ok()
    }
}

#[derive(Debug, Eq, PartialEq)]
struct MergedPullRequest {
    number: u64,
    head_commit: String,
}

/// Select the unique merged PR whose immutable merge result is the tagged commit.
///
/// GitHub's commit-to-PR endpoint is an authenticated source boundary. An
/// empty result means the tag is not a PR merge result; malformed, ambiguous,
/// or incomplete source data is unavailable and must remain retryable.
fn merged_pull_request_for_commit(
    response: &serde_json::Value,
    commit: &str,
) -> Result<Option<MergedPullRequest>, ()> {
    let entries = response.as_array().ok_or(())?;
    if entries.len() > 100 {
        return Err(());
    }
    let mut matching = Vec::new();
    for entry in entries {
        let number = entry
            .get("number")
            .and_then(serde_json::Value::as_u64)
            .filter(|number| *number > 0)
            .ok_or(())?;
        let state = entry
            .get("state")
            .and_then(serde_json::Value::as_str)
            .filter(|state| matches!(*state, "open" | "closed"))
            .ok_or(())?;
        let head_commit = entry
            .pointer("/head/sha")
            .and_then(serde_json::Value::as_str)
            .filter(|value| is_commit_id(value))
            .ok_or(())?;
        let merged_at = entry.get("merged_at").ok_or(())?;
        let merge_commit = entry.get("merge_commit_sha").ok_or(())?;
        let merged_at = match merged_at {
            serde_json::Value::Null => None,
            serde_json::Value::String(value) if !value.is_empty() => Some(value.as_str()),
            _ => return Err(()),
        };
        let merge_commit = match merge_commit {
            serde_json::Value::Null => None,
            serde_json::Value::String(value) if is_commit_id(value) => Some(value.as_str()),
            _ => return Err(()),
        };
        if state == "closed" && merged_at.is_some() && merge_commit == Some(commit) {
            matching.push(MergedPullRequest {
                number,
                head_commit: head_commit.to_owned(),
            });
        }
    }
    if matching.len() > 1 {
        return Err(());
    }
    Ok(matching.pop())
}

fn select_merged_pull_request(
    response: &serde_json::Value,
    commit: &str,
) -> Result<MergedPullRequest, AttestationVerifyError> {
    merged_pull_request_for_commit(response, commit)
        .map_err(|()| AttestationVerifyError::RoleSourceUnavailable)?
        .ok_or(AttestationVerifyError::NoPullRequest)
}

struct ReadStatement {
    payload: Vec<u8>,
    bundle: String,
}

fn validate_request(request: &Request) -> Result<(), AttestationVerifyError> {
    if request.protocol != REQUEST_PROTOCOL
        || !valid_repository(&request.github_repository)
        || !request.repository.is_absolute()
        || !is_plain_ref_part(&request.tag)
        || !engineering_assurance::release::is_repository_path(&request.record_path)
        || !engineering_assurance::release::is_repository_path(&request.allowed_signers_path)
        || request.allowed_signers_path.starts_with('-')
        || !is_plain_ref_part(&request.base_branch)
        || !valid_repository(&request.signer_repository)
        || !valid_workflow_path(&request.signer_workflow_path)
        || !request.signer_ref.starts_with("refs/tags/")
        || !valid_ref(&request.signer_ref)
        || !valid_repository(&request.approval_signer_repository)
        || !valid_workflow_path(&request.approval_signer_workflow_path)
        || !request.approval_signer_ref.starts_with("refs/heads/")
        || !valid_ref(&request.approval_signer_ref)
        || !valid_repository(&request.record_publisher_repository)
        || !valid_workflow_path(&request.record_publisher_workflow_path)
        || !request.record_publisher_ref.starts_with("refs/heads/")
        || !valid_ref(&request.record_publisher_ref)
        || request.artifacts.is_empty()
        || request.artifacts.len() > MAX_ARTIFACTS
    {
        return Err(AttestationVerifyError::Invalid);
    }
    let mut names = std::collections::BTreeSet::new();
    if request.artifacts.iter().any(|artifact| {
        artifact.name.is_empty()
            || artifact.name.len() > 1024
            || !artifact.path.is_absolute()
            || !names.insert(artifact.name.as_str())
    }) {
        return Err(AttestationVerifyError::Invalid);
    }
    Ok(())
}

fn read_signed_statement(
    request: &SignedStatementRequest,
) -> Result<ReadStatement, AttestationVerifyError> {
    let payload = read_bounded(&request.payload, MAX_DOCUMENT_BYTES)
        .map_err(|()| AttestationVerifyError::Statement)?;
    let bundle = read_bounded(&request.bundle, MAX_DOCUMENT_BYTES)
        .map_err(|()| AttestationVerifyError::Statement)?;
    let bundle = String::from_utf8(bundle).map_err(|_| AttestationVerifyError::Statement)?;
    Ok(ReadStatement { payload, bundle })
}

fn sha256_file(path: &Path) -> Result<String, ()> {
    let args: [&OsStr; 2] = [OsStr::new("--"), path.as_os_str()];
    let output = process_host::run(
        OsStr::new("/usr/bin/sha256sum"),
        &args,
        ProcessLimits {
            timeout: PROCESS_TIMEOUT,
            max_output_bytes: 1024,
        },
    )
    .map_err(|_| ())?;
    if !output.status.success() {
        return Err(());
    }
    let text = std::str::from_utf8(&output.stdout).map_err(|_| ())?;
    let digest = text.split_ascii_whitespace().next().ok_or(())?;
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(());
    }
    Ok(digest.to_owned())
}

fn sha256_canonical_bytes(canonical: &[u8]) -> Result<String, ()> {
    use std::io::Write as _;

    let mut file = tempfile::NamedTempFile::new().map_err(|_| ())?;
    file.write_all(canonical).map_err(|_| ())?;
    sha256_file(file.path())
}

fn absolute_file(path: &Path, max_bytes: u64) -> Result<PathBuf, ()> {
    if !path.is_absolute() {
        return Err(());
    }
    let metadata = fs::metadata(path).map_err(|_| ())?;
    if !metadata.is_file() || metadata.len() > max_bytes {
        return Err(());
    }
    Ok(path.to_path_buf())
}

fn read_bounded(path: &Path, max_bytes: u64) -> Result<Vec<u8>, ()> {
    absolute_file(path, max_bytes)?;
    let file = File::open(path).map_err(|_| ())?;
    let mut bytes = Vec::new();
    file.take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if u64::try_from(bytes.len()).map_or(true, |len| len > max_bytes) {
        return Err(());
    }
    Ok(bytes)
}

fn resolve_tag_commit(repository: &Path, tag: &str) -> Result<String, AttestationVerifyError> {
    let revision = OsString::from(format!("{tag}^{{commit}}"));
    let args = [
        OsStr::new("-C"),
        repository.as_os_str(),
        OsStr::new("rev-parse"),
        OsStr::new("--verify"),
        OsStr::new("--end-of-options"),
        revision.as_os_str(),
    ];
    let output = run_git(&args, 1024).map_err(|_| AttestationVerifyError::Release)?;
    if !output.status.success() {
        return Err(AttestationVerifyError::Release);
    }
    let commit = String::from_utf8(output.stdout)
        .map_err(|_| AttestationVerifyError::Release)?
        .trim()
        .to_owned();
    if !(commit.len() == 40 || commit.len() == 64)
        || !commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(AttestationVerifyError::Release);
    }
    Ok(commit)
}

fn git_show_base_file(
    repository: &Path,
    commit: &str,
    path: &str,
    max_bytes: u64,
) -> Result<Vec<u8>, ()> {
    if !is_commit_id(commit) {
        return Err(());
    }
    let object = OsString::from(format!("{commit}:{path}"));
    let args = [
        OsStr::new("-C"),
        repository.as_os_str(),
        OsStr::new("show"),
        OsStr::new("--no-ext-diff"),
        OsStr::new("--no-textconv"),
        object.as_os_str(),
    ];
    let output =
        run_git(&args, usize::try_from(max_bytes).unwrap_or(usize::MAX)).map_err(|_| ())?;
    if !output.status.success()
        || u64::try_from(output.stdout.len()).map_or(true, |size| size > max_bytes)
    {
        return Err(());
    }
    Ok(output.stdout)
}

fn run_git(
    arguments: &[&OsStr],
    max_output_bytes: usize,
) -> Result<crate::process_host::CompletedProcess, crate::process_host::ProcessError> {
    let removed = REMOVED_GIT_ENVIRONMENT.map(OsStr::new);
    process_host::run_configured(
        OsStr::new("/usr/bin/git"),
        arguments,
        None,
        &[],
        &removed,
        ProcessLimits {
            timeout: PROCESS_TIMEOUT,
            max_output_bytes,
        },
    )
}

fn protected_base_commit(
    repository: &str,
    repository_path: &Path,
    branch: &str,
) -> Result<String, AttestationVerifyError> {
    let endpoint = github_branch_endpoint(repository, branch);
    let args = [
        OsStr::new("api"),
        OsStr::new(&endpoint),
        OsStr::new("--jq"),
        OsStr::new(".protected, .commit.sha"),
    ];
    let output = process_host::run(
        OsStr::new("/usr/bin/gh"),
        &args,
        ProcessLimits {
            timeout: PROCESS_TIMEOUT,
            max_output_bytes: 1024,
        },
    )
    .map_err(|_| AttestationVerifyError::Unprotected)?;
    if !output.status.success() {
        return Err(AttestationVerifyError::Unprotected);
    }
    let response = std::str::from_utf8(&output.stdout)
        .map_err(|_| AttestationVerifyError::Unprotected)?
        .trim();
    let mut lines = response.lines();
    if lines.next() != Some("true") {
        return Err(AttestationVerifyError::Unprotected);
    }
    let protected_commit = lines
        .next()
        .filter(|commit| is_commit_id(commit))
        .ok_or(AttestationVerifyError::Unprotected)?;
    if lines.next().is_some() {
        return Err(AttestationVerifyError::Unprotected);
    }
    let local_commit = local_base_commit(repository_path, branch)?;
    if protected_commit != local_commit {
        return Err(AttestationVerifyError::Unprotected);
    }
    Ok(protected_commit.to_owned())
}

fn local_base_commit(repository: &Path, branch: &str) -> Result<String, AttestationVerifyError> {
    let revision = OsString::from(format!("refs/remotes/origin/{branch}^{{commit}}"));
    let args = [
        OsStr::new("-C"),
        repository.as_os_str(),
        OsStr::new("rev-parse"),
        OsStr::new("--verify"),
        OsStr::new("--end-of-options"),
        revision.as_os_str(),
    ];
    let output = run_git(&args, 1024).map_err(|_| AttestationVerifyError::Unprotected)?;
    if !output.status.success() {
        return Err(AttestationVerifyError::Unprotected);
    }
    let commit = String::from_utf8(output.stdout)
        .map_err(|_| AttestationVerifyError::Unprotected)?
        .trim()
        .to_owned();
    if !is_commit_id(&commit) {
        return Err(AttestationVerifyError::Unprotected);
    }
    Ok(commit)
}

fn is_commit_id(value: &str) -> bool {
    (value.len() == 40 || value.len() == 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn github_branch_endpoint(repository: &str, branch: &str) -> String {
    let encoded_branch = branch.replace('/', "%2F");
    format!("repos/{repository}/branches/{encoded_branch}")
}

fn checkout_matches_repository(
    repository_path: &Path,
    expected_repository: &str,
) -> Result<bool, AttestationVerifyError> {
    let args = [
        OsStr::new("-C"),
        repository_path.as_os_str(),
        OsStr::new("remote"),
        OsStr::new("get-url"),
        OsStr::new("origin"),
    ];
    let output = run_git(&args, 2048).map_err(|_| AttestationVerifyError::Invalid)?;
    if !output.status.success() {
        return Ok(false);
    }
    let origin = std::str::from_utf8(&output.stdout)
        .map_err(|_| AttestationVerifyError::Invalid)?
        .trim();
    let repository = origin
        .strip_prefix(concat!("https:", "//github.com/"))
        .or_else(|| origin.strip_prefix(concat!("http:", "//github.com/")))
        .or_else(|| origin.strip_prefix("git@github.com:"))
        .or_else(|| origin.strip_prefix("ssh://git@github.com/"))
        .map(|path| path.strip_suffix(".git").unwrap_or(path));
    Ok(repository.is_some_and(|repository| repository.eq_ignore_ascii_case(expected_repository)))
}

fn is_plain_ref_part(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !value.starts_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/'))
        && !value.contains("..")
        && !value.contains("//")
}

fn valid_repository(value: &str) -> bool {
    let mut parts = value.split('/');
    parts.next().is_some_and(is_component)
        && parts.next().is_some_and(is_component)
        && parts.next().is_none()
}

fn is_component(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        && value != "."
        && value != ".."
}

fn valid_workflow_path(value: &str) -> bool {
    value.starts_with(".github/workflows/")
        && value.len() > ".github/workflows/".len()
        && !value.contains("..")
        && !value.ends_with('/')
        && !value.bytes().any(|byte| byte.is_ascii_control())
}

fn valid_ref(value: &str) -> bool {
    let mut parts = value.split('/');
    matches!(parts.next(), Some("refs"))
        && parts
            .next()
            .is_some_and(|part| matches!(part, "heads" | "tags"))
        && parts.all(is_component)
}

#[cfg(test)]
mod tests {
    use super::*;
    use engineering_assurance::approval_gate::GateResult;
    use ix_trace_rs::trace;

    fn request_json() -> serde_json::Value {
        serde_json::json!({
            "protocol": REQUEST_PROTOCOL,
            "github_repository": "fictional/project",
            "repository": "/tmp/fictional-project",
            "tag": "v1.2.3",
            "record_path": "records/release.md",
            "allowed_signers_path": ".engineering-assurance/allowed_signers",
            "base_branch": "main",
            "signer_repository": "agent-ix/engineering-assurance",
            "signer_workflow_path": ".github/workflows/release-attestation.yml",
            "signer_ref": "refs/tags/v0.9.0",
            "approval_signer_repository": "fictional/project",
            "approval_signer_workflow_path": ".github/workflows/review-gate.yml",
            "approval_signer_ref": "refs/heads/main",
            "record_publisher_repository": "fictional/project",
            "record_publisher_workflow_path": ".github/workflows/record-publish.yml",
            "record_publisher_ref": "refs/heads/main",
            "transparency": "public_log",
            "artifacts": [{"name":"release.tar.gz", "path":"/tmp/release.tar.gz"}],
            "statements": {
                "execution_provenance":{"payload":"/tmp/execution-provenance.json", "bundle":"/tmp/execution-provenance.bundle"},
                "provenance":{"payload":"/tmp/provenance.json", "bundle":"/tmp/provenance.bundle"},
                "sbom":{"payload":"/tmp/sbom.json", "bundle":"/tmp/sbom.bundle"},
                "test_result":{"payload":"/tmp/test-result.json", "bundle":"/tmp/test-result.bundle"},
                "summary":{"payload":"/tmp/summary.json", "bundle":"/tmp/summary.bundle"}
            }
        })
    }

    #[trace("TC-1055", "FR-067-AC-6", "FR-067-AC-8")]
    #[test]
    fn source_lookup_requires_the_tagged_commit_to_be_the_unique_merge_result() {
        let commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let head = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let squash = serde_json::json!([{
            "number": 42,
            "state": "closed",
            "merged_at": "2026-11-27T12:00:00Z",
            "merge_commit_sha": commit,
            "head": {"sha": head}
        }]);
        assert_eq!(
            merged_pull_request_for_commit(&squash, commit),
            Ok(Some(MergedPullRequest {
                number: 42,
                head_commit: head.to_owned(),
            }))
        );

        let direct_push = serde_json::json!([]);
        assert_eq!(
            merged_pull_request_for_commit(&direct_push, commit),
            Ok(None)
        );
        assert_eq!(
            select_merged_pull_request(&direct_push, commit)
                .expect_err("direct push has no merged PR")
                .code(),
            "release_roles_no_pull_request"
        );

        let rebased_non_tip = serde_json::json!([{
            "number": 42,
            "state": "closed",
            "merged_at": "2026-11-27T12:00:00Z",
            "merge_commit_sha": "cccccccccccccccccccccccccccccccccccccccc",
            "head": {"sha": head}
        }]);
        assert_eq!(
            merged_pull_request_for_commit(&rebased_non_tip, commit),
            Ok(None)
        );
        assert_eq!(
            select_merged_pull_request(&rebased_non_tip, commit)
                .expect_err("non-tip rebased commit has no merged PR result")
                .code(),
            "release_roles_no_pull_request"
        );

        let ambiguous = serde_json::json!([
            squash[0].clone(),
            {
                "number": 43,
                "state": "closed",
                "merged_at": "2026-11-27T13:00:00Z",
                "merge_commit_sha": commit,
                "head": {"sha": head}
            }
        ]);
        assert!(merged_pull_request_for_commit(&ambiguous, commit).is_err());
    }

    #[trace("TC-1057", "FR-067-AC-4", "FR-067-AC-5")]
    #[test]
    fn approval_result_names_unstaffed_roles_and_pending_statements() {
        let pending = ApprovalStatementResult {
            status: ApprovalStatementStatus::Unverifiable,
            reason: Some(ApprovalStatementReason::StatementsNotRetained),
            role_verdict: GateResult {
                protocol: "engineering-assurance.approval-gate/v1",
                status: GateStatus::Unsatisfied,
                required_roles: vec!["release_owner".to_owned()],
                assignments: Vec::new(),
                missing: Vec::new(),
                error: None,
                unavailable_accounts: Vec::new(),
            },
        };
        assert_eq!(
            classify_approval_result(&pending)
                .expect_err("not-yet-retained statements are retryable")
                .code(),
            "release_roles_statements_pending"
        );

        let unstaffed = ApprovalStatementResult {
            status: ApprovalStatementStatus::Unverifiable,
            reason: Some(ApprovalStatementReason::InputUnreadable),
            role_verdict: GateResult {
                protocol: "engineering-assurance.approval-gate/v1",
                status: GateStatus::Unavailable,
                required_roles: vec!["release_owner".to_owned()],
                assignments: Vec::new(),
                missing: Vec::new(),
                error: Some(GateErrorCode::ApprovalRoleUnstaffed),
                unavailable_accounts: Vec::new(),
            },
        };
        assert_eq!(
            classify_approval_result(&unstaffed)
                .expect_err("an unstaffed required role is named")
                .code(),
            "approval_role_unstaffed"
        );
    }

    #[trace("TC-679", "FR-068-AC-1", "FR-068-AC-3")]
    #[test]
    fn attestation_verify_request_rejects_unknown_members_before_io() {
        let mut request = request_json();
        request["unexpected"] = serde_json::Value::Bool(true);
        let error = execute(request.to_string().as_bytes()).expect_err("closed request rejected");
        assert!(matches!(error, AttestationVerifyError::Invalid));
    }

    #[trace("TC-1054", "FR-069-AC-2")]
    #[test]
    fn attestation_verify_request_rejects_absent_execution_provenance_before_io() {
        let mut request = request_json();
        request["statements"]
            .as_object_mut()
            .expect("statement set object")
            .remove("execution_provenance");
        let error = match parse_request(request.to_string().as_bytes()) {
            Ok(_) => panic!("execution provenance is required"),
            Err(error) => error,
        };
        assert!(matches!(error, AttestationVerifyError::Invalid));
    }

    #[trace("TC-680", "FR-055-AC-5", "FR-068-AC-1")]
    #[test]
    fn policy_bytes_come_from_the_named_base_ref_not_the_worktree() {
        let root = tempfile::tempdir().expect("temporary repository");
        let repository = root.path();
        run_git(
            &[
                OsStr::new("-C"),
                repository.as_os_str(),
                OsStr::new("init"),
                OsStr::new("--initial-branch=main"),
            ],
            1024,
        )
        .expect("git init launches");
        let policy_path = repository.join(".engineering-assurance/allowed_signers");
        fs::create_dir_all(policy_path.parent().expect("policy parent")).expect("policy directory");
        fs::write(&policy_path, b"protected base policy\n").expect("base policy");
        run_git(
            &[
                OsStr::new("-C"),
                repository.as_os_str(),
                OsStr::new("add"),
                OsStr::new(".engineering-assurance/allowed_signers"),
            ],
            1024,
        )
        .expect("git add launches");
        run_git(
            &[
                OsStr::new("-C"),
                repository.as_os_str(),
                OsStr::new("-c"),
                OsStr::new("user.name=fixture"),
                OsStr::new("-c"),
                OsStr::new("user.email=fixture@example.invalid"),
                OsStr::new("commit"),
                OsStr::new("-m"),
                OsStr::new("base policy"),
            ],
            1024,
        )
        .expect("git commit launches");
        run_git(
            &[
                OsStr::new("-C"),
                repository.as_os_str(),
                OsStr::new("update-ref"),
                OsStr::new("refs/remotes/origin/main"),
                OsStr::new("HEAD"),
            ],
            1024,
        )
        .expect("base ref fixture");
        let commit = local_base_commit(repository, "main").expect("base commit resolves");
        fs::write(&policy_path, b"untrusted worktree replacement\n")
            .expect("candidate replacement");
        assert_eq!(
            git_show_base_file(
                repository,
                &commit,
                ".engineering-assurance/allowed_signers",
                1024,
            )
            .expect("base policy reads"),
            b"protected base policy\n"
        );
    }

    #[trace("TC-681", "FR-055-AC-6", "FR-068-AC-1")]
    #[test]
    fn branch_protection_endpoint_encodes_branch_slashes() {
        assert_eq!(
            github_branch_endpoint("fictional/project", "release/v1"),
            "repos/fictional/project/branches/release%2Fv1"
        );
    }

    #[trace("TC-682", "FR-068-AC-1")]
    #[test]
    fn policy_source_repository_must_match_checkout_origin() {
        let root = tempfile::tempdir().expect("temporary repository");
        let repository = root.path();
        run_git(
            &[OsStr::new("-C"), repository.as_os_str(), OsStr::new("init")],
            1024,
        )
        .expect("git init launches");
        run_git(
            &[
                OsStr::new("-C"),
                repository.as_os_str(),
                OsStr::new("remote"),
                OsStr::new("add"),
                OsStr::new("origin"),
                OsStr::new(concat!("https:", "//github.com/fictional/project.git")),
            ],
            1024,
        )
        .expect("origin fixture");
        assert!(
            checkout_matches_repository(repository, "fictional/project")
                .expect("repository identity reads")
        );
        assert!(
            !checkout_matches_repository(repository, "other/project")
                .expect("mismatched identity reads")
        );
    }
}
