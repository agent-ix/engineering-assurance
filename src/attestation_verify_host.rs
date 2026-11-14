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
    approval_policy::ApprovalPolicy,
    attestation_signing::{
        SigstoreBundleVerifier, TransparencyRequirement, WorkflowIdentityDocument, WorkflowRole,
    },
    attestation_verify::{
        ArtifactSubject, ExpectedRelease, ReleaseInputs, SignedStatement, VerificationConditions,
        verify_release_attestation_set,
    },
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::process_host::{self, ProcessLimits};

const REQUEST_PROTOCOL: &str = "engineering-assurance.attestation-verify-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.attestation-verify-result/v1";
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
    trusted_root: PathBuf,
    signer_repository: String,
    signer_workflow_path: String,
    signer_ref: String,
    transparency: String,
    artifacts: Vec<ArtifactRequest>,
    statements: StatementSetRequest,
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

#[derive(Debug, Error)]
pub(crate) enum AttestationVerifyError {
    #[error("attestation-verify request is not readable JSON")]
    Unreadable,
    #[error("attestation-verify request is not an exact valid request")]
    Invalid,
    #[error("attestation-verify policy is not a valid protected policy document")]
    Policy,
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
            Self::Unprotected => "policy_branch_unprotected",
            Self::Release => "release_not_verified",
            Self::Statement => "attestation_set_invalid",
            Self::Encoding => "attestation_verify_result_encoding_failed",
        }
    }
}

