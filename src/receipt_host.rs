// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded Git-note command adapter for receipt bundles (FR-076).

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use engineering_assurance::{
    approval_gate::parse_policy,
    attestation_signing::{
        SigstoreBundleVerifier, SigstoreSigner, TransparencyRequirement, WorkflowIdentityDocument,
        WorkflowRole,
    },
    producer_execution::private_git_store::{
        GitPrivateStore, HttpsGitDestinationResolver, RecordPublisherIdentity,
    },
    receipt::{
        ExpectedReceiptSubject, MissingReceipt, RECEIPT_NOTES_REF, ReceiptBundleSet,
        ReceiptHistoryEntry, ReceiptNoteError, ReceiptStatus, ReceiptVerificationContext,
        ReceiptVerifyResult, UnverifiableReason, VERIFY_RESULT_PROTOCOL, find_missing_receipts,
        is_valid_verify_result, receipt_status_allows_idempotent_skip, verify_receipt_groups,
    },
    receipt_record::{
        RECEIPT_GATE_RESULTS_PROTOCOL, ReceiptGateResults, ReceiptInput, ReceiptInputReader,
        ReceiptPublishContext, ReceiptRecordInput, build_receipt_statements, publish_receipt,
        verified_receipt_approvals,
    },
    record_sink::{
        config::{PolicySource, SinkConfig, SinkKind, Visibility, resolve_sink_for_repository},
        form::RecordId,
        sink::{
            ApprovalStatementLookup, ApprovalStatementReadError, ApprovalStatementReader,
            LogRejected, Ports, PublicLogClient, RetainedApprovalStatement, SinkLayer,
        },
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::tag_receipt_host::{
    ProductionVerifiers, TagFindingLine, TagReceiptHostError, repository_findings,
};

const REQUEST_PROTOCOL: &str = "engineering-assurance.receipt-note-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.receipt-note-result/v1";
const MAX_REQUEST_BYTES: usize = 2 * 1024 * 1024;
const MISSING_REQUEST_PROTOCOL: &str = "engineering-assurance.receipt-missing-request/v1";
const MISSING_RESULT_PROTOCOL: &str = "engineering-assurance.receipt-missing-result/v1";
const VERIFY_REQUEST_PROTOCOL: &str = "engineering-assurance.receipt-verify-request/v1";
const RECORD_REQUEST_PROTOCOL: &str = "engineering-assurance.receipt-record-request/v1";
const MAX_POLICY_BYTES: usize = 1024 * 1024;
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
    operation: Operation,
    repository: PathBuf,
    commit: String,
    #[serde(default)]
    bundles: Option<ReceiptBundleSet>,
    #[serde(default)]
    verification: Option<ExistingVerificationRequest>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExistingVerificationRequest {
    github_repository: String,
    base_branch: String,
    merged_pull_request: String,
    receipt_signer_repository: String,
    receipt_signer_workflow_path: String,
    receipt_signer_ref: String,
    approval_signer_repository: String,
    approval_signer_workflow_path: String,
    approval_signer_ref: String,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Operation {
    Read,
    Store,
}

#[derive(Serialize)]
struct ResultLine {
    protocol: &'static str,
    operation: &'static str,
    commit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    bundles: Option<Vec<ReceiptBundleSet>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<&'static str>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MissingRequest {
    protocol: String,
    repository: PathBuf,
    github_repository: String,
    base_branch: String,
    receipt_signer_repository: String,
    receipt_signer_workflow_path: String,
    receipt_signer_ref: String,
    approval_signer_repository: String,
    approval_signer_workflow_path: String,
    approval_signer_ref: String,
    merged_pull_requests: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptPolicy {
    protocol: String,
    gates: Vec<ReceiptGate>,
    adopted_at: String,
    grace_minutes: u16,
    /// The optional `tag_receipts` member (FR-081), validated by the tag
    /// receipt policy reader.
    #[serde(default)]
    tag_receipts: Option<Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptGate {
    name: String,
    argv: Vec<String>,
}

/// A commit finding as the command prints it: FR-077's members and `kind`.
#[derive(Serialize)]
struct ChangeFindingLine<'a> {
    kind: &'static str,
    #[serde(flatten)]
    finding: &'a MissingReceipt,
}

#[derive(Serialize)]
struct MissingResultLine<'a> {
    protocol: &'static str,
    missing: Vec<ChangeFindingLine<'a>>,
    failed_receipts: Vec<ChangeFindingLine<'a>>,
    /// The tag findings of FR-082, each with `kind` `tag`.
    tag_findings: Vec<TagFindingLine>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MissingResultLineOwned {
    protocol: String,
    missing: Vec<Value>,
    #[serde(rename = "failed_receipts")]
    _failed_receipts: Vec<Value>,
    #[serde(default)]
    tag_findings: Vec<TagFindingStatus>,
}

#[derive(Deserialize)]
struct TagFindingStatus {
    status: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifyRequest {
    protocol: String,
    repository: PathBuf,
    github_repository: String,
    base_branch: String,
    commit: String,
    merged_pull_request: String,
    receipt_signer_repository: String,
    receipt_signer_workflow_path: String,
    receipt_signer_ref: String,
    approval_signer_repository: String,
    approval_signer_workflow_path: String,
    approval_signer_ref: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordRequest {
    protocol: String,
    repository: PathBuf,
    github_repository: String,
    base_branch: String,
    commit: String,
    merged_pull_request: Option<String>,
    gate_results: PathBuf,
    test_result: PathBuf,
    record_id: String,
    receipt_signer_repository: String,
    receipt_signer_workflow_path: String,
    receipt_signer_ref: String,
    record_publisher_repository: String,
    record_publisher_workflow_path: String,
    record_publisher_ref: String,
    approval_signer_repository: String,
    approval_signer_workflow_path: String,
    approval_signer_ref: String,
}

#[derive(Debug, Error)]
pub(crate) enum ReceiptHostError {
    #[error("receipt-note request is unreadable")]
    Unreadable,
    #[error("receipt-note request is invalid")]
    Invalid,
    #[error("receipt note is too large")]
    TooLarge,
    #[error("receipt note is malformed")]
    NoteInvalid,
    #[error("receipt note could not be read or written")]
    Git,
    #[error("receipt note already exists for the commit")]
    NoteConflict,
    #[error("receipt-note result could not be encoded")]
    Encoding,
    #[error("receipt-missing request is invalid")]
    MissingRequest,
    #[error("receipt-verify request is unreadable")]
    VerifyUnreadable,
    #[error("receipt-verify request is invalid")]
    VerifyInvalid,
    #[error("receipt policy adopted_at is not on the protected base first-parent history")]
    AdoptedAtInvalid,
    #[error("receipt-record request or protected policy is invalid")]
    RecordRequest,
    #[error("receipt-record input, evidence, or publication failed: {0}")]
    Record(#[from] engineering_assurance::receipt_record::ReceiptRecordError),
    #[error("tag receipts could not be compared: {0}")]
    Tag(TagReceiptHostError),
}

impl ReceiptHostError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Unreadable => "receipt_note_request_unreadable",
            Self::Invalid => "receipt_note_request_invalid",
            Self::TooLarge => "receipt_note_too_large",
            Self::NoteInvalid => "receipt_note_invalid",
            Self::Git => "receipt_note_git_failed",
            Self::NoteConflict => "receipt_note_conflict",
            Self::Encoding => "receipt_note_result_encoding_failed",
            Self::MissingRequest => "receipt_missing_request_invalid",
            Self::VerifyUnreadable => "receipt_verify_request_unreadable",
            Self::VerifyInvalid => "receipt_verify_request_invalid",
            Self::AdoptedAtInvalid => "receipt_policy_adopted_at_invalid",
            Self::RecordRequest => "receipt_record_request_invalid",
            Self::Record(_) => "receipt_record_failed",
            Self::Tag(error) => error.code(),
        }
    }
}

/// Produce and sign one receipt from protected policy, retained PR inputs,
/// verified approvals, gate outputs, and Quoin's Test Result export.
pub(crate) fn execute_record(bytes: &[u8]) -> Result<Vec<u8>, ReceiptHostError> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(ReceiptHostError::RecordRequest);
    }
    let request: RecordRequest =
        serde_json::from_slice(bytes).map_err(|_| ReceiptHostError::RecordRequest)?;
    if request.protocol != RECORD_REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !request.repository.is_dir()
        || !valid_github_repository(&request.github_repository)
        || !valid_branch(&request.base_branch)
        || !is_full_object_id(&request.commit)
        || !valid_workflow_ref(&request.receipt_signer_ref)
        || !valid_workflow_ref(&request.record_publisher_ref)
        || !valid_workflow_ref(&request.approval_signer_ref)
    {
        return Err(ReceiptHostError::RecordRequest);
    }
    let record_id = RecordId::try_from(request.record_id.clone())
        .map_err(|_| ReceiptHostError::RecordRequest)?;
    let protected = load_protected_record_policy(&request)?;
    let trusted_root_json = match protected.transparency {
        TransparencyRequirement::PublicLog => {
            sigstore_verify::trust_root::SIGSTORE_PRODUCTION_TRUSTED_ROOT
        }
        TransparencyRequirement::TimestampOnly => {
            sigstore_verify::trust_root::SIGSTORE_GITHUB_TRUSTED_ROOT
        }
    };
    let trusted_root = sigstore_verify::trust_root::TrustedRoot::from_json(trusted_root_json)
        .map_err(|_| ReceiptHostError::RecordRequest)?;
    let verifier =
        SigstoreBundleVerifier::new(&trusted_root).map_err(|_| ReceiptHostError::RecordRequest)?;
    let signer = SigstoreSigner::new();
    let resolver = HttpsGitDestinationResolver::new(concat!("https:", "//github.com"))
        .map_err(|_| ReceiptHostError::RecordRequest)?;
    let publisher = RecordPublisherIdentity {
        repository: request.record_publisher_repository.clone(),
        workflow_path: request.record_publisher_workflow_path.clone(),
        reference: request.record_publisher_ref.clone(),
    };
    let store = GitPrivateStore::new(
        &resolver,
        &signer,
        &verifier,
        &protected.source,
        &publisher,
        &request.github_repository,
        "git",
    )
    .map_err(|_| ReceiptHostError::RecordRequest)?;
    let evidence = load_record_evidence(&request, &protected, &store, &verifier)?;
    publish_record_evidence(
        &request, &record_id, &protected, &signer, &verifier, &store, evidence,
    )
}

struct ReceiptRecordEvidence {
    gate_results: ReceiptGateResults,
    test_result: Value,
    subject: ExpectedReceiptSubject,
    receipt_input: Option<ReceiptInput>,
    approvals: Vec<(
        String,
        engineering_assurance::change_record::ApprovalDecision,
    )>,
}

fn load_record_evidence(
    request: &RecordRequest,
    protected: &ProtectedReceiptPolicy,
    store: &GitPrivateStore<'_>,
    verifier: &SigstoreBundleVerifier,
) -> Result<ReceiptRecordEvidence, ReceiptHostError> {
    let gate_results: ReceiptGateResults =
        serde_json::from_slice(&read_bounded(&request.gate_results, 1024 * 1024)?)
            .map_err(|_| ReceiptHostError::RecordRequest)?;
    if gate_results.protocol != RECEIPT_GATE_RESULTS_PROTOCOL
        || gate_results.commit != request.commit
        || gate_results.results.is_empty()
    {
        return Err(ReceiptHostError::RecordRequest);
    }
    let test_result: Value =
        serde_json::from_slice(&read_bounded(&request.test_result, 4 * 1024 * 1024)?)
            .map_err(|_| ReceiptHostError::RecordRequest)?;
    let pull_request = request.merged_pull_request.clone().unwrap_or_default();
    if !pull_request.is_empty() && !valid_pull_request(&pull_request, &request.github_repository) {
        return Err(ReceiptHostError::RecordRequest);
    }
    let tree_output = git_output(
        &request.repository,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{}^{{tree}}", request.commit),
        ],
    )
    .map_err(|_| ReceiptHostError::RecordRequest)?;
    let tree = String::from_utf8(tree_output)
        .map_err(|_| ReceiptHostError::RecordRequest)?
        .trim()
        .to_owned();
    if !is_full_object_id(&tree) {
        return Err(ReceiptHostError::RecordRequest);
    }
    let subject = ExpectedReceiptSubject {
        repository: request.github_repository.clone(),
        commit: request.commit.clone(),
        tree,
        merged_pull_request: pull_request.clone(),
    };
    let private_destination = protected
        .selection
        .private_store_destination
        .as_deref()
        .ok_or(ReceiptHostError::RecordRequest)?;
    let receipt_input = if pull_request.is_empty() {
        None
    } else {
        store
            .read_receipt_input(private_destination, &subject.repository, &pull_request)
            .ok()
            .flatten()
            .and_then(|bytes| ReceiptInput::parse(&bytes, &pull_request).ok())
    };
    let approvals = verified_receipt_approvals(
        store,
        private_destination,
        &subject,
        &protected.approval_signer,
        &protected.role_policy,
        verifier,
    );
    Ok(ReceiptRecordEvidence {
        gate_results,
        test_result,
        subject,
        receipt_input,
        approvals,
    })
}

fn publish_record_evidence(
    request: &RecordRequest,
    record_id: &RecordId,
    protected: &ProtectedReceiptPolicy,
    signer: &SigstoreSigner,
    verifier: &SigstoreBundleVerifier,
    store: &GitPrivateStore<'_>,
    evidence: ReceiptRecordEvidence,
) -> Result<Vec<u8>, ReceiptHostError> {
    let created_at = current_seconds_timestamp();
    let workflow_id = format!(
        concat!("https:", "//github.com/{}/{}@{}"),
        request.receipt_signer_repository,
        request.receipt_signer_workflow_path,
        request.receipt_signer_ref
    );
    let policy_uri = format!(
        concat!(
            "https:",
            "//github.com/{}/blob/{}/.engineering-assurance/approval-policy.json"
        ),
        request.github_repository, protected.source.base_commit
    );
    let expected_gate_names = protected
        .expected_gate_names
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let statements = build_receipt_statements(ReceiptRecordInput {
        subject: &evidence.subject,
        gate_results: &evidence.gate_results,
        expected_gate_names: &expected_gate_names,
        test_result: &evidence.test_result,
        receipt_input: evidence.receipt_input.as_ref(),
        role_policy: &protected.role_policy,
        approvals: evidence.approvals,
        record_id: record_id.clone(),
        created_at: &created_at,
        verifier_id: &workflow_id,
        policy_uri: &policy_uri,
    })?;
    let log = HttpReceiptLog;
    let ports = Ports {
        store,
        lookup: &GitHubVisibility,
        signer,
        client: &log,
    };
    let layer = SinkLayer::new(&protected.selection, &request.github_repository, ports);
    let result = publish_receipt(
        &layer,
        &ReceiptPublishContext {
            verifier,
            signer_policy: &protected.receipt_signer,
            source_policy: &protected.receipt_source,
            subject: &evidence.subject,
            record_id,
            created_at: &created_at,
        },
        statements,
    )?;
    let mut output = serde_json::to_vec(&result).map_err(|_| ReceiptHostError::RecordRequest)?;
    output.push(b'\n');
    Ok(output)
}

struct ProtectedReceiptPolicy {
    source: ReceiptPolicySource,
    expected_gate_names: Vec<String>,
    role_policy: engineering_assurance::approval_gate::RolePolicy,
    selection: engineering_assurance::record_sink::config::SinkSelection,
    transparency: TransparencyRequirement,
    receipt_signer: engineering_assurance::attestation_signing::WorkflowIdentityPolicy,
    receipt_source: engineering_assurance::attestation_signing::SourceWorkflowPolicy,
    approval_signer: engineering_assurance::attestation_signing::WorkflowIdentityPolicy,
}

fn load_protected_record_policy(
    request: &RecordRequest,
) -> Result<ProtectedReceiptPolicy, ReceiptHostError> {
    let (private, source) = protected_repository_source(request)?;
    let receipt_policy_bytes = git_show_base_file(
        &request.repository,
        &source.base_commit,
        ".engineering-assurance/receipt-policy.json",
    )
    .map_err(|()| ReceiptHostError::RecordRequest)?;
    let receipt_policy: ReceiptPolicy = serde_json::from_slice(&receipt_policy_bytes)
        .map_err(|_| ReceiptHostError::RecordRequest)?;
    if !valid_receipt_policy(&receipt_policy) {
        return Err(ReceiptHostError::RecordRequest);
    }
    let expected_gate_names = receipt_policy
        .gates
        .into_iter()
        .map(|gate| gate.name)
        .collect();
    let identity_bytes = git_show_base_file(
        &request.repository,
        &source.base_commit,
        ".engineering-assurance/workflow-identities.json",
    )
    .map_err(|()| ReceiptHostError::RecordRequest)?;
    let identities = WorkflowIdentityDocument::parse(&identity_bytes)
        .map_err(|_| ReceiptHostError::RecordRequest)?;
    let role_bytes = git_show_base_file(
        &request.repository,
        &source.base_commit,
        ".engineering-assurance/approval-policy.json",
    )
    .map_err(|()| ReceiptHostError::RecordRequest)?;
    let role_policy = parse_policy(&role_bytes).map_err(|_| ReceiptHostError::RecordRequest)?;
    let sink_bytes = git_show_base_file(
        &request.repository,
        &source.base_commit,
        ".engineering-assurance/sinks.json",
    )
    .map_err(|()| ReceiptHostError::RecordRequest)?;
    let sink_config =
        SinkConfig::parse(&sink_bytes).map_err(|_| ReceiptHostError::RecordRequest)?;
    let visibility = if private {
        Visibility::Private
    } else {
        Visibility::Public
    };
    let selection =
        resolve_sink_for_repository(Some(&sink_config), visibility, &request.github_repository)
            .map_err(|_| ReceiptHostError::RecordRequest)?;
    validate_receipt_sink(selection.sink, private)?;
    let transparency = if private {
        TransparencyRequirement::TimestampOnly
    } else {
        TransparencyRequirement::PublicLog
    };
    let receipt_signer = identities
        .signer_policy(
            WorkflowRole::ReceiptWriter,
            &request.receipt_signer_repository,
            &request.receipt_signer_workflow_path,
            &request.receipt_signer_ref,
            transparency,
        )
        .ok_or(ReceiptHostError::RecordRequest)?;
    let receipt_source = identities
        .source_policy(
            WorkflowRole::ReceiptWriter,
            &request.receipt_signer_repository,
            &request.receipt_signer_workflow_path,
            &request.receipt_signer_ref,
        )
        .ok_or(ReceiptHostError::RecordRequest)?;
    let approval_signer = identities
        .signer_policy(
            WorkflowRole::StatementWriter,
            &request.approval_signer_repository,
            &request.approval_signer_workflow_path,
            &request.approval_signer_ref,
            transparency,
        )
        .ok_or(ReceiptHostError::RecordRequest)?;
    Ok(ProtectedReceiptPolicy {
        source,
        expected_gate_names,
        role_policy,
        selection,
        transparency,
        receipt_signer,
        receipt_source,
        approval_signer,
    })
}

fn protected_repository_source(
    request: &RecordRequest,
) -> Result<(bool, ReceiptPolicySource), ReceiptHostError> {
    let repository_info = gh_api_json(&format!("repos/{}", request.github_repository))
        .map_err(|()| ReceiptHostError::RecordRequest)?;
    if repository_info
        .get("default_branch")
        .and_then(Value::as_str)
        != Some(request.base_branch.as_str())
    {
        return Err(ReceiptHostError::RecordRequest);
    }
    let private = repository_info
        .get("private")
        .and_then(Value::as_bool)
        .ok_or(ReceiptHostError::RecordRequest)?;
    let branch_info = gh_api_json(&format!(
        "repos/{}/branches/{}",
        request.github_repository,
        encode_path_segment(&request.base_branch)
    ))
    .map_err(|()| ReceiptHostError::RecordRequest)?;
    if branch_info.get("protected").and_then(Value::as_bool) != Some(true) {
        return Err(ReceiptHostError::RecordRequest);
    }
    let base_commit = branch_info
        .pointer("/commit/sha")
        .and_then(Value::as_str)
        .filter(|commit| is_full_object_id(commit))
        .ok_or(ReceiptHostError::RecordRequest)?
        .to_owned();
    Ok((
        private,
        ReceiptPolicySource {
            repository: request.repository.clone(),
            base_commit,
        },
    ))
}

fn validate_receipt_sink(sink: SinkKind, private_repository: bool) -> Result<(), ReceiptHostError> {
    use engineering_assurance::receipt_record::ReceiptRecordError;

    if sink == SinkKind::None {
        return Err(ReceiptHostError::Record(
            ReceiptRecordError::PrivateStoreRequired,
        ));
    }
    if !private_repository && sink != SinkKind::PublicLog {
        return Err(ReceiptHostError::Record(
            ReceiptRecordError::PublicLogRequired,
        ));
    }
    Ok(())
}

struct ReceiptPolicySource {
    repository: PathBuf,
    base_commit: String,
}

impl PolicySource for ReceiptPolicySource {
    fn base_branch_protected(&self) -> Option<bool> {
        Some(true)
    }

    fn read_from_base_branch(&self, path: &str) -> Option<Vec<u8>> {
        git_show_base_file(&self.repository, &self.base_commit, path).ok()
    }
}

pub(crate) struct GitHubVisibility;

impl engineering_assurance::record_sink::leak::VisibilityLookup for GitHubVisibility {
    fn repo_visibility(
        &self,
        repository: &str,
    ) -> Result<Visibility, engineering_assurance::record_sink::leak::LookupFailed> {
        let value = gh_api_json(&format!("repos/{repository}"))
            .map_err(|()| engineering_assurance::record_sink::leak::LookupFailed)?;
        match value.get("private").and_then(Value::as_bool) {
            Some(true) => Ok(Visibility::Private),
            Some(false) => Ok(Visibility::Public),
            None => Err(engineering_assurance::record_sink::leak::LookupFailed),
        }
    }
}

struct HttpReceiptLog;

impl PublicLogClient for HttpReceiptLog {
    fn send(
        &self,
        endpoint: &str,
        entry: &engineering_assurance::record_sink::sink::CheckedEntry,
    ) -> Result<(), LogRejected> {
        if !endpoint.starts_with(concat!("https:", "//")) {
            return Err(LogRejected);
        }
        let bytes = entry.record().entry_bytes().map_err(|_| LogRejected)?;
        if bytes.len() > 1024 * 1024 {
            return Err(LogRejected);
        }
        let mut child = std::process::Command::new("curl")
            .args([
                "--silent",
                "--show-error",
                "--fail",
                "--max-time",
                "30",
                "--output",
                "/dev/null",
                "--header",
                "content-type: application/json",
                "--data-binary",
                "@-",
                endpoint,
            ])
            .env_clear()
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|_| LogRejected)?;
        child
            .stdin
            .take()
            .ok_or(LogRejected)?
            .write_all(&bytes)
            .map_err(|_| LogRejected)?;
        let status = child.wait().map_err(|_| LogRejected)?;
        status.success().then_some(()).ok_or(LogRejected)
    }
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, ReceiptHostError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| ReceiptHostError::RecordRequest)?;
    if !metadata.file_type().is_file() || metadata.len() > limit as u64 {
        return Err(ReceiptHostError::RecordRequest);
    }
    std::fs::read(path).map_err(|_| ReceiptHostError::RecordRequest)
}

fn valid_receipt_policy(policy: &ReceiptPolicy) -> bool {
    policy.protocol == "engineering-assurance.receipt-policy/v1"
        && policy.grace_minutes <= 1440
        && !policy.gates.is_empty()
        && policy.gates.len() <= 256
        && policy.gates.iter().all(|gate| {
            !gate.name.is_empty()
                && gate.name.len() <= 256
                && !gate.argv.is_empty()
                && gate.argv.len() <= 256
                && gate
                    .argv
                    .iter()
                    .all(|argument| !argument.is_empty() && argument.len() <= 16_384)
        })
        && policy
            .gates
            .iter()
            .map(|gate| gate.name.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            == policy.gates.len()
}

fn valid_pull_request(value: &str, repository: &str) -> bool {
    let Some((actual_repository, number)) = value.split_once('#') else {
        return false;
    };
    actual_repository == repository
        && !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit())
}

fn current_seconds_timestamp() -> String {
    let now = time::OffsetDateTime::now_utc();
    let month = match now.month() {
        time::Month::January => 1,
        time::Month::February => 2,
        time::Month::March => 3,
        time::Month::April => 4,
        time::Month::May => 5,
        time::Month::June => 6,
        time::Month::July => 7,
        time::Month::August => 8,
        time::Month::September => 9,
        time::Month::October => 10,
        time::Month::November => 11,
        time::Month::December => 12,
    };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        month,
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

/// Evaluate a first-parent receipt inventory using platform reachability times.
pub(crate) fn execute_missing(bytes: &[u8]) -> Result<Vec<u8>, ReceiptHostError> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(ReceiptHostError::MissingRequest);
    }
    let request: MissingRequest =
        serde_json::from_slice(bytes).map_err(|_| ReceiptHostError::MissingRequest)?;
    if request.protocol != MISSING_REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !request.repository.is_dir()
        || !valid_github_repository(&request.github_repository)
        || !valid_branch(&request.base_branch)
        || !valid_workflow_ref(&request.receipt_signer_ref)
        || !valid_workflow_ref(&request.approval_signer_ref)
    {
        return Err(ReceiptHostError::MissingRequest);
    }
    let (findings, tag_findings) = missing_receipt_findings(&request)?;
    let (failed_receipts, missing) = separate_receipt_findings(findings);
    render_missing(&missing, &failed_receipts, &tag_findings)
}