/// Verify one release-attestation set and the signed release tag it describes.
pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, AttestationVerifyError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| AttestationVerifyError::Unreadable)?;
    let request: Request =
        serde_json::from_value(value).map_err(|_| AttestationVerifyError::Invalid)?;
    validate_request(&request)?;
    if !checkout_matches_repository(&request.repository, &request.github_repository)? {
        return Err(AttestationVerifyError::Invalid);
    }
    let base_commit = protected_base_commit(
        &request.github_repository,
        &request.repository,
        &request.base_branch,
    )?;

    let allowed_signers_bytes = git_show_base_file(
        &request.repository,
        &base_commit,
        &request.allowed_signers_path,
        1024 * 1024,
    )
    .map_err(|_| AttestationVerifyError::Policy)?;
    let mut allowed_signers =
        tempfile::NamedTempFile::new().map_err(|_| AttestationVerifyError::Policy)?;
    std::io::Write::write_all(&mut allowed_signers, &allowed_signers_bytes)
        .map_err(|_| AttestationVerifyError::Policy)?;
    let identity_bytes = git_show_base_file(
        &request.repository,
        &base_commit,
        ".engineering-assurance/workflow-identities.json",
        MAX_DOCUMENT_BYTES,
    )
    .map_err(|_| AttestationVerifyError::Policy)?;
    let identities = WorkflowIdentityDocument::parse(&identity_bytes)
        .map_err(|_| AttestationVerifyError::Policy)?;
    let role_bytes = git_show_base_file(
        &request.repository,
        &base_commit,
        ".engineering-assurance/approval-policy.json",
        MAX_DOCUMENT_BYTES,
    )
    .map_err(|_| AttestationVerifyError::Policy)?;
    let role_policy: ApprovalPolicy =
        serde_json::from_slice(&role_bytes).map_err(|_| AttestationVerifyError::Policy)?;
    role_policy
        .validate()
        .map_err(|_| AttestationVerifyError::Policy)?;

    let transparency = match request.transparency.as_str() {
        "public_log" => TransparencyRequirement::PublicLog,
        "timestamp_only" => TransparencyRequirement::TimestampOnly,
        _ => return Err(AttestationVerifyError::Invalid),
    };
    let signer_policy = identities
        .signer_policy(
            WorkflowRole::ReleaseAttester,
            &request.signer_repository,
            &request.signer_workflow_path,
            &request.signer_ref,
            transparency,
        )
        .ok_or(AttestationVerifyError::Policy)?;
    let source_policy = identities
        .source_policy(
            WorkflowRole::ReleaseAttester,
            &request.signer_repository,
            &request.signer_workflow_path,
            &request.signer_ref,
        )
        .ok_or(AttestationVerifyError::Policy)?;

    let release_request = serde_json::json!({
        "protocol": "engineering-assurance.release-verify-request/v1",
        "repository": &request.repository,
        "tag": &request.tag,
        "record_path": &request.record_path,
        "allowed_signers": allowed_signers.path().to_string_lossy(),
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
    let commit = resolve_tag_commit(&request.repository, &request.tag)?;

    let artifacts = request
        .artifacts
        .iter()
        .map(|artifact| {
            let path = absolute_file(&artifact.path, MAX_ARTIFACT_BYTES)
                .map_err(|_| AttestationVerifyError::Invalid)?;
            Ok(ArtifactSubject {
                name: artifact.name.clone(),
                sha256: sha256_file(&path).map_err(|_| AttestationVerifyError::Statement)?,
            })
        })
        .collect::<Result<Vec<_>, AttestationVerifyError>>()?;

    let provenance = read_signed_statement(&request.statements.provenance)?;
    let sbom = read_signed_statement(&request.statements.sbom)?;
    let test_result = read_signed_statement(&request.statements.test_result)?;
    let summary = read_signed_statement(&request.statements.summary)?;
    let input_digests = [
        canonical_digest(&provenance.payload)?,
        canonical_digest(&sbom.payload)?,
        canonical_digest(&test_result.payload)?,
    ];
    let repository = request.github_repository.clone();
    let verifier_id = format!(
        "{}{}github.com/{}/{}@{}",
        "https:", "//", request.signer_repository, request.signer_workflow_path, request.signer_ref
    );
    let policy_uri = format!(
        "{}{}github.com/{}/blob/{}/.engineering-assurance/approval-policy.json",
        "https:", "//", repository, request.base_branch
    );
    let expected = ExpectedRelease {
        repository,
        tag: request.tag.clone(),
        commit: commit.clone(),
        policy_uri,
        verifier_id,
        artifacts,
        input_digests,
    };
    let trusted_root = sigstore_verify::trust_root::TrustedRoot::from_file(&request.trusted_root)
        .map_err(|_| AttestationVerifyError::Policy)?;
    let verifier =
        SigstoreBundleVerifier::new(&trusted_root).map_err(|_| AttestationVerifyError::Policy)?;
    verify_release_attestation_set(
        &verifier,
        &signer_policy,
        &source_policy,
        &expected,
        VerificationConditions {
            release_verified: true,
            roles_satisfied: true,
            inputs_verified: true,
        },
        ReleaseInputs {
            provenance: SignedStatement {
                payload: &provenance.payload,
                bundle_json: &provenance.bundle,
            },
            sbom: SignedStatement {
                payload: &sbom.payload,
                bundle_json: &sbom.bundle,
            },
            test_result: SignedStatement {
                payload: &test_result.payload,
                bundle_json: &test_result.bundle,
            },
        },
        SignedStatement {
            payload: &summary.payload,
            bundle_json: &summary.bundle,
        },
    )
    .map_err(|_| AttestationVerifyError::Statement)?;

    let mut result = serde_json::to_vec(&ResultLine {
        protocol: RESULT_PROTOCOL,
        status: "verified",
        tag: request.tag,
        commit,
    })
    .map_err(|_| AttestationVerifyError::Encoding)?;
    result.push(b'\n');
    Ok(result)
}

struct ReadStatement {
    payload: Vec<u8>,
    bundle: String,
}

fn validate_request(request: &Request) -> Result<(), AttestationVerifyError> {
    if request.protocol != REQUEST_PROTOCOL
        || !valid_repository(&request.github_repository)
        || !request.repository.is_absolute()
        || !request.trusted_root.is_absolute()
        || !is_plain_ref_part(&request.tag)
        || !engineering_assurance::release::is_repository_path(&request.record_path)
        || !engineering_assurance::release::is_repository_path(&request.allowed_signers_path)
        || request.allowed_signers_path.starts_with('-')
        || !is_plain_ref_part(&request.base_branch)
        || !valid_repository(&request.signer_repository)
        || !valid_workflow_path(&request.signer_workflow_path)
        || !request.signer_ref.starts_with("refs/tags/")
        || !valid_ref(&request.signer_ref)
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
        .map_err(|_| AttestationVerifyError::Statement)?;
    let bundle = read_bounded(&request.bundle, MAX_DOCUMENT_BYTES)
        .map_err(|_| AttestationVerifyError::Statement)?;
    let bundle = String::from_utf8(bundle).map_err(|_| AttestationVerifyError::Statement)?;
    Ok(ReadStatement { payload, bundle })
}

fn canonical_digest(bytes: &[u8]) -> Result<String, AttestationVerifyError> {
    let statement: engineering_assurance::attestation_signing::StatementV1 =
        serde_json::from_slice(bytes).map_err(|_| AttestationVerifyError::Statement)?;
    let canonical = statement
        .canonical_bytes()
        .map_err(|_| AttestationVerifyError::Statement)?;
    let mut file = tempfile::NamedTempFile::new().map_err(|_| AttestationVerifyError::Statement)?;
    std::io::Write::write_all(&mut file, &canonical)
        .map_err(|_| AttestationVerifyError::Statement)?;
    let digest = sha256_file(file.path()).map_err(|_| AttestationVerifyError::Statement)?;
    Ok(digest)
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
            "trusted_root": "/tmp/trusted-root.json",
            "signer_repository": "agent-ix/engineering-assurance",
            "signer_workflow_path": ".github/workflows/release-attestation.yml",
            "signer_ref": "refs/tags/v0.9.0",
            "transparency": "public_log",
            "artifacts": [{"name":"release.tar.gz", "path":"/tmp/release.tar.gz"}],
            "statements": {
                "provenance":{"payload":"/tmp/provenance.json", "bundle":"/tmp/provenance.bundle"},
                "sbom":{"payload":"/tmp/sbom.json", "bundle":"/tmp/sbom.bundle"},
                "test_result":{"payload":"/tmp/test-result.json", "bundle":"/tmp/test-result.bundle"},
                "summary":{"payload":"/tmp/summary.json", "bundle":"/tmp/summary.bundle"}
            }
        })
    }

    #[trace("TC-679", "FR-068-AC-1", "FR-068-AC-3")]
    #[test]
    fn attestation_verify_request_rejects_unknown_members_before_io() {
        let mut request = request_json();
        request["unexpected"] = serde_json::Value::Bool(true);
        let error = execute(request.to_string().as_bytes()).expect_err("closed request rejected");
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