/// The one line `receipt-missing` prints: every commit finding with `kind`
/// `change` and every tag finding with `kind` `tag`.
fn render_missing(
    missing: &[MissingReceipt],
    failed_receipts: &[MissingReceipt],
    tag_findings: &[engineering_assurance::tag_receipt::Judged],
) -> Result<Vec<u8>, ReceiptHostError> {
    fn change(findings: &[MissingReceipt]) -> Vec<ChangeFindingLine<'_>> {
        findings
            .iter()
            .map(|finding| ChangeFindingLine {
                kind: "change",
                finding,
            })
            .collect()
    }
    let mut encoded = serde_json::to_vec(&MissingResultLine {
        protocol: MISSING_RESULT_PROTOCOL,
        missing: change(missing),
        failed_receipts: change(failed_receipts),
        tag_findings: tag_findings.iter().map(TagFindingLine::from).collect(),
    })
    .map_err(|_| ReceiptHostError::Encoding)?;
    encoded.push(b'\n');
    Ok(encoded)
}

fn separate_receipt_findings(
    findings: Vec<MissingReceipt>,
) -> (Vec<MissingReceipt>, Vec<MissingReceipt>) {
    findings.into_iter().partition(|finding| {
        finding.reason == engineering_assurance::receipt::MissingReceiptReason::FailedReceipt
    })
}

struct MissingHistoryContext {
    policy: ReceiptPolicy,
    /// The protected base commit the policy documents were read at.
    base_commit: String,
    tag_policy: Option<engineering_assurance::tag_receipt::TagReceiptPolicy>,
    commits: Vec<String>,
    pushed_at: BTreeMap<String, u64>,
    in_progress: BTreeSet<String>,
}

fn missing_receipt_findings(
    request: &MissingRequest,
) -> Result<
    (
        Vec<MissingReceipt>,
        Vec<engineering_assurance::tag_receipt::Judged>,
    ),
    ReceiptHostError,
> {
    let MissingHistoryContext {
        policy,
        base_commit,
        tag_policy,
        commits: history_commits,
        pushed_at,
        in_progress,
    } = load_missing_history_context(request)?;
    let verify_template = VerifyRequest {
        protocol: VERIFY_REQUEST_PROTOCOL.to_owned(),
        repository: request.repository.clone(),
        github_repository: request.github_repository.clone(),
        base_branch: request.base_branch.clone(),
        commit: history_commits[0].clone(),
        merged_pull_request: String::new(),
        receipt_signer_repository: request.receipt_signer_repository.clone(),
        receipt_signer_workflow_path: request.receipt_signer_workflow_path.clone(),
        receipt_signer_ref: request.receipt_signer_ref.clone(),
        approval_signer_repository: request.approval_signer_repository.clone(),
        approval_signer_workflow_path: request.approval_signer_workflow_path.clone(),
        approval_signer_ref: request.approval_signer_ref.clone(),
    };
    let verification_setup = load_verification_setup(&verify_template).ok();
    let now_unix = u64::try_from(time::OffsetDateTime::now_utc().unix_timestamp())
        .map_err(|_| ReceiptHostError::MissingRequest)?;
    let mut history = Vec::with_capacity(history_commits.len());
    for commit in history_commits {
        let state = receipt_state_for_commit(request, &commit, verification_setup.as_ref());
        let (state, receipt_run_in_progress) = match state {
            Ok(state) => (state, in_progress.contains(&commit)),
            Err(_) => (
                engineering_assurance::receipt::ReceiptCheckState::StatusUnknown,
                false,
            ),
        };
        history.push(ReceiptHistoryEntry {
            pushed_at_unix: pushed_at.get(&commit).copied(),
            commit,
            receipt_run_in_progress,
            state,
        });
    }
    let findings =
        find_missing_receipts(&history, &policy.adopted_at, policy.grace_minutes, now_unix)
            .map_err(|error| match error {
                engineering_assurance::receipt::MissingReceiptError::AdoptedAtInvalid => {
                    ReceiptHostError::AdoptedAtInvalid
                }
                _ => ReceiptHostError::MissingRequest,
            })?;
    let tag_findings = match tag_policy {
        None => Vec::new(),
        Some(tag_policy) => {
            let identity_bytes = git_show_base_file(
                &request.repository,
                &base_commit,
                ".engineering-assurance/workflow-identities.json",
            )
            .map_err(|()| ReceiptHostError::MissingRequest)?;
            let identities = WorkflowIdentityDocument::parse(&identity_bytes)
                .map_err(|_| ReceiptHostError::MissingRequest)?;
            repository_findings(
                &request.repository,
                &identities,
                &tag_policy,
                &ProductionVerifiers::default(),
            )
            .map_err(ReceiptHostError::Tag)?
        }
    };
    Ok((findings, tag_findings))
}

fn load_missing_history_context(
    request: &MissingRequest,
) -> Result<MissingHistoryContext, ReceiptHostError> {
    let repository_info = gh_api_json(&format!("repos/{}", request.github_repository))
        .map_err(|()| ReceiptHostError::Git)?;
    if repository_info
        .get("default_branch")
        .and_then(Value::as_str)
        != Some(request.base_branch.as_str())
    {
        return Err(ReceiptHostError::MissingRequest);
    }
    let branch_json = gh_api_json(&format!(
        "repos/{}/branches/{}",
        request.github_repository,
        encode_path_segment(&request.base_branch)
    ))
    .map_err(|()| ReceiptHostError::Git)?;
    if branch_json.get("protected").and_then(Value::as_bool) != Some(true) {
        return Err(ReceiptHostError::MissingRequest);
    }
    let base_commit = branch_json
        .pointer("/commit/sha")
        .and_then(Value::as_str)
        .filter(|value| is_full_object_id(value))
        .ok_or(ReceiptHostError::Git)?;
    let policy_bytes = git_show_base_file(
        &request.repository,
        base_commit,
        ".engineering-assurance/receipt-policy.json",
    )
    .map_err(|()| ReceiptHostError::Git)?;
    let policy: ReceiptPolicy =
        serde_json::from_slice(&policy_bytes).map_err(|_| ReceiptHostError::MissingRequest)?;
    if !valid_receipt_policy(&policy) {
        return Err(ReceiptHostError::MissingRequest);
    }
    let history_bytes = git_output(
        &request.repository,
        &[
            "rev-list",
            "--first-parent",
            "--max-count=100001",
            base_commit,
        ],
    )
    .map_err(|_| ReceiptHostError::Git)?;
    let history_commits = String::from_utf8(history_bytes)
        .map_err(|_| ReceiptHostError::Git)?
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if history_commits.is_empty() || history_commits.len() > 100_000 {
        return Err(ReceiptHostError::MissingRequest);
    }
    let activities = github_push_activities(&request.github_repository, &request.base_branch)
        .unwrap_or_default();
    let branch_ref = format!("refs/heads/{}", request.base_branch);
    let pushed_at = pushed_commit_times(
        &request.repository,
        &history_commits,
        &activities,
        &branch_ref,
    )
    .unwrap_or_default();
    let in_progress = receipt_runs_in_progress(&request.github_repository, &request.base_branch)
        .unwrap_or_default();
    let tag_policy = policy
        .tag_receipts
        .as_ref()
        .map(engineering_assurance::tag_receipt::TagReceiptPolicy::parse)
        .transpose()
        .map_err(|_| ReceiptHostError::Tag(TagReceiptHostError::PolicyInvalid))?;
    Ok(MissingHistoryContext {
        policy,
        base_commit: base_commit.to_owned(),
        tag_policy,
        commits: history_commits,
        pushed_at,
        in_progress,
    })
}

fn receipt_state_for_commit(
    request: &MissingRequest,
    commit: &str,
    setup: Option<&VerificationSetup>,
) -> Result<engineering_assurance::receipt::ReceiptCheckState, UnverifiableReason> {
    use engineering_assurance::receipt::ReceiptCheckState;
    let note = match git_output(
        &request.repository,
        &["notes", "--ref", RECEIPT_NOTES_REF, "show", commit],
    ) {
        Ok(note) => note,
        Err(GitError::NotFound) => return Ok(ReceiptCheckState::NoNote),
        Err(GitError::Failed) => return Err(UnverifiableReason::NoteUnreadable),
    };
    if ReceiptBundleSet::parse_note_groups(&note).is_err() {
        return Err(UnverifiableReason::NoteUnreadable);
    }
    let Some(setup) = setup else {
        return Ok(ReceiptCheckState::StatusUnknown);
    };
    let verify = VerifyRequest {
        protocol: VERIFY_REQUEST_PROTOCOL.to_owned(),
        repository: request.repository.clone(),
        github_repository: request.github_repository.clone(),
        base_branch: request.base_branch.clone(),
        commit: commit.to_owned(),
        merged_pull_request: request
            .merged_pull_requests
            .get(commit)
            .cloned()
            .unwrap_or_default(),
        receipt_signer_repository: request.receipt_signer_repository.clone(),
        receipt_signer_workflow_path: request.receipt_signer_workflow_path.clone(),
        receipt_signer_ref: request.receipt_signer_ref.clone(),
        approval_signer_repository: request.approval_signer_repository.clone(),
        approval_signer_workflow_path: request.approval_signer_workflow_path.clone(),
        approval_signer_ref: request.approval_signer_ref.clone(),
    };
    let groups = ReceiptBundleSet::parse_note_groups(&note)
        .map_err(|_| UnverifiableReason::NoteUnreadable)?;
    match verify_groups_for_commit(&verify, commit, &groups, setup) {
        Ok(ReceiptStatus::Verified) => Ok(ReceiptCheckState::Verified),
        Ok(ReceiptStatus::VerifiedFailed) => Ok(ReceiptCheckState::VerifiedFailed),
        Ok(ReceiptStatus::Unverifiable) | Err(_) => Ok(ReceiptCheckState::StatusUnknown),
        Ok(_) => Ok(ReceiptCheckState::InvalidReceipt),
    }
}

fn github_push_activities(
    github_repository: &str,
    base_branch: &str,
) -> Result<Vec<Value>, ReceiptHostError> {
    let reference = encode_query_value(&format!("refs/heads/{base_branch}"));
    let endpoint = format!(
        "repos/{github_repository}/activity?ref={reference}&activity_type=push&direction=asc&per_page=100"
    );
    let output = run_host_tool(
        OsStr::new("gh"),
        &[
            OsStr::new("api"),
            OsStr::new("--paginate"),
            OsStr::new("--slurp"),
            OsStr::new(&endpoint),
        ],
        None,
        32 * 1024 * 1024,
    )
    .map_err(|()| ReceiptHostError::Git)?;
    let pages: Value = serde_json::from_slice(&output).map_err(|_| ReceiptHostError::Git)?;
    let pages = pages.as_array().ok_or(ReceiptHostError::Git)?;
    let mut activities = Vec::new();
    for page in pages {
        let rows = page.as_array().ok_or(ReceiptHostError::Git)?;
        for row in rows {
            if row.get("ref").and_then(Value::as_str)
                == Some(format!("refs/heads/{base_branch}").as_str())
            {
                activities.push(row.clone());
            }
        }
    }
    Ok(activities)
}

fn pushed_commit_times(
    repository: &Path,
    first_parent_history: &[String],
    activities: &[Value],
    expected_ref: &str,
) -> Result<BTreeMap<String, u64>, ReceiptHostError> {
    let history = first_parent_history
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut pushed_at_by_commit = BTreeMap::new();
    for activity in activities {
        let before = activity
            .get("before")
            .and_then(Value::as_str)
            .ok_or(ReceiptHostError::Git)?;
        let after = activity
            .get("after")
            .and_then(Value::as_str)
            .filter(|value| is_full_object_id(value))
            .ok_or(ReceiptHostError::Git)?;
        let reference = activity
            .get("ref")
            .and_then(Value::as_str)
            .ok_or(ReceiptHostError::Git)?;
        if reference != expected_ref {
            continue;
        }
        let pushed_at = activity
            .get("pushed_at")
            .and_then(Value::as_str)
            .and_then(parse_unix_time)
            .ok_or(ReceiptHostError::Git)?;
        let exclusion = format!("^{before}");
        let arguments: Vec<&str> = if before.bytes().all(|byte| byte == b'0') {
            vec![
                "rev-list",
                "--first-parent",
                "--max-count=100001",
                "--reverse",
                after,
            ]
        } else {
            vec![
                "rev-list",
                "--first-parent",
                "--max-count=100001",
                "--reverse",
                after,
                &exclusion,
            ]
        };
        let output = git_output(repository, &arguments).map_err(|_| ReceiptHostError::Git)?;
        let new_commits = String::from_utf8(output).map_err(|_| ReceiptHostError::Git)?;
        for commit in new_commits.lines() {
            if !is_full_object_id(commit) {
                return Err(ReceiptHostError::Git);
            }
            if history.contains(commit) {
                pushed_at_by_commit
                    .entry(commit.to_owned())
                    .and_modify(|current: &mut u64| *current = (*current).min(pushed_at))
                    .or_insert(pushed_at);
            }
        }
    }
    Ok(pushed_at_by_commit)
}

fn receipt_runs_in_progress(
    github_repository: &str,
    base_branch: &str,
) -> Result<BTreeSet<String>, ReceiptHostError> {
    let branch = encode_query_value(base_branch);
    let endpoint = format!(
        "repos/{github_repository}/actions/runs?branch={branch}&status=in_progress&per_page=100"
    );
    let output = run_host_tool(
        OsStr::new("gh"),
        &[
            OsStr::new("api"),
            OsStr::new("--paginate"),
            OsStr::new("--slurp"),
            OsStr::new(&endpoint),
        ],
        None,
        8 * 1024 * 1024,
    )
    .map_err(|()| ReceiptHostError::Git)?;
    let pages: Value = serde_json::from_slice(&output).map_err(|_| ReceiptHostError::Git)?;
    let pages = pages.as_array().ok_or(ReceiptHostError::Git)?;
    let mut commits = BTreeSet::new();
    for page in pages {
        let runs = page
            .get("workflow_runs")
            .and_then(Value::as_array)
            .ok_or(ReceiptHostError::Git)?;
        for run in runs {
            if run.get("path").and_then(Value::as_str)
                == Some(".github/workflows/change-receipt.yml")
                && let Some(commit) = run
                    .get("head_sha")
                    .and_then(Value::as_str)
                    .filter(|commit| is_full_object_id(commit))
            {
                commits.insert(commit.to_owned());
            }
        }
    }
    Ok(commits)
}

fn parse_unix_time(value: &str) -> Option<u64> {
    time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .ok()
        .and_then(|time| u64::try_from(time.unix_timestamp()).ok())
}

fn encode_query_value(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

/// Verify one commit's note with policies loaded from the protected base.
pub(crate) fn execute_verify(bytes: &[u8]) -> Result<Vec<u8>, ReceiptHostError> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(ReceiptHostError::VerifyInvalid);
    }
    let request: VerifyRequest =
        serde_json::from_slice(bytes).map_err(|_| ReceiptHostError::VerifyUnreadable)?;
    if request.protocol != VERIFY_REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !request.repository.is_dir()
        || !valid_github_repository(&request.github_repository)
        || !valid_branch(&request.base_branch)
        || !is_full_object_id(&request.commit)
        || !valid_workflow_ref(&request.receipt_signer_ref)
        || !valid_workflow_ref(&request.approval_signer_ref)
    {
        return Err(ReceiptHostError::VerifyInvalid);
    }
    let status = verify_request(&request);
    let (status, reason) = match status {
        Ok(status) => (status, None),
        Err(reason) => (ReceiptStatus::Unverifiable, Some(reason)),
    };
    let result = ReceiptVerifyResult {
        protocol: VERIFY_RESULT_PROTOCOL.to_owned(),
        status,
        reason,
    };
    if !is_valid_verify_result(&result) {
        return Err(ReceiptHostError::Encoding);
    }
    let mut line = serde_json::to_vec(&result).map_err(|_| ReceiptHostError::Encoding)?;
    line.push(b'\n');
    Ok(line)
}

fn verify_request(request: &VerifyRequest) -> Result<ReceiptStatus, UnverifiableReason> {
    let commit = verify_commit(&request.repository, &request.commit)
        .map_err(|_| UnverifiableReason::NoteUnreadable)?;
    let note = match git_output(
        &request.repository,
        &["notes", "--ref", RECEIPT_NOTES_REF, "show", &commit],
    ) {
        Ok(note) => note,
        Err(GitError::NotFound) => return Err(UnverifiableReason::NoteAbsent),
        Err(GitError::Failed) => return Err(UnverifiableReason::NoteUnreadable),
    };
    let bundles = ReceiptBundleSet::parse_note_groups(&note)
        .map_err(|_| UnverifiableReason::NoteUnreadable)?;
    let setup = load_verification_setup(request)?;
    verify_groups_for_commit(request, &commit, &bundles, &setup)
}

struct VerificationSetup {
    receipt_signer: engineering_assurance::attestation_signing::WorkflowIdentityPolicy,
    receipt_source: engineering_assurance::attestation_signing::SourceWorkflowPolicy,
    approval_signer: engineering_assurance::attestation_signing::WorkflowIdentityPolicy,
    role_policy: engineering_assurance::approval_gate::RolePolicy,
    private_destination: String,
    verifier: SigstoreBundleVerifier,
}

struct ProtectedVerificationInputs {
    private: bool,
    identities: WorkflowIdentityDocument,
    role_policy: engineering_assurance::approval_gate::RolePolicy,
    sink_config: SinkConfig,
}

fn load_verification_setup(
    request: &VerifyRequest,
) -> Result<VerificationSetup, UnverifiableReason> {
    let ProtectedVerificationInputs {
        private,
        identities,
        role_policy,
        sink_config,
    } = load_protected_verification_inputs(request)?;
    let visibility = if private {
        Visibility::Private
    } else {
        Visibility::Public
    };
    let sink =
        resolve_sink_for_repository(Some(&sink_config), visibility, &request.github_repository)
            .map_err(|_| UnverifiableReason::PolicyUnreadable)?;
    let private_destination = sink
        .private_store_destination
        .filter(|destination| !destination.is_empty())
        .ok_or(UnverifiableReason::PolicyUnreadable)?;

    let transparency = if private {
        TransparencyRequirement::TimestampOnly
    } else {
        TransparencyRequirement::PublicLog
    };
    let receipt_signer = identities
        .signer_policy(
            WorkflowRole::ReceiptWriter,
            &request.receipt_signer_repository,
            &request.receipt_signer_workflow_path,
            &request.receipt_signer_ref,
            transparency,
        )
        .ok_or(UnverifiableReason::PolicyUnreadable)?;
    let receipt_source = identities
        .source_policy(
            WorkflowRole::ReceiptWriter,
            &request.receipt_signer_repository,
            &request.receipt_signer_workflow_path,
            &request.receipt_signer_ref,
        )
        .ok_or(UnverifiableReason::PolicyUnreadable)?;
    let approval_signer = identities
        .signer_policy(
            WorkflowRole::StatementWriter,
            &request.approval_signer_repository,
            &request.approval_signer_workflow_path,
            &request.approval_signer_ref,
            transparency,
        )
        .ok_or(UnverifiableReason::PolicyUnreadable)?;
    let trust_root_json = match transparency {
        TransparencyRequirement::PublicLog => {
            sigstore_verify::trust_root::SIGSTORE_PRODUCTION_TRUSTED_ROOT
        }
        TransparencyRequirement::TimestampOnly => {
            sigstore_verify::trust_root::SIGSTORE_GITHUB_TRUSTED_ROOT
        }
    };
    let trusted_root = sigstore_verify::trust_root::TrustedRoot::from_json(trust_root_json)
        .map_err(|_| UnverifiableReason::TrustRootUnreadable)?;
    let verifier = SigstoreBundleVerifier::new(&trusted_root)
        .map_err(|_| UnverifiableReason::TrustRootUnreadable)?;
    Ok(VerificationSetup {
        receipt_signer,
        receipt_source,
        approval_signer,
        role_policy,
        private_destination,
        verifier,
    })
}

fn load_protected_verification_inputs(
    request: &VerifyRequest,
) -> Result<ProtectedVerificationInputs, UnverifiableReason> {
    let repository_json = gh_api_json(&format!("repos/{}", request.github_repository))
        .map_err(|()| UnverifiableReason::PolicyUnreadable)?;
    if repository_json
        .get("default_branch")
        .and_then(Value::as_str)
        != Some(request.base_branch.as_str())
    {
        return Err(UnverifiableReason::PolicyUnreadable);
    }
    let private = repository_json
        .get("private")
        .and_then(Value::as_bool)
        .ok_or(UnverifiableReason::PolicyUnreadable)?;
    let branch_json = gh_api_json(&format!(
        "repos/{}/branches/{}",
        request.github_repository,
        encode_path_segment(&request.base_branch)
    ))
    .map_err(|()| UnverifiableReason::PolicyUnreadable)?;
    if branch_json.get("protected").and_then(Value::as_bool) != Some(true) {
        return Err(UnverifiableReason::PolicyUnreadable);
    }
    let base_commit = branch_json
        .pointer("/commit/sha")
        .and_then(Value::as_str)
        .filter(|value| is_full_object_id(value))
        .ok_or(UnverifiableReason::PolicyUnreadable)?;
    let identities_bytes = git_show_base_file(
        &request.repository,
        base_commit,
        ".engineering-assurance/workflow-identities.json",
    )
    .map_err(|()| UnverifiableReason::PolicyUnreadable)?;
    let identities = WorkflowIdentityDocument::parse(&identities_bytes)
        .map_err(|_| UnverifiableReason::PolicyUnreadable)?;
    let role_bytes = git_show_base_file(
        &request.repository,
        base_commit,
        ".engineering-assurance/approval-policy.json",
    )
    .map_err(|()| UnverifiableReason::PolicyUnreadable)?;
    let role_policy =
        parse_policy(&role_bytes).map_err(|_| UnverifiableReason::PolicyUnreadable)?;
    let sink_bytes = git_show_base_file(
        &request.repository,
        base_commit,
        ".engineering-assurance/sinks.json",
    )
    .map_err(|()| UnverifiableReason::PolicyUnreadable)?;
    let sink_config =
        SinkConfig::parse(&sink_bytes).map_err(|_| UnverifiableReason::PolicyUnreadable)?;
    Ok(ProtectedVerificationInputs {
        private,
        identities,
        role_policy,
        sink_config,
    })
}

fn verify_groups_for_commit(
    request: &VerifyRequest,
    commit: &str,
    groups: &[ReceiptBundleSet],
    setup: &VerificationSetup,
) -> Result<ReceiptStatus, UnverifiableReason> {
    let tree = git_output(
        &request.repository,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{commit}^{{tree}}"),
        ],
    )
    .map_err(|_| UnverifiableReason::NoteUnreadable)?;
    let tree = String::from_utf8(tree).map_err(|_| UnverifiableReason::NoteUnreadable)?;
    let tree = tree.trim();
    if !is_full_object_id(tree) {
        return Err(UnverifiableReason::NoteUnreadable);
    }
    let reader = StatementsNotRetainedReader;
    let subject = ExpectedReceiptSubject {
        repository: request.github_repository.clone(),
        commit: commit.to_owned(),
        tree: tree.to_owned(),
        merged_pull_request: request.merged_pull_request.clone(),
    };
    let context = ReceiptVerificationContext {
        subject: &subject,
        receipt_signer: &setup.receipt_signer,
        receipt_source: &setup.receipt_source,
        approval_signer: &setup.approval_signer,
        role_policy: &setup.role_policy,
        approval_reader: &reader,
        private_destination: &setup.private_destination,
        verifier: &setup.verifier,
    };
    match verify_receipt_groups(groups, &context) {
        Ok(ReceiptStatus::Unverifiable) => Err(UnverifiableReason::StatementsNotRetained),
        Ok(status) => Ok(status),
        Err(engineering_assurance::receipt::ReceiptVerificationError::BundleInvalid) => {
            Ok(ReceiptStatus::SignatureInvalid)
        }
        Err(
            engineering_assurance::receipt::ReceiptVerificationError::PredicateInvalid
            | engineering_assurance::receipt::ReceiptVerificationError::DigestUnavailable,
        ) => Ok(ReceiptStatus::PredicateInvalid),
    }
}

struct StatementsNotRetainedReader;

pub(crate) fn gh_api_json(endpoint: &str) -> Result<Value, ()> {
    let output = run_host_tool(
        OsStr::new("gh"),
        &[OsStr::new("api"), OsStr::new(endpoint)],
        None,
        2 * 1024 * 1024,
    )?;
    serde_json::from_slice(&output).map_err(|_| ())
}

pub(crate) fn git_show_base_file(
    repository: &Path,
    commit: &str,
    path: &str,
) -> Result<Vec<u8>, ()> {
    if !is_full_object_id(commit)
        || !matches!(
            path,
            ".engineering-assurance/workflow-identities.json"
                | ".engineering-assurance/approval-policy.json"
                | ".engineering-assurance/sinks.json"
                | ".engineering-assurance/receipt-policy.json"
        )
    {
        return Err(());
    }
    let object = format!("{commit}:{path}");
    run_host_tool(
        OsStr::new("git"),
        &[
            OsStr::new("-C"),
            repository.as_os_str(),
            OsStr::new("show"),
            OsStr::new("--no-ext-diff"),
            OsStr::new("--no-textconv"),
            OsStr::new(&object),
        ],
        None,
        MAX_POLICY_BYTES,
    )
}

pub(crate) fn run_host_tool(
    executable: &OsStr,
    arguments: &[&OsStr],
    current_directory: Option<&Path>,
    max_output_bytes: usize,
) -> Result<Vec<u8>, ()> {
    let removed = REMOVED_GIT_ENVIRONMENT.map(OsStr::new);
    let output = crate::process_host::run_configured(
        executable,
        arguments,
        current_directory,
        &[],
        &removed,
        crate::process_host::ProcessLimits {
            timeout: PROCESS_TIMEOUT,
            max_output_bytes,
        },
    )
    .map_err(|_| ())?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(())
    }
}

pub(crate) fn valid_github_repository(value: &str) -> bool {
    let Some((owner, repository)) = value.split_once('/') else {
        return false;
    };
    !owner.is_empty()
        && !repository.is_empty()
        && [owner, repository].into_iter().all(|part| {
            part.bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        })
        && !Path::new(repository)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("git"))
}

pub(crate) fn valid_branch(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.ends_with('.')
        && !value.contains("..")
        && !value.contains("@{")
        && !value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
        && value.split('/').all(|part| {
            !part.is_empty()
                && !part.starts_with('.')
                && !part
                    .rsplit_once('.')
                    .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("lock"))
        })
}

fn valid_workflow_ref(value: &str) -> bool {
    (value.starts_with("refs/heads/") || value.starts_with("refs/tags/"))
        && value.len() > "refs/heads/".len()
        && !value.chars().any(char::is_control)
}

pub(crate) fn encode_path_segment(value: &str) -> String {
    value.replace('%', "%25").replace('/', "%2F")
}

impl ApprovalStatementReader for StatementsNotRetainedReader {
    fn lookup_approval_statements(
        &self,
        destination: &str,
        lookup: ApprovalStatementLookup<'_>,
    ) -> Result<Vec<RetainedApprovalStatement>, ApprovalStatementReadError> {
        if destination.is_empty() {
            return Err(ApprovalStatementReadError);
        }
        let _ = lookup;
        Ok(Vec::new())
    }
}

pub(crate) fn missing_result_is_non_failing(bytes: &[u8]) -> bool {
    serde_json::from_slice::<MissingResultLineOwned>(bytes).is_ok_and(|result| {
        result.protocol == MISSING_RESULT_PROTOCOL
            && result.missing.is_empty()
            && result
                .tag_findings
                .iter()
                .all(|finding| finding.status == "accepted")
    })
}

/// Read or write one receipt note using the fixed notes ref.
pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, ReceiptHostError> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(ReceiptHostError::Invalid);
    }
    let request: Request =
        serde_json::from_slice(bytes).map_err(|_| ReceiptHostError::Unreadable)?;
    if request.protocol != REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !request.repository.is_dir()
        || !is_full_object_id(&request.commit)
    {
        return Err(ReceiptHostError::Invalid);
    }
    let commit = verify_commit(&request.repository, &request.commit)?;
    let (operation, bundles, status) = match request.operation {
        Operation::Store => {
            let bundle_set = request.bundles.ok_or(ReceiptHostError::Invalid)?;
            let note = bundle_set.encode_note().map_err(map_note_error)?;
            let skipped =
                store_note_bytes(&request.repository, &commit, &note, |existing_groups| {
                    let verification = request.verification.as_ref().ok_or(())?;
                    if !valid_github_repository(&verification.github_repository)
                        || !valid_branch(&verification.base_branch)
                        || !valid_workflow_ref(&verification.receipt_signer_ref)
                        || !valid_workflow_ref(&verification.approval_signer_ref)
                    {
                        return Err(());
                    }
                    let verify_request = VerifyRequest {
                        protocol: VERIFY_REQUEST_PROTOCOL.to_owned(),
                        repository: request.repository.clone(),
                        github_repository: verification.github_repository.clone(),
                        base_branch: verification.base_branch.clone(),
                        commit: commit.clone(),
                        merged_pull_request: verification.merged_pull_request.clone(),
                        receipt_signer_repository: verification.receipt_signer_repository.clone(),
                        receipt_signer_workflow_path: verification
                            .receipt_signer_workflow_path
                            .clone(),
                        receipt_signer_ref: verification.receipt_signer_ref.clone(),
                        approval_signer_repository: verification.approval_signer_repository.clone(),
                        approval_signer_workflow_path: verification
                            .approval_signer_workflow_path
                            .clone(),
                        approval_signer_ref: verification.approval_signer_ref.clone(),
                    };
                    let setup = load_verification_setup(&verify_request).map_err(|_| ())?;
                    verify_groups_for_commit(&verify_request, &commit, existing_groups, &setup)
                        .map_err(|_| ())
                })?;
            (
                "store",
                None,
                Some(if skipped { "skipped" } else { "stored" }),
            )
        }
        Operation::Read => {
            if request.bundles.is_some() {
                return Err(ReceiptHostError::Invalid);
            }
            match git_output(
                &request.repository,
                &["notes", "--ref", RECEIPT_NOTES_REF, "show", &commit],
            ) {
                Ok(note) => {
                    let bundle_sets =
                        ReceiptBundleSet::parse_note_groups(&note).map_err(map_note_error)?;
                    ("read", Some(bundle_sets), Some("present"))
                }
                Err(GitError::NotFound) => ("read", None, Some("absent")),
                Err(GitError::Failed) => return Err(ReceiptHostError::Git),
            }
        }
    };
    let mut encoded = serde_json::to_vec(&ResultLine {
        protocol: RESULT_PROTOCOL,
        operation,
        commit,
        bundles,
        status,
    })
    .map_err(|_| ReceiptHostError::Encoding)?;
    encoded.push(b'\n');
    Ok(encoded)
}

fn store_note_bytes<F>(
    repository: &Path,
    commit: &str,
    note: &[u8],
    mut verify_existing: F,
) -> Result<bool, ReceiptHostError>
where
    F: FnMut(&[ReceiptBundleSet]) -> Result<ReceiptStatus, ()>,
{
    match git_output(
        repository,
        &["notes", "--ref", RECEIPT_NOTES_REF, "show", commit],
    ) {
        Ok(existing_note) => {
            let existing_groups =
                ReceiptBundleSet::parse_note_groups(&existing_note).map_err(map_note_error)?;
            if verify_existing(&existing_groups).is_ok_and(receipt_status_allows_idempotent_skip) {
                return Ok(true);
            }
            return Err(ReceiptHostError::NoteConflict);
        }
        Err(GitError::NotFound) => {}
        Err(GitError::Failed) => return Err(ReceiptHostError::Git),
    }
    git_with_input(
        repository,
        &[
            "notes",
            "--ref",
            RECEIPT_NOTES_REF,
            "add",
            "-F",
            "-",
            commit,
        ],
        note,
    )
    .map_err(|_| ReceiptHostError::Git)?;
    Ok(false)
}

fn map_note_error(error: ReceiptNoteError) -> ReceiptHostError {
    match error {
        ReceiptNoteError::TooLarge => ReceiptHostError::TooLarge,
        ReceiptNoteError::InvalidLines | ReceiptNoteError::InvalidBundle => {
            ReceiptHostError::NoteInvalid
        }
    }
}

pub(crate) fn is_full_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn verify_commit(repository: &Path, commit: &str) -> Result<String, ReceiptHostError> {
    let output = git_output(
        repository,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{commit}^{{commit}}"),
        ],
    )
    .map_err(|_| ReceiptHostError::Git)?;
    let resolved = String::from_utf8(output).map_err(|_| ReceiptHostError::Git)?;
    let resolved = resolved.trim();
    if !is_full_object_id(resolved) || resolved != commit {
        return Err(ReceiptHostError::Invalid);
    }
    Ok(resolved.to_owned())
}

#[derive(Clone, Copy)]
pub(crate) enum GitError {
    NotFound,
    Failed,
}

pub(crate) fn git_output(repository: &Path, arguments: &[&str]) -> Result<Vec<u8>, GitError> {
    let output = git_command(repository, arguments, None)?;
    if output.status.success() {
        Ok(output.stdout)
    } else if output.status.code() == Some(1)
        && arguments.first() == Some(&"notes")
        && arguments.get(3) == Some(&"show")
    {
        Err(GitError::NotFound)
    } else {
        Err(GitError::Failed)
    }
}

pub(crate) fn git_with_input(
    repository: &Path,
    arguments: &[&str],
    input: &[u8],
) -> Result<(), GitError> {
    let output = git_command(repository, arguments, Some(input))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(GitError::Failed)
    }
}

pub(crate) fn git_command(
    repository: &Path,
    arguments: &[&str],
    input: Option<&[u8]>,
) -> Result<std::process::Output, GitError> {
    let mut command_arguments = vec![OsStr::new("-C"), repository.as_os_str()];
    command_arguments.extend(arguments.iter().map(OsStr::new));
    let removed = REMOVED_GIT_ENVIRONMENT.map(OsStr::new);
    let limits = crate::process_host::ProcessLimits {
        timeout: PROCESS_TIMEOUT,
        max_output_bytes: 8 * 1024 * 1024,
    };
    let result = if let Some(bytes) = input {
        let mut input_file = tempfile::tempfile().map_err(|_| GitError::Failed)?;
        input_file.write_all(bytes).map_err(|_| GitError::Failed)?;
        input_file
            .seek(SeekFrom::Start(0))
            .map_err(|_| GitError::Failed)?;
        crate::process_host::run_configured_stdin(
            OsStr::new("git"),
            &command_arguments,
            None,
            &[],
            &removed,
            input_file,
            limits,
        )
    } else {
        crate::process_host::run_configured(
            OsStr::new("git"),
            &command_arguments,
            None,
            &[],
            &removed,
            limits,
        )
    }
    .map_err(|_| GitError::Failed)?;
    Ok(std::process::Output {
        status: result.status,
        stdout: result.stdout,
        stderr: result.stderr,
    })
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use std::fs;

    use serde_json::Value;
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;

    fn test_git(repository: &Path, arguments: &[&OsStr]) -> std::process::Output {
        let mut command_arguments = vec![OsStr::new("-C"), repository.as_os_str()];
        command_arguments.extend_from_slice(arguments);
        let output = crate::process_host::run_configured(
            OsStr::new("git"),
            &command_arguments,
            None,
            &[],
            &[],
            crate::process_host::ProcessLimits {
                timeout: PROCESS_TIMEOUT,
                max_output_bytes: 8 * 1024 * 1024,
            },
        )
        .expect("bounded Git test command runs");
        std::process::Output {
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }

    fn test_git_str(repository: &Path, arguments: &[&str]) -> std::process::Output {
        let arguments = arguments.iter().map(OsStr::new).collect::<Vec<_>>();
        test_git(repository, &arguments)
    }

    fn fixture_repo() -> (TempDir, String) {
        let directory = TempDir::new().expect("temporary repository");
        let root = directory.path();
        assert!(test_git_str(root, &["init", "-q"]).status.success());
        for (key, value) in [
            ("user.name", "Receipt Test"),
            ("user.email", "receipt@example.invalid"),
        ] {
            assert!(test_git_str(root, &["config", key, value]).status.success());
        }
        fs::write(root.join("tracked.txt"), "fictional receipt fixture\n").unwrap();
        assert!(test_git_str(root, &["add", "tracked.txt"]).status.success());
        assert!(
            test_git_str(root, &["commit", "-qm", "fixture"])
                .status
                .success()
        );
        let output = test_git_str(root, &["rev-parse", "HEAD"]);
        let commit = String::from_utf8(output.stdout).unwrap().trim().to_owned();
        (directory, commit)
    }

    fn request(
        operation: &str,
        repository: &Path,
        commit: &str,
        bundles: Option<&Value>,
    ) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "protocol": REQUEST_PROTOCOL,
            "operation": operation,
            "repository": repository,
            "commit": commit,
            "bundles": bundles,
        }))
        .unwrap()
    }

    fn verification_request(repository: &Path, commit: &str) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "protocol": VERIFY_REQUEST_PROTOCOL,
            "repository": repository,
            "github_repository": "fictional/project",
            "base_branch": "main",
            "commit": commit,
            "merged_pull_request": "fictional/project#1",
            "receipt_signer_repository": "fictional/assurance",
            "receipt_signer_workflow_path": ".github/workflows/change-receipt.yml",
            "receipt_signer_ref": "refs/heads/main",
            "approval_signer_repository": "fictional/assurance",
            "approval_signer_workflow_path": ".github/workflows/approval-statement.yml",
            "approval_signer_ref": "refs/heads/main"
        }))
        .unwrap()
    }

    fn bundle_value() -> Value {
        let signed_bundle = |predicate_type: &str, predicate: Value| {
            let payload = serde_json::to_vec(&json!({
                "_type": engineering_assurance::attestation_signing::STATEMENT_V1_TYPE,
                "subject": [],
                "predicateType": predicate_type,
                "predicate": predicate
            }))
            .unwrap();
            let envelope = sigstore_types::DsseEnvelope::new(
                "application/vnd.in-toto+json",
                sigstore_types::PayloadBytes::from(payload),
                sigstore_types::DsseSignature::new(
                    sigstore_types::SignatureBytes::from(vec![0]),
                    sigstore_types::KeyId::default(),
                ),
            );
            let bundle = sigstore_types::Bundle::new(
                sigstore_types::MediaType::Bundle0_3,
                sigstore_types::VerificationMaterial::new(
                    sigstore_types::VerificationMaterialContent::PublicKey(
                        sigstore_types::PublicKeyIdentifier::new("fictional"),
                    ),
                ),
                sigstore_types::SignatureContent::DsseEnvelope(envelope),
            );
            serde_json::to_value(bundle).unwrap()
        };
        let change = signed_bundle(
            engineering_assurance::change_record::PREDICATE_TYPE,
            json!({}),
        );
        let test = signed_bundle(
            engineering_assurance::attestation_verify::TEST_RESULT_PREDICATE,
            json!({}),
        );
        let digest = |bundle: &Value| {
            let bundle: sigstore_types::Bundle = serde_json::from_value(bundle.clone()).unwrap();
            let sigstore_types::SignatureContent::DsseEnvelope(envelope) = bundle.content else {
                panic!("fixture is a DSSE bundle");
            };
            let statement: Value = serde_json::from_slice(envelope.payload.as_ref()).unwrap();
            let canonical = serde_json_canonicalizer::to_vec(&statement).unwrap();
            format!(
                "sha256:{}",
                sigstore_verify::crypto::sha256(&canonical).to_hex()
            )
        };
        let summary = signed_bundle(
            engineering_assurance::attestation_verify::VERIFICATION_SUMMARY_PREDICATE,
            json!({
                "verifier": {"id": "fixture"},
                "timeVerified": "2026-11-14T00:00:00Z",
                "resourceUri": "fixture",
                "policy": {"uri": "fixture"},
                "inputAttestations": [
                    {"uri": "change", "digest": {"sha256": digest(&change)}},
                    {"uri": "test", "digest": {"sha256": digest(&test)}}
                ],
                "verificationResult": "PASSED",
                "verifiedLevels": []
            }),
        );
        json!({
            "change_record": change,
            "test_result": test,
            "summary": summary
        })
    }

    #[trace("TC-597", "FR-076-AC-1")]
    #[test]
    fn stores_and_reads_a_note_on_the_requested_commit() {
        let (directory, commit) = fixture_repo();
        let root = directory.path();
        let bundles = bundle_value();
        let stored = execute(&request("store", root, &commit, Some(&bundles)))
            .expect("receipt note is written");
        assert!(
            String::from_utf8(stored)
                .unwrap()
                .contains("\"status\":\"stored\"")
        );
        let duplicate = execute(&request("store", root, &commit, Some(&bundles)))
            .expect_err("existing receipt is never overwritten");
        assert_eq!(duplicate.code(), "receipt_note_conflict");
        let read = execute(&request("read", root, &commit, None)).expect("receipt note is read");
        let result: Value = serde_json::from_slice(&read).unwrap();
        assert_eq!(result["status"], "present");
        assert_eq!(result["bundles"][0], bundle_value());
        let raw_note = test_git_str(
            root,
            &["notes", "--ref", RECEIPT_NOTES_REF, "show", &commit],
        );
        assert_eq!(
            raw_note
                .stdout
                .split_inclusive(|byte| *byte == b'\n')
                .count(),
            3
        );
    }

    #[trace("TC-597", "FR-076-AC-1")]
    #[test]
    fn receipt_note_ref_survives_a_second_default_branch_push() {
        let (directory, commit) = fixture_repo();
        let repository = directory.path();
        let remote = TempDir::new().expect("local bare remote");
        assert!(
            test_git_str(remote.path(), &["init", "-q", "--bare"])
                .status
                .success()
        );
        assert!(
            test_git_str(repository, &["branch", "-M", "main"])
                .status
                .success()
        );
        assert!(
            test_git(
                repository,
                &[
                    OsStr::new("remote"),
                    OsStr::new("add"),
                    OsStr::new("origin"),
                    remote.path().as_os_str(),
                ],
            )
            .status
            .success()
        );
        assert!(
            test_git_str(repository, &["push", "-u", "origin", "main"])
                .status
                .success()
        );

        let bundles = bundle_value();
        let stored = execute(&request("store", repository, &commit, Some(&bundles)))
            .expect("fixture receipt is stored locally");
        assert!(
            String::from_utf8(stored)
                .unwrap()
                .contains("\"status\":\"stored\"")
        );
        assert!(
            test_git_str(repository, &["push", "origin", RECEIPT_NOTES_REF])
                .status
                .success()
        );
        let note_ref_before = test_git_str(remote.path(), &["rev-parse", RECEIPT_NOTES_REF]);
        assert!(note_ref_before.status.success());
        let note_ref_before = String::from_utf8(note_ref_before.stdout)
            .unwrap()
            .trim()
            .to_owned();

        fs::write(
            repository.join("second.txt"),
            "second default-branch push\n",
        )
        .unwrap();
        assert!(
            test_git_str(repository, &["add", "second.txt"])
                .status
                .success()
        );
        assert!(
            test_git_str(repository, &["commit", "-qm", "second push"])
                .status
                .success()
        );
        assert!(
            test_git_str(repository, &["push", "origin", "main"])
                .status
                .success()
        );
        let note_ref_after = test_git_str(remote.path(), &["rev-parse", RECEIPT_NOTES_REF]);
        assert!(note_ref_after.status.success());
        assert_eq!(
            String::from_utf8(note_ref_after.stdout).unwrap().trim(),
            note_ref_before
        );
        let remote_note = test_git_str(
            remote.path(),
            &["notes", "--ref", RECEIPT_NOTES_REF, "show", &commit],
        );
        assert!(remote_note.status.success());
        assert_eq!(
            remote_note
                .stdout
                .split_inclusive(|byte| *byte == b'\n')
                .count(),
            3
        );
    }

    #[trace("TC-603", "FR-076-AC-7")]
    #[test]
    fn verified_and_verified_failed_receipt_reruns_leave_the_existing_note_unchanged() {
        for status in [ReceiptStatus::Verified, ReceiptStatus::VerifiedFailed] {
            let (directory, commit) = fixture_repo();
            let repository = directory.path();
            let original: ReceiptBundleSet =
                serde_json::from_value(bundle_value()).expect("fixture receipt bundles");
            let original_bytes = original.encode_note().expect("fixture note bytes");
            git_with_input(
                repository,
                &[
                    "notes",
                    "--ref",
                    RECEIPT_NOTES_REF,
                    "add",
                    "-F",
                    "-",
                    &commit,
                ],
                &original_bytes,
            )
            .expect("initial verified receipt note is written");

            let skipped =
                store_note_bytes(repository, &commit, b"replacement note\n", |_| Ok(status))
                    .expect("verified receipt retry is skipped");

            assert!(skipped, "{status:?} receipt must be idempotent");
            let current = test_git_str(
                repository,
                &["notes", "--ref", RECEIPT_NOTES_REF, "show", &commit],
            );
            assert_eq!(
                current.stdout, original_bytes,
                "{status:?} note remains unchanged"
            );
        }
    }

    #[trace("TC-598", "FR-076-AC-2")]
    #[test]
    fn refuses_oversized_note_without_writing_anything() {
        let (directory, commit) = fixture_repo();
        let huge = json!({
            "change_record": {"payload": "x".repeat(1024 * 1024)},
            "test_result": {}, "summary": {}
        });
        let error = execute(&request("store", directory.path(), &commit, Some(&huge)))
            .expect_err("oversized note is rejected");
        assert_eq!(error.code(), "receipt_note_too_large");
        assert!(
            test_git_str(
                directory.path(),
                &["notes", "--ref", RECEIPT_NOTES_REF, "list"],
            )
            .stdout
            .is_empty()
        );
    }

    #[trace("TC-610", "FR-077-AC-7")]
    #[test]
    fn push_activity_assigns_platform_timestamp_to_newly_reachable_commit() {
        let (directory, commit) = fixture_repo();
        let history = vec![commit.clone()];
        let activities = vec![json!({
            "before": "0000000000000000000000000000000000000000",
            "after": commit,
            "ref": "refs/heads/main",
            "pushed_at": "2026-11-13T12:34:56Z"
        })];
        let times = pushed_commit_times(directory.path(), &history, &activities, "refs/heads/main")
            .expect("platform push record resolves commit time");
        assert_eq!(times.get(&history[0]), Some(&1_794_573_296));
    }

    #[trace("TC-614", "FR-078-AC-4")]
    #[test]
    fn receipt_verify_rejects_caller_supplied_approval_proofs() {
        let (directory, commit) = fixture_repo();
        let mut request: Value =
            serde_json::from_slice(&verification_request(directory.path(), &commit)).unwrap();
        request["approval_statements"] = json!([{
            "repository": "fictional/project",
            "merged_pull_request": "fictional/project#1",
            "merged_commit": commit,
            "statement": "caller-supplied statement",
            "signature_bundle": "caller-supplied bundle",
            "publish_outcome": "published"
        }]);
        let error = execute_verify(&serde_json::to_vec(&request).unwrap())
            .expect_err("caller-provided proof rows are rejected at the wire boundary");
        assert_eq!(error.code(), "receipt_verify_request_unreadable");

        let reader = StatementsNotRetainedReader;
        assert!(
            reader
                .lookup_approval_statements(
                    "private-store",
                    ApprovalStatementLookup {
                        repository: "fictional/project",
                        merged_pull_request: "fictional/project#1",
                        merged_commit: &"a".repeat(40),
                    },
                )
                .unwrap()
                .is_empty()
        );
    }

    #[trace("TC-615", "FR-078-AC-5")]
    #[test]
    fn receipt_verify_reports_absent_and_unparsable_notes() {
        let (directory, commit) = fixture_repo();
        let absent = execute_verify(&verification_request(directory.path(), &commit))
            .expect("missing note produces a protocol result");
        let absent: Value = serde_json::from_slice(&absent).unwrap();
        assert_eq!(absent["status"], "unverifiable");
        assert_eq!(absent["reason"], "note_absent");

        assert!(
            test_git_str(
                directory.path(),
                &[
                    "notes",
                    "--ref",
                    RECEIPT_NOTES_REF,
                    "add",
                    "-m",
                    "not-json",
                    &commit,
                ],
            )
            .status
            .success()
        );
        let unreadable = execute_verify(&verification_request(directory.path(), &commit))
            .expect("unparsable note produces a protocol result");
        let unreadable: Value = serde_json::from_slice(&unreadable).unwrap();
        assert_eq!(unreadable["status"], "unverifiable");
        assert_eq!(unreadable["reason"], "note_unreadable");
    }

    #[trace("TC-616", "FR-078-AC-6", "FR-064-AC-8")]
    #[test]
    fn protected_policy_reads_use_the_explicit_base_commit() {
        let (directory, _) = fixture_repo();
        let repository = directory.path();
        let policy_path = repository.join(".engineering-assurance/approval-policy.json");
        let identities_path = repository.join(".engineering-assurance/workflow-identities.json");
        fs::create_dir_all(policy_path.parent().unwrap()).unwrap();
        fs::write(&policy_path, "protected policy\n").unwrap();
        fs::write(&identities_path, "protected workflow identities\n").unwrap();
        assert!(
            test_git_str(repository, &["add", ".engineering-assurance"])
                .status
                .success()
        );
        assert!(
            test_git_str(repository, &["commit", "-qm", "protected policy"])
                .status
                .success()
        );
        let base_output = test_git_str(repository, &["rev-parse", "HEAD"]);
        let base_commit = String::from_utf8(base_output.stdout)
            .unwrap()
            .trim()
            .to_owned();
        fs::write(&policy_path, "branch policy\n").unwrap();
        fs::write(&identities_path, "branch workflow identities\n").unwrap();
        assert!(
            test_git_str(repository, &["add", ".engineering-assurance"])
                .status
                .success()
        );
        assert!(
            test_git_str(repository, &["commit", "-qm", "branch policy"])
                .status
                .success()
        );
        let branch_output = test_git_str(repository, &["rev-parse", "HEAD"]);
        let branch_commit = String::from_utf8(branch_output.stdout)
            .unwrap()
            .trim()
            .to_owned();
        assert_eq!(
            git_show_base_file(
                repository,
                &base_commit,
                ".engineering-assurance/approval-policy.json"
            )
            .unwrap(),
            b"protected policy\n"
        );
        assert_eq!(
            git_show_base_file(
                repository,
                &branch_commit,
                ".engineering-assurance/approval-policy.json"
            )
            .unwrap(),
            b"branch policy\n"
        );
        assert_eq!(
            git_show_base_file(
                repository,
                &base_commit,
                ".engineering-assurance/workflow-identities.json"
            )
            .unwrap(),
            b"protected workflow identities\n"
        );
        assert_eq!(
            git_show_base_file(
                repository,
                &branch_commit,
                ".engineering-assurance/workflow-identities.json"
            )
            .unwrap(),
            b"branch workflow identities\n"
        );
    }

    #[trace("TC-605", "TC-609", "FR-077-AC-2", "FR-077-AC-6")]
    #[test]
    fn failed_receipt_findings_do_not_fail_a_missing_receipt_check() {
        let (failed_receipts, missing) = separate_receipt_findings(vec![
            MissingReceipt {
                commit: "a".repeat(40),
                reason: engineering_assurance::receipt::MissingReceiptReason::FailedReceipt,
            },
            MissingReceipt {
                commit: "b".repeat(40),
                reason: engineering_assurance::receipt::MissingReceiptReason::NoNote,
            },
        ]);
        assert_eq!(failed_receipts.len(), 1);
        assert_eq!(missing.len(), 1);
        let empty = serde_json::to_vec(&json!({
            "protocol": MISSING_RESULT_PROTOCOL,
            "missing": [],
            "failed_receipts": []
        }))
        .unwrap();
        let failed_only = serde_json::to_vec(&json!({
            "protocol": MISSING_RESULT_PROTOCOL,
            "missing": [],
            "failed_receipts": [{"commit": "a".repeat(40), "reason": "failed_receipt"}]
        }))
        .unwrap();
        let actual_missing = serde_json::to_vec(&json!({
            "protocol": MISSING_RESULT_PROTOCOL,
            "missing": [{"commit": "a".repeat(40), "reason": "no_note"}],
            "failed_receipts": []
        }))
        .unwrap();
        assert!(missing_result_is_non_failing(&empty));
        assert!(missing_result_is_non_failing(&failed_only));
        assert!(!missing_result_is_non_failing(&actual_missing));
    }

    fn tag_receipt(
        name: &str,
        object: char,
        commit: char,
    ) -> engineering_assurance::tag_receipt::TagReceipt {
        engineering_assurance::tag_receipt::TagReceipt {
            reference: format!("refs/tags/{name}"),
            tag_object: object.to_string().repeat(40),
            commit: commit.to_string().repeat(40),
            pushed_at: "2026-11-20T10:00:00Z".to_owned(),
            trailers_readable: true,
            record_ids: Vec::new(),
        }
    }

    fn judged(
        finding: engineering_assurance::tag_receipt::TagFinding,
        accepted: bool,
    ) -> engineering_assurance::tag_receipt::Judged {
        engineering_assurance::tag_receipt::Judged {
            finding,
            accepted: accepted.then(|| engineering_assurance::tag_receipt::Acceptance {
                accepted_by: "alice".to_owned(),
                accepted_reason: "reviewed".to_owned(),
            }),
        }
    }

    #[test]
    #[trace("TC-1133", "FR-082-AC-7", "FR-082-AC-11")]
    fn tag_findings_carry_kind_tag_and_fail_the_check_unless_accepted() {
        use engineering_assurance::tag_receipt::{
            InvalidReason, InvalidReceipt, MissingTagReason, TagFinding,
        };
        let no_receipt = TagFinding::TagNoReceipt {
            reference: "refs/tags/approve-1".to_owned(),
            current_object: "a".repeat(40),
        };
        let missing = TagFinding::MissingTag {
            receipt: tag_receipt("approve-2", 'b', 'c'),
            reason: MissingTagReason::TagChanged,
            current_object: Some("d".repeat(40)),
        };
        let invalid = TagFinding::InvalidReceipt(InvalidReceipt {
            commit: "c".repeat(40),
            reason: InvalidReason::SignatureInvalid,
        });
        let change = [MissingReceipt {
            commit: "e".repeat(40),
            reason: engineering_assurance::receipt::MissingReceiptReason::InvalidReceipt,
        }];

        // An empty finding list leaves the check green; a commit finding of
        // `kind` `change` and a tag finding of `kind` `tag` are told apart.
        let empty = render_missing(&[], &[], &[]).unwrap();
        assert!(missing_result_is_non_failing(&empty));
        let line: Value = serde_json::from_slice(
            &render_missing(
                &change,
                &[],
                &[
                    judged(missing.clone(), false),
                    judged(invalid.clone(), false),
                ],
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(line["missing"][0]["kind"], "change");
        assert_eq!(line["missing"][0]["reason"], "invalid_receipt");
        let tag_invalid = &line["tag_findings"][1];
        assert_eq!(tag_invalid["kind"], "tag");
        assert_eq!(tag_invalid["status"], "invalid_receipt");
        assert_eq!(tag_invalid["reason"], "signature_invalid");
        assert!(
            tag_invalid.get("ref").is_none(),
            "an untrusted receipt names no ref"
        );
        let tag_missing = &line["tag_findings"][0];
        assert_eq!(tag_missing["status"], "missing_tag");
        assert_eq!(tag_missing["ref"], "refs/tags/approve-2");
        assert_eq!(tag_missing["tag_object"], "b".repeat(40));
        assert_eq!(tag_missing["reason"], "tag_changed");
        assert_eq!(tag_missing["current_object"], "d".repeat(40));
        assert_eq!(tag_missing["commit"], "c".repeat(40));

        // Each of the three tag statuses fails the check; an accepted one does not.
        for finding in [no_receipt.clone(), missing.clone(), invalid] {
            let result = render_missing(&[], &[], &[judged(finding, false)]).unwrap();
            assert!(!missing_result_is_non_failing(&result));
        }
        let accepted =
            render_missing(&[], &[], &[judged(missing, true), judged(no_receipt, true)]).unwrap();
        assert!(missing_result_is_non_failing(&accepted));
        let accepted: Value = serde_json::from_slice(&accepted).unwrap();
        assert_eq!(accepted["tag_findings"][0]["status"], "accepted");
        assert_eq!(accepted["tag_findings"][0]["finding"], "missing_tag");
        assert_eq!(accepted["tag_findings"][0]["accepted_by"], "alice");
        assert_eq!(accepted["tag_findings"][0]["accepted_reason"], "reviewed");
        assert_eq!(accepted["tag_findings"][1]["finding"], "tag_no_receipt");
    }
}
