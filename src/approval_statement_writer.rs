// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Post-merge selection, signing, and sink handoff for approval statements
//! (FR-058, FR-060, FR-065).

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::implements;
use serde_json::{Value, json};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    approval_gate::{
        self, AccountRef, AccountType, Approval, GateErrorCode, GateResult, RolePolicy,
    },
    attestation_signing::{STATEMENT_V1_TYPE, SigningFailure, StatementV1, SubjectV1},
    record_sink::{
        form::{
            GitObjectId, Hashes, PrivateRecord, PublicProtocol, PublicRecord, Record, RecordId,
            RecordKind, References, Subject, Timestamps, Verdict, Verdicts,
        },
        sink::{RecordSink, SinkResult},
    },
};

const APPROVAL_PREDICATE_TYPE: &str =
    concat!("https:", "//schemas.agent-ix.ai/attestation/approval/v1");

/// Signs one canonical in-toto statement with the workflow's signing identity.
pub trait ApprovalStatementSigner {
    /// Sign canonical statement bytes and return the Sigstore bundle JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the signing service refuses or cannot complete the
    /// operation.
    fn sign_statement(&self, canonical_statement: &[u8]) -> Result<String, StatementSigningError>;
}

/// The statement signing operation failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("approval statement signing failed")]
pub struct StatementSigningError;

/// The production signer that delegates directly to the Sigstore stack.
#[derive(Clone, Copy, Debug, Default)]
pub struct SigstoreStatementSigner;

impl ApprovalStatementSigner for SigstoreStatementSigner {
    fn sign_statement(&self, canonical_statement: &[u8]) -> Result<String, StatementSigningError> {
        crate::attestation_signing::sign_statement_bytes(canonical_statement)
            .map_err(|_failure: SigningFailure| StatementSigningError)
    }
}

/// The metadata shared by every statement for one merged pull request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MergedPullRequest<'a> {
    /// Repository in `owner/name` form.
    pub repository: &'a str,
    /// Pull request reference, for example `owner/name#12`.
    pub pull_request: &'a str,
    /// Pull request head commit that was merged.
    pub head_commit: &'a str,
    /// Commit produced on the base branch.
    pub merged_commit: &'a str,
}

/// The source-reported lifecycle state of a pull request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PullRequestState<'a> {
    /// The pull request is still open.
    Open,
    /// The pull request was closed without merging.
    Closed,
    /// The source reports a completed merge and its immutable commits.
    Merged(MergedPullRequest<'a>),
}

/// One statement handed to the configured sink and the result it returned.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WrittenApprovalStatement {
    /// Stable policy account identity for the approval.
    pub account: AccountRef,
    /// Role covered by this statement.
    pub role: String,
    /// Exact canonical in-toto bytes that were signed.
    pub statement_bytes: Vec<u8>,
    /// Sigstore bundle returned by the statement signer.
    pub signature_bundle: String,
    /// Result of handing the approval record to the configured sink.
    pub sink_result: SinkResult,
}

/// A refusal before all selected approval statements have been handed to the sink.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum StatementWriteError {
    /// The source reports that the pull request has not merged.
    #[error("approval statements require a merged pull request")]
    NotMerged,
    /// Policy evaluation could not safely select approvals.
    #[error("approval policy or source is unavailable")]
    GateUnavailable(GateErrorCode),
    /// There is not exactly one caller-provided random record id per selected approval.
    #[error("approval statement record id count does not match selected approvals")]
    RecordIdCount,
    /// The merge metadata, selected approval, or display name is invalid.
    #[error("approval statement metadata is invalid")]
    Metadata,
    /// Canonical statement encoding failed.
    #[error("approval statement encoding failed")]
    Encoding,
    /// The delegated signer refused a statement.
    #[error("approval statement signing failed")]
    Signing,
    /// The sink could not construct a valid approval record.
    #[error("approval record construction failed")]
    Record,
}

/// Inputs that determine the approval statements written for one merged pull request.
#[derive(Clone, Copy)]
pub struct ApprovalStatementWriteRequest<'a> {
    /// Source lifecycle state and, after a merge, its immutable metadata.
    pub pull_request: PullRequestState<'a>,
    /// Required roles and eligible people used by the review gate.
    pub policy: &'a RolePolicy,
    /// Reviews to evaluate, including reviews that may not match the current head.
    pub approvals: &'a [Approval],
    /// Account types used to exclude agents and bots from matching.
    pub account_types: &'a BTreeMap<AccountRef, AccountType>,
    /// Display names keyed by stable account identity.
    pub approver_names: &'a BTreeMap<AccountRef, String>,
    /// Random record IDs, one per selected approval.
    pub record_ids: &'a [RecordId],
}

/// Evaluate source approvals with the shared FR-061 matcher, write one signed
/// statement for each counted approval, and hand each record to `sink`.
///
/// The caller supplies cryptographically random record IDs so ID generation
/// stays with the workflow host. `approver_names` is informational source
/// metadata keyed by stable account identity; it is never used for matching.
/// A normal unsatisfied result still records its maximum matching, while an
/// unavailable or invalid policy writes nothing.
///
/// # Errors
///
/// Returns before signing or publishing if the pull request has not merged.
/// Returns before publishing if policy evaluation is unavailable, merge or
/// approval metadata is invalid, the IDs do not match the selected approvals,
/// statement encoding fails, or signing fails.
#[implements("FR-060-AC-2", "FR-060-AC-4", "FR-060-AC-5", "FR-060-AC-6")]
pub fn write_approval_statements(
    sink: &dyn RecordSink,
    signer: &dyn ApprovalStatementSigner,
    request: ApprovalStatementWriteRequest<'_>,
) -> Result<(GateResult, Vec<WrittenApprovalStatement>), StatementWriteError> {
    let ApprovalStatementWriteRequest {
        pull_request,
        policy,
        approvals,
        account_types,
        approver_names,
        record_ids,
    } = request;
    let PullRequestState::Merged(merged) = pull_request else {
        return Err(StatementWriteError::NotMerged);
    };
    validate_merge(merged)?;
    let merged_approvals: Vec<_> = approvals
        .iter()
        .filter(|approval| {
            approval.commit == merged.head_commit && approval.pull_request == merged.pull_request
        })
        .cloned()
        .collect();
    let verdict = approval_gate::evaluate(policy, &merged_approvals, account_types);
    if let Some(error) = verdict.error {
        return Err(StatementWriteError::GateUnavailable(error));
    }

    let selected = select_counted(policy, &merged_approvals, &verdict, approver_names)?;
    if selected.len() != record_ids.len() {
        return Err(StatementWriteError::RecordIdCount);
    }
    let unique_ids: BTreeSet<_> = record_ids.iter().map(RecordId::as_str).collect();
    if unique_ids.len() != record_ids.len() {
        return Err(StatementWriteError::Record);
    }

    let mut prepared_records = Vec::with_capacity(selected.len());
    for ((approval, name), record_id) in selected.into_iter().zip(record_ids) {
        validate_approval(approval, name, merged)?;
        let statement = build_statement(approval, name, merged);
        let statement_bytes = statement
            .canonical_bytes()
            .map_err(|_| StatementWriteError::Encoding)?;
        let signature_bundle = signer
            .sign_statement(&statement_bytes)
            .map_err(|_| StatementWriteError::Signing)?;
        if signature_bundle.is_empty() {
            return Err(StatementWriteError::Signing);
        }
        let record = approval_record(
            record_id.clone(),
            merged,
            &statement_bytes,
            &signature_bundle,
        )
        .map_err(|_| StatementWriteError::Record)?;
        prepared_records.push((approval, statement_bytes, signature_bundle, record));
    }

    let mut written = Vec::with_capacity(prepared_records.len());
    for (approval, statement_bytes, signature_bundle, record) in prepared_records {
        let sink_result = sink.publish(&record);
        written.push(WrittenApprovalStatement {
            account: approval.account.clone(),
            role: approval.role.clone(),
            statement_bytes,
            signature_bundle,
            sink_result,
        });
    }
    Ok((verdict, written))
}

fn validate_merge(merged: MergedPullRequest<'_>) -> Result<(), StatementWriteError> {
    Subject::repo(merged.repository).map_err(|_| StatementWriteError::Metadata)?;
    GitObjectId::try_from(merged.head_commit).map_err(|_| StatementWriteError::Metadata)?;
    GitObjectId::try_from(merged.merged_commit).map_err(|_| StatementWriteError::Metadata)?;
    let Some((repository, number)) = merged.pull_request.rsplit_once('#') else {
        return Err(StatementWriteError::Metadata);
    };
    if repository != merged.repository
        || number.is_empty()
        || !number.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(StatementWriteError::Metadata);
    }
    Ok(())
}

fn select_counted<'a>(
    policy: &'a RolePolicy,
    approvals: &'a [Approval],
    verdict: &GateResult,
    approver_names: &'a BTreeMap<AccountRef, String>,
) -> Result<Vec<(&'a Approval, &'a str)>, StatementWriteError> {
    verdict
        .assignments
        .iter()
        .map(|assignment| {
            let accounts = policy
                .individuals
                .get(&assignment.individual)
                .ok_or(StatementWriteError::Metadata)?;
            let approval = approvals
                .iter()
                .find(|approval| {
                    approval.role == assignment.role
                        && approval.human
                        && accounts.contains(&approval.account)
                        && approval.source == approval.account.source
                })
                .ok_or(StatementWriteError::Metadata)?;
            let name = approver_names
                .get(&approval.account)
                .filter(|name| !name.is_empty())
                .map(String::as_str)
                .ok_or(StatementWriteError::Metadata)?;
            Ok((approval, name))
        })
        .collect()
}

fn validate_approval(
    approval: &Approval,
    approver_name: &str,
    merged: MergedPullRequest<'_>,
) -> Result<(), StatementWriteError> {
    let normalized = normalized_timestamp(&approval.timestamp)?;
    if approval.account.source.is_empty()
        || approval.account.id.is_empty()
        || approval.source != approval.account.source
        || approval.pull_request != merged.pull_request
        || approval.commit != merged.head_commit
        || !matches!(approval.commit.len(), 40 | 64)
        || !approval.commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        || !valid_role_name(&approval.role)
        || approval.meaning.trim() != approval.meaning
        || approval.meaning.is_empty()
        || approval.meaning.len() > 256
        || approval.meaning.chars().any(char::is_control)
        || approver_name.is_empty()
        || approver_name.chars().any(char::is_control)
        || normalized.len() != 20
    {
        return Err(StatementWriteError::Metadata);
    }
    if approval.account.source == "github"
        && !approval
            .account
            .id
            .bytes()
            .all(|byte| byte.is_ascii_digit())
    {
        return Err(StatementWriteError::Metadata);
    }
    Ok(())
}

fn normalized_timestamp(value: &str) -> Result<String, StatementWriteError> {
    let timestamp = OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|_| StatementWriteError::Metadata)?
        .to_offset(time::UtcOffset::UTC);
    if timestamp.nanosecond() != 0 {
        return Err(StatementWriteError::Metadata);
    }
    let normalized = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        timestamp.year(),
        timestamp.month() as u8,
        timestamp.day(),
        timestamp.hour(),
        timestamp.minute(),
        timestamp.second()
    );
    (normalized.len() == 20)
        .then_some(normalized)
        .ok_or(StatementWriteError::Metadata)
}

fn valid_role_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=32).contains(&bytes.len())
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

#[implements("FR-060-AC-2", "FR-060-AC-6")]
fn build_statement(
    approval: &Approval,
    approver_name: &str,
    merged: MergedPullRequest<'_>,
) -> StatementV1 {
    let timestamp = normalized_timestamp(&approval.timestamp).expect("validated timestamp");
    StatementV1 {
        type_: STATEMENT_V1_TYPE.to_owned(),
        subject: vec![SubjectV1 {
            name: merged.repository.to_owned(),
            digest: BTreeMap::from([("gitCommit".to_owned(), merged.merged_commit.to_owned())]),
        }],
        predicate_type: APPROVAL_PREDICATE_TYPE.to_owned(),
        predicate: BTreeMap::from([
            ("source".to_owned(), json!(approval.source)),
            (
                "approver".to_owned(),
                json!(format!(
                    "{}:{}",
                    approval.account.source, approval.account.id
                )),
            ),
            ("approver_name".to_owned(), json!(approver_name)),
            ("role".to_owned(), json!(approval.role)),
            ("meaning".to_owned(), json!(approval.meaning)),
            ("signed_at".to_owned(), json!(timestamp)),
            ("head_commit".to_owned(), json!(merged.head_commit)),
            ("pull_request".to_owned(), json!(approval.pull_request)),
            ("merged_commit".to_owned(), json!(merged.merged_commit)),
        ]),
    }
}

fn approval_record(
    record_id: RecordId,
    merged: MergedPullRequest<'_>,
    statement_bytes: &[u8],
    signature_bundle: &str,
) -> Result<Record, StatementWriteError> {
    let statement: Value =
        serde_json::from_slice(statement_bytes).map_err(|_| StatementWriteError::Encoding)?;
    let digest = format!(
        "sha256:{}",
        sigstore_verify::crypto::sha256(statement_bytes).to_hex()
    );
    let public = PublicRecord {
        protocol: PublicProtocol::V1,
        record_id: record_id.clone(),
        kind: RecordKind::Approval,
        subject: Subject::repo(merged.repository).map_err(|_| StatementWriteError::Record)?,
        references: References::default(),
        hashes: Hashes {
            statement: Some(digest.try_into().map_err(|_| StatementWriteError::Record)?),
            subject_commit: Some(
                merged
                    .merged_commit
                    .try_into()
                    .map_err(|_| StatementWriteError::Record)?,
            ),
            ..Hashes::default()
        },
        verdicts: Verdicts {
            approval: Some(Verdict::Satisfied),
            ..Verdicts::default()
        },
        timestamps: Timestamps::default(),
        signature: None,
    };
    let mut private = PrivateRecord::new(record_id);
    private.statement = Some(json!({
        "statement": statement,
        "signature_bundle": signature_bundle,
    }));
    Record::new(public, private).map_err(|_| StatementWriteError::Record)
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        collections::{BTreeMap, BTreeSet},
        fmt::Write as _,
        process::Command,
    };

    use ix_trace_rs::trace;

    use super::*;
    use crate::{
        approval_gate::{ApprovalSource, GitHubReviewAdapter, ReviewSnapshot, ReviewState},
        approval_statement::{
            ApprovalStatementLookup, ApprovalStatementPublishOutcome, ApprovalStatementReadError,
            ApprovalStatementStatus, RetainedApprovalStatement, verify_approval_statements,
        },
        attestation_signing::{
            BundleVerifier, TransparencyRequirement, VerificationFailure, WorkflowIdentityPolicy,
        },
        record_sink::{
            config::{SelectionSource, SinkKind, SinkSelection, Visibility},
            form::{PrivateRecord, PublicRecord},
            leak::{LookupFailed, VisibilityLookup},
            sink::{
                ApprovalStatementReader, LogRejected, Ports, PrivateStore, PublicLogClient,
                Signer as SinkSigner, SigningFailed, SinkLayer, SinkResult, StoreError,
            },
        },
    };

    const REPOSITORY: &str = "agent-ix/approval-fixture";
    const PULL_REQUEST: &str = "agent-ix/approval-fixture#41";
    const HEAD: &str = "1111111111111111111111111111111111111111";
    const MERGED: &str = "2222222222222222222222222222222222222222";

    struct GitFixture(tempfile::TempDir);

    impl GitFixture {
        fn new() -> Self {
            let fixture = Self(tempfile::tempdir().unwrap());
            fixture.git(&["init", "--quiet"]);
            fixture
        }

        fn git(&self, args: &[&str]) -> String {
            let output = Command::new("git")
                .current_dir(self.0.path())
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_AUTHOR_NAME", "Fixture Author")
                .env("GIT_AUTHOR_EMAIL", "author@example.test")
                .env("GIT_COMMITTER_NAME", "Fixture Committer")
                .env("GIT_COMMITTER_EMAIL", "committer@example.test")
                .env("GIT_AUTHOR_DATE", "2026-11-14T18:00:00Z")
                .env("GIT_COMMITTER_DATE", "2026-11-14T18:00:00Z")
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8(output.stdout).unwrap().trim().to_owned()
        }

        fn commit(&self, parents: &[&str], message: &str) -> String {
            let tree = self.git(&["mktree"]);
            let mut args = vec!["commit-tree", &tree, "-m", message];
            for parent in parents {
                args.extend(["-p", parent]);
            }
            self.git(&args)
        }
    }

    fn hex_encode(bytes: &[u8]) -> String {
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            write!(encoded, "{byte:02x}").expect("writing to a String cannot fail");
        }
        encoded
    }

    #[derive(Default)]
    struct CaptureSink(RefCell<Vec<Record>>);

    impl RecordSink for CaptureSink {
        fn publish(&self, record: &Record) -> SinkResult {
            self.0.borrow_mut().push(record.clone());
            SinkResult::published(SinkKind::PrivateStore, record.id())
        }
    }

    struct FixtureSigner;

    impl ApprovalStatementSigner for FixtureSigner {
        fn sign_statement(
            &self,
            canonical_statement: &[u8],
        ) -> Result<String, StatementSigningError> {
            let hex = hex_encode(canonical_statement);
            Ok(format!(".github/workflows/statement-writer.yml|{hex}"))
        }
    }

    struct FixtureVerifier;

    impl BundleVerifier for FixtureVerifier {
        fn verify(
            &self,
            payload: &[u8],
            bundle_json: &str,
            policy: &WorkflowIdentityPolicy,
        ) -> Result<(), VerificationFailure> {
            let Some((identity, hex)) = bundle_json.split_once('|') else {
                return Err(VerificationFailure::InvalidSignature);
            };
            let expected = hex_encode(payload);
            if hex != expected {
                return Err(VerificationFailure::InvalidSignature);
            }
            if identity != policy.workflow_path {
                return Err(VerificationFailure::UntrustedSigner);
            }
            Ok(())
        }
    }

    #[derive(Default)]
    struct Reader(Vec<RetainedApprovalStatement>);

    impl ApprovalStatementReader for Reader {
        fn lookup_approval_statements(
            &self,
            _destination: &str,
            _lookup: ApprovalStatementLookup<'_>,
        ) -> Result<Vec<RetainedApprovalStatement>, ApprovalStatementReadError> {
            Ok(self.0.clone())
        }
    }

    #[derive(Default)]
    struct SinkStore {
        private: RefCell<Vec<PrivateRecord>>,
        public: RefCell<Vec<PublicRecord>>,
    }

    impl PrivateStore for SinkStore {
        fn write_private(
            &self,
            _destination: &str,
            private: &PrivateRecord,
        ) -> Result<(), StoreError> {
            self.private.borrow_mut().push(private.clone());
            Ok(())
        }

        fn write_public(
            &self,
            _destination: &str,
            public: &PublicRecord,
        ) -> Result<(), StoreError> {
            self.public.borrow_mut().push(public.clone());
            Ok(())
        }
    }

    impl ApprovalStatementReader for SinkStore {}

    #[derive(Default)]
    struct SinkLog(Cell<u32>, RefCell<Vec<PublicRecord>>);

    impl PublicLogClient for SinkLog {
        fn send(
            &self,
            _endpoint: &str,
            entry: &crate::record_sink::sink::CheckedEntry,
        ) -> Result<(), LogRejected> {
            self.0.set(self.0.get() + 1);
            self.1.borrow_mut().push(entry.record().clone());
            Ok(())
        }
    }

    struct SinkVisibility(BTreeMap<String, Visibility>);

    impl VisibilityLookup for SinkVisibility {
        fn repo_visibility(&self, repository: &str) -> Result<Visibility, LookupFailed> {
            self.0.get(repository).copied().ok_or(LookupFailed)
        }
    }

    #[derive(Default)]
    struct RecordSigner {
        payload: RefCell<Option<Vec<u8>>>,
        public_calls: Cell<u32>,
    }

    impl SinkSigner for RecordSigner {
        fn sign_for_public_log(
            &self,
            proof: &crate::record_sink::leak::PublicPathProof<'_>,
        ) -> Result<String, SigningFailed> {
            self.public_calls.set(self.public_calls.get() + 1);
            self.payload.replace(Some(proof.signed_bytes().to_vec()));
            Ok("AQID".into())
        }

        fn sign_without_public_log(&self, canonical: &[u8]) -> Result<String, SigningFailed> {
            self.payload.replace(Some(canonical.to_vec()));
            Ok("AQID".into())
        }

        fn signed_payload(&self, bundle: &str) -> Result<Vec<u8>, SigningFailed> {
            (bundle == "AQID")
                .then(|| self.payload.borrow_mut().take())
                .flatten()
                .ok_or(SigningFailed)
        }
    }

    fn account(id: &str) -> AccountRef {
        AccountRef {
            source: "github".into(),
            id: id.into(),
        }
    }

    fn role_policy() -> RolePolicy {
        RolePolicy {
            protocol: approval_gate::POLICY_PROTOCOL.into(),
            required_roles: vec!["developer".into(), "engineer".into(), "stakeholder".into()],
            individuals: BTreeMap::from([
                ("alice".into(), vec![account("101")]),
                ("bob".into(), vec![account("202")]),
                ("carol".into(), vec![account("303")]),
            ]),
            roles: BTreeMap::from([
                ("developer".into(), vec!["alice".into()]),
                ("engineer".into(), vec!["bob".into()]),
                ("stakeholder".into(), vec!["carol".into()]),
            ]),
            agents: Vec::new(),
        }
    }

    fn approvals() -> Vec<Approval> {
        [
            ("101", "developer", "reviewed the implementation"),
            ("202", "engineer", "checked the design"),
            ("303", "stakeholder", "accepted the change"),
        ]
        .into_iter()
        .map(|(id, role, meaning)| Approval {
            source: "github".into(),
            account: account(id),
            human: true,
            role: role.into(),
            meaning: meaning.into(),
            timestamp: "2026-11-14T10:00:00-08:00".into(),
            commit: HEAD.into(),
            pull_request: PULL_REQUEST.into(),
        })
        .collect()
    }

    fn account_types() -> BTreeMap<AccountRef, AccountType> {
        ["101", "202", "303"]
            .into_iter()
            .map(|id| (account(id), AccountType::Human))
            .collect()
    }

    fn display_names() -> BTreeMap<AccountRef, String> {
        [("101", "Alice"), ("202", "Bob"), ("303", "Carol")]
            .into_iter()
            .map(|(id, name)| (account(id), name.to_owned()))
            .collect()
    }

    fn identity_policy(workflow_path: &str) -> WorkflowIdentityPolicy {
        WorkflowIdentityPolicy {
            repository: REPOSITORY.into(),
            workflow_path: workflow_path.into(),
            references: BTreeSet::from(["refs/heads/main".into()]),
            issuer: concat!("https:", "//token.actions.githubusercontent.com").into(),
            transparency: TransparencyRequirement::PublicLog,
        }
    }

    fn lookup() -> ApprovalStatementLookup<'static> {
        ApprovalStatementLookup {
            repository: REPOSITORY,
            merged_pull_request: PULL_REQUEST,
            merged_commit: MERGED,
        }
    }

    fn write_fixture_reviews(
        sink: &CaptureSink,
        merged: MergedPullRequest<'_>,
        reviews: &[Approval],
        record_ids: &[RecordId],
    ) -> (GateResult, Vec<WrittenApprovalStatement>) {
        write_approval_statements(
            sink,
            &FixtureSigner,
            ApprovalStatementWriteRequest {
                pull_request: PullRequestState::Merged(merged),
                policy: &role_policy(),
                approvals: reviews,
                account_types: &account_types(),
                approver_names: &display_names(),
                record_ids,
            },
        )
        .unwrap()
    }

    #[test]
    #[trace("FR-060-AC-2")]
    fn merge_commit_statements_name_the_source_merge_and_reviewed_head() {
        let git = GitFixture::new();
        let base = git.commit(&[], "base");
        let head = git.commit(&[&base], "reviewed head");
        let merged = git.commit(&[&base, &head], "merge pull request");
        assert_eq!(git.git(&["rev-parse", &format!("{merged}^2")]), head);
        let reviews: Vec<_> = approvals()
            .into_iter()
            .map(|mut approval| {
                approval.commit.clone_from(&head);
                approval
            })
            .collect();
        let sink = CaptureSink::default();
        let ids = [1_u8, 2, 3].map(|byte| RecordId::from_random_bytes([byte; 16]));
        let (_, statements) = write_fixture_reviews(
            &sink,
            MergedPullRequest {
                repository: REPOSITORY,
                pull_request: PULL_REQUEST,
                head_commit: &head,
                merged_commit: &merged,
            },
            &reviews,
            &ids,
        );
        assert_eq!(statements.len(), 3);
        assert_eq!(sink.0.borrow().len(), 3);
        for statement in statements {
            let value: Value = serde_json::from_slice(&statement.statement_bytes).unwrap();
            assert_eq!(value["subject"][0]["digest"]["gitCommit"], merged);
            assert_eq!(value["predicate"]["merged_commit"], merged);
            assert_eq!(value["predicate"]["head_commit"], head);
        }
    }

    #[test]
    #[trace("FR-060-AC-4")]
    fn open_and_closed_pull_requests_are_refused_before_signing_or_publishing() {
        struct CountingSigner(Cell<usize>);
        impl ApprovalStatementSigner for CountingSigner {
            fn sign_statement(&self, _: &[u8]) -> Result<String, StatementSigningError> {
                self.0.set(self.0.get() + 1);
                Err(StatementSigningError)
            }
        }
        for state in [PullRequestState::Open, PullRequestState::Closed] {
            let sink = CaptureSink::default();
            let signer = CountingSigner(Cell::new(0));
            let ids = [1_u8, 2, 3].map(|byte| RecordId::from_random_bytes([byte; 16]));
            let result = write_approval_statements(
                &sink,
                &signer,
                ApprovalStatementWriteRequest {
                    pull_request: state,
                    policy: &role_policy(),
                    approvals: &approvals(),
                    account_types: &account_types(),
                    approver_names: &display_names(),
                    record_ids: &ids,
                },
            );
            assert_eq!(result, Err(StatementWriteError::NotMerged), "{state:?}");
            assert_eq!(signer.0.get(), 0, "{state:?} must not be signed");
            assert!(
                sink.0.borrow().is_empty(),
                "{state:?} must not be published"
            );
        }
    }

    #[test]
    #[trace("FR-060-AC-5")]
    fn an_identical_tree_does_not_make_a_different_review_head_count() {
        let git = GitFixture::new();
        let base = git.commit(&[], "base");
        let reviewed = git.commit(&[&base], "reviewed head");
        let pushed = git.commit(&[&base], "head pushed after review");
        let squash = git.commit(&[&base], "squash result");
        assert_ne!(reviewed, pushed);
        assert_eq!(
            git.git(&["rev-parse", &format!("{reviewed}^{{tree}}")]),
            git.git(&["rev-parse", &format!("{squash}^{{tree}}")])
        );
        let reviews: Vec<_> = approvals()
            .into_iter()
            .map(|mut approval| {
                approval.commit.clone_from(&reviewed);
                approval
            })
            .collect();
        let sink = CaptureSink::default();
        let (verdict, statements) = write_fixture_reviews(
            &sink,
            MergedPullRequest {
                repository: REPOSITORY,
                pull_request: PULL_REQUEST,
                head_commit: &pushed,
                merged_commit: &squash,
            },
            &reviews,
            &[],
        );
        assert_eq!(verdict.status, approval_gate::GateStatus::Unsatisfied);
        assert!(statements.is_empty());
        assert!(sink.0.borrow().is_empty());
    }

    #[test]
    #[trace("FR-060-AC-6")]
    fn a_three_commit_rebase_writes_subjects_for_the_rebased_tip_only() {
        let git = GitFixture::new();
        let base = git.commit(&[], "base");
        let original_one = git.commit(&[&base], "first change");
        let original_two = git.commit(&[&original_one], "second change");
        let head = git.commit(&[&original_two], "third change");
        let advanced_base = git.commit(&[&base], "base advanced");
        let rebased_one = git.commit(&[&advanced_base], "first change");
        let rebased_two = git.commit(&[&rebased_one], "second change");
        let rebased_tip = git.commit(&[&rebased_two], "third change");
        assert_ne!(head, rebased_tip);
        assert_eq!(
            git.git(&[
                "rev-list",
                "--count",
                &format!("{advanced_base}..{rebased_tip}")
            ]),
            "3"
        );
        let reviews: Vec<_> = approvals()
            .into_iter()
            .map(|mut approval| {
                approval.commit.clone_from(&head);
                approval
            })
            .collect();
        let sink = CaptureSink::default();
        let ids = [1_u8, 2, 3].map(|byte| RecordId::from_random_bytes([byte; 16]));
        let (_, statements) = write_fixture_reviews(
            &sink,
            MergedPullRequest {
                repository: REPOSITORY,
                pull_request: PULL_REQUEST,
                head_commit: &head,
                merged_commit: &rebased_tip,
            },
            &reviews,
            &ids,
        );
        assert_eq!(statements.len(), 3);
        assert_eq!(sink.0.borrow().len(), 3);
        for statement in statements {
            let value: Value = serde_json::from_slice(&statement.statement_bytes).unwrap();
            let subject = &value["subject"][0]["digest"]["gitCommit"];
            assert_eq!(subject, &rebased_tip);
            assert_ne!(subject, &rebased_one);
            assert_ne!(subject, &rebased_two);
            assert_eq!(value["predicate"]["merged_commit"], rebased_tip);
            assert_eq!(value["predicate"]["head_commit"], head);
        }
    }

    #[trace(
        "TC-969",
        "FR-058-AC-1",
        "FR-058-AC-3",
        "FR-058-AC-4",
        "FR-058-AC-5",
        "FR-060-AC-1",
        "FR-060-AC-3",
        "FR-065-AC-7"
    )]
    #[test]
    fn statements_written_after_merge_verify_and_preserve_the_gate_verdict() {
        let policy = role_policy();
        let approvals = approvals();
        let types = account_types();
        let before_merge = approval_gate::evaluate(&policy, &approvals, &types);
        let sink = CaptureSink::default();
        let ids = [1_u8, 2, 3].map(|byte| RecordId::from_random_bytes([byte; 16]));
        let (writer_verdict, written) = write_approval_statements(
            &sink,
            &FixtureSigner,
            ApprovalStatementWriteRequest {
                pull_request: PullRequestState::Merged(MergedPullRequest {
                    repository: REPOSITORY,
                    pull_request: PULL_REQUEST,
                    head_commit: HEAD,
                    merged_commit: MERGED,
                }),
                policy: &policy,
                approvals: &approvals,
                account_types: &types,
                approver_names: &display_names(),
                record_ids: &ids,
            },
        )
        .unwrap();

        assert_eq!(before_merge.status, approval_gate::GateStatus::Satisfied);
        assert_eq!(writer_verdict, before_merge);
        assert_eq!(written.len(), 3);
        assert_eq!(sink.0.borrow().len(), 3);
        for (index, output) in written.iter().enumerate() {
            assert_eq!(
                output.sink_result.status,
                crate::record_sink::sink::Status::Published
            );
            let record = &sink.0.borrow()[index];
            assert_eq!(record.public().kind, RecordKind::Approval);
            let value: Value = serde_json::from_slice(&output.statement_bytes).unwrap();
            assert_eq!(
                value["predicate"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>(),
                BTreeSet::from([
                    "source",
                    "approver",
                    "approver_name",
                    "role",
                    "meaning",
                    "signed_at",
                    "head_commit",
                    "pull_request",
                    "merged_commit"
                ])
            );
            assert_eq!(value["subject"][0]["name"], REPOSITORY);
            assert_eq!(value["subject"][0]["digest"]["gitCommit"], MERGED);
            assert_eq!(value["predicate"]["head_commit"], HEAD);
            assert_ne!(
                value["predicate"]["head_commit"],
                value["predicate"]["merged_commit"]
            );
            assert_eq!(value["predicateType"], APPROVAL_PREDICATE_TYPE);
            assert_eq!(value["predicate"]["signed_at"], "2026-11-14T18:00:00Z");
            let private = record.private().statement.as_ref().unwrap();
            assert_eq!(private["signature_bundle"], output.signature_bundle);
            assert_eq!(private["statement"], value);
        }

        let retained = written
            .iter()
            .map(|output| RetainedApprovalStatement {
                statement_bytes: output.statement_bytes.clone(),
                signature_bundle: output.signature_bundle.clone(),
                publish_outcome: ApprovalStatementPublishOutcome::Published,
            })
            .collect();
        let result = verify_approval_statements(
            &Reader(retained),
            "private-store",
            lookup(),
            &identity_policy(".github/workflows/statement-writer.yml"),
            &policy,
            &FixtureVerifier,
        );
        assert_eq!(result.status, ApprovalStatementStatus::Verified);
        assert_eq!(result.role_verdict, before_merge);
    }

    #[trace("TC-587", "FR-058-AC-6")]
    #[test]
    fn review_without_either_required_marker_yields_no_statement() {
        for body in [
            "Reviewed the change.\nRole: developer",
            "Reviewed the change.\nSigned-off-meaning: acceptable",
        ] {
            let policy = role_policy();
            let reviews = [ReviewSnapshot {
                review_id: 1,
                account: account("101"),
                account_type: AccountType::Human,
                state: ReviewState::Approved,
                submitted_body: body.to_owned(),
                body_edited: false,
                submitted_at: "2026-11-14T10:00:00Z".to_owned(),
                commit: HEAD.to_owned(),
                pull_request: PULL_REQUEST.to_owned(),
            }];
            let normalized = GitHubReviewAdapter.approvals(&policy, &reviews, HEAD);
            assert!(normalized.is_empty(), "invalid marker set accepted: {body}");
            let sink = CaptureSink::default();
            let signer = FixtureSigner;
            let (_, written) = write_approval_statements(
                &sink,
                &signer,
                ApprovalStatementWriteRequest {
                    pull_request: PullRequestState::Merged(MergedPullRequest {
                        repository: REPOSITORY,
                        pull_request: PULL_REQUEST,
                        head_commit: HEAD,
                        merged_commit: MERGED,
                    }),
                    policy: &policy,
                    approvals: &normalized,
                    account_types: &account_types(),
                    approver_names: &display_names(),
                    record_ids: &[],
                },
            )
            .expect("invalid review markers yield an empty statement set");
            assert!(written.is_empty());
            assert!(sink.0.borrow().is_empty());
        }
    }

    #[trace("TC-969", "FR-065-AC-7")]
    #[test]
    fn stale_head_approval_is_omitted_from_post_merge_statements() {
        let policy = role_policy();
        let mut review_set = approvals();
        review_set[0].commit = "3333333333333333333333333333333333333333".into();
        let current_head_reviews: Vec<_> = review_set
            .iter()
            .filter(|approval| approval.commit == HEAD)
            .cloned()
            .collect();
        let types = account_types();
        let expected_partial = approval_gate::evaluate(&policy, &current_head_reviews, &types);
        let sink = CaptureSink::default();
        let names = display_names();
        let ids = [1_u8, 2].map(|byte| RecordId::from_random_bytes([byte; 16]));
        let (partial, statements) = write_approval_statements(
            &sink,
            &FixtureSigner,
            ApprovalStatementWriteRequest {
                pull_request: PullRequestState::Merged(MergedPullRequest {
                    repository: REPOSITORY,
                    pull_request: PULL_REQUEST,
                    head_commit: HEAD,
                    merged_commit: MERGED,
                }),
                policy: &policy,
                approvals: &review_set,
                account_types: &types,
                approver_names: &names,
                record_ids: &ids,
            },
        )
        .unwrap();
        assert_eq!(partial, expected_partial);
        assert_eq!(statements.len(), 2);
        assert!(statements.iter().all(|item| item.account.id != "101"));
    }

    #[trace("TC-969", "FR-060-AC-3")]
    #[test]
    fn subsecond_approval_timestamp_is_refused_before_sink_handoff() {
        let policy = role_policy();
        let mut review_set = approvals();
        review_set[0].timestamp = "2026-11-14T10:00:00.500Z".into();
        let types = account_types();
        let names = display_names();
        let ids = [1_u8, 2, 3].map(|byte| RecordId::from_random_bytes([byte; 16]));
        let sink = CaptureSink::default();
        let error = write_approval_statements(
            &sink,
            &FixtureSigner,
            ApprovalStatementWriteRequest {
                pull_request: PullRequestState::Merged(MergedPullRequest {
                    repository: REPOSITORY,
                    pull_request: PULL_REQUEST,
                    head_commit: HEAD,
                    merged_commit: MERGED,
                }),
                policy: &policy,
                approvals: &review_set,
                account_types: &types,
                approver_names: &names,
                record_ids: &ids,
            },
        );
        assert_eq!(error, Err(StatementWriteError::Metadata));
        assert!(sink.0.borrow().is_empty());
    }

    #[trace("TC-968", "FR-065-AC-3")]
    #[test]
    fn writer_and_publisher_workflow_identity_policies_are_independent() {
        let payload = b"statement payload";
        let writer_bundle = format!(
            ".github/workflows/statement-writer.yml|{}",
            hex_encode(payload)
        );
        assert_eq!(
            FixtureVerifier.verify(
                payload,
                &writer_bundle,
                &identity_policy(".github/workflows/statement-writer.yml")
            ),
            Ok(())
        );
        assert_eq!(
            FixtureVerifier.verify(
                payload,
                &writer_bundle,
                &identity_policy(".github/workflows/record-publisher.yml")
            ),
            Err(VerificationFailure::UntrustedSigner)
        );
        let publisher_bundle = writer_bundle.replace("statement-writer", "record-publisher");
        assert_eq!(
            FixtureVerifier.verify(
                payload,
                &publisher_bundle,
                &identity_policy(".github/workflows/record-publisher.yml")
            ),
            Ok(())
        );
        assert_eq!(
            FixtureVerifier.verify(
                payload,
                &publisher_bundle,
                &identity_policy(".github/workflows/statement-writer.yml")
            ),
            Err(VerificationFailure::UntrustedSigner)
        );
    }

    #[trace("TC-971", "FR-065-AC-9", "FR-058-AC-9")]
    #[test]
    fn private_repository_approval_records_stay_off_public_log() {
        let policy = role_policy();
        let approvals = approvals();
        let types = account_types();
        let names = display_names();
        let signer = FixtureSigner;
        let ids = [4_u8, 5, 6].map(|byte| RecordId::from_random_bytes([byte; 16]));

        let private_store = SinkStore::default();
        let private_log = SinkLog::default();
        let private_lookup =
            SinkVisibility(BTreeMap::from([(REPOSITORY.into(), Visibility::Private)]));
        let private_record_signer = RecordSigner::default();
        let private_selection = SinkSelection {
            sink: SinkKind::PrivateStore,
            source: SelectionSource::Default,
            public_log_endpoint: Some(concat!("https:", "//transparency.invalid").into()),
            private_store_destination: Some("agent-ix/approval-records".into()),
            public_linear_teams: Vec::new(),
        };
        let private_ports = Ports {
            store: &private_store,
            lookup: &private_lookup,
            signer: &private_record_signer,
            client: &private_log,
        };
        let private_sink = SinkLayer::new(&private_selection, REPOSITORY, private_ports);
        let (_, private_written) = write_approval_statements(
            &private_sink,
            &signer,
            ApprovalStatementWriteRequest {
                pull_request: PullRequestState::Merged(MergedPullRequest {
                    repository: REPOSITORY,
                    pull_request: PULL_REQUEST,
                    head_commit: HEAD,
                    merged_commit: MERGED,
                }),
                policy: &policy,
                approvals: &approvals,
                account_types: &types,
                approver_names: &names,
                record_ids: &ids,
            },
        )
        .unwrap();
        assert!(
            private_written
                .iter()
                .all(|item| item.sink_result.status == crate::record_sink::sink::Status::Published)
        );
        assert_eq!(private_store.private.borrow().len(), 3);
        assert_eq!(private_store.public.borrow().len(), 3);
        assert_eq!(private_log.0.get(), 0);
        assert_eq!(private_record_signer.public_calls.get(), 0);
    }

    #[trace("TC-588", "FR-058-AC-8")]
    #[test]
    fn public_approval_record_contains_only_the_statement_digest() {
        let policy = role_policy();
        let reviews = approvals();
        let types = account_types();
        let names = display_names();
        let ids = [11_u8, 12, 13].map(|byte| RecordId::from_random_bytes([byte; 16]));
        let store = SinkStore::default();
        let log = SinkLog::default();
        let lookup = SinkVisibility(BTreeMap::from([(REPOSITORY.into(), Visibility::Public)]));
        let record_signer = RecordSigner::default();
        let selection = SinkSelection {
            sink: SinkKind::PublicLog,
            source: SelectionSource::Configured,
            public_log_endpoint: Some(concat!("https:", "//transparency.invalid").into()),
            private_store_destination: Some("agent-ix/approval-records".into()),
            public_linear_teams: Vec::new(),
        };
        let ports = Ports {
            store: &store,
            lookup: &lookup,
            signer: &record_signer,
            client: &log,
        };
        let sink = SinkLayer::new(&selection, REPOSITORY, ports);

        let (_, written) = write_approval_statements(
            &sink,
            &FixtureSigner,
            ApprovalStatementWriteRequest {
                pull_request: PullRequestState::Merged(MergedPullRequest {
                    repository: REPOSITORY,
                    pull_request: PULL_REQUEST,
                    head_commit: HEAD,
                    merged_commit: MERGED,
                }),
                policy: &policy,
                approvals: &reviews,
                account_types: &types,
                approver_names: &names,
                record_ids: &ids,
            },
        )
        .expect("public approval statements are written");

        assert_eq!(written.len(), 3);
        assert_eq!(store.private.borrow().len(), 3);
        assert!(store.public.borrow().is_empty());
        assert_eq!(log.0.get(), 3);
        let private_records = store.private.borrow();
        let public_records = log.1.borrow();
        for (private, public) in private_records.iter().zip(public_records.iter()) {
            assert_eq!(public.kind, RecordKind::Approval);
            assert_eq!(public.record_id, private.record_id);
            let stored = private
                .statement
                .as_ref()
                .expect("private statement retained");
            let statement = &stored["statement"];
            let canonical =
                serde_json_canonicalizer::to_vec(statement).expect("full statement canonicalizes");
            let digest = format!(
                "sha256:{}",
                sigstore_verify::crypto::sha256(&canonical).to_hex()
            );
            assert_eq!(
                public.hashes.statement.as_ref().map(|value| value.as_str()),
                Some(digest.as_str())
            );
            let public_text = serde_json::to_string(public).expect("public record serializes");
            let private_text = serde_json::to_string(private).expect("private record serializes");
            for member in ["approver", "approver_name", "pull_request", "meaning"] {
                let value = statement["predicate"][member]
                    .as_str()
                    .expect("private approval member is text");
                assert!(private_text.contains(value));
                assert!(!public_text.contains(value));
            }
        }
    }

    #[trace("TC-971", "FR-065-AC-9")]
    #[test]
    fn leak_refusal_retains_private_form_without_public_calls() {
        let policy = role_policy();
        let approvals = approvals();
        let types = account_types();
        let names = display_names();
        let signer = FixtureSigner;
        let mut private_repository_approval = approvals[0].clone();
        private_repository_approval.pull_request = "acme/secret#41".into();
        let mut private_repo_names = names.clone();
        private_repo_names.insert(private_repository_approval.account.clone(), "Alice".into());
        let public_store = SinkStore::default();
        let public_log = SinkLog::default();
        let public_lookup = SinkVisibility(BTreeMap::from([
            ("acme/publisher".into(), Visibility::Public),
            ("acme/secret".into(), Visibility::Private),
        ]));
        let public_record_signer = RecordSigner::default();
        let public_selection = SinkSelection {
            sink: SinkKind::PublicLog,
            source: SelectionSource::Configured,
            public_log_endpoint: Some(concat!("https:", "//transparency.invalid").into()),
            private_store_destination: Some("agent-ix/approval-records".into()),
            public_linear_teams: Vec::new(),
        };
        let public_ports = Ports {
            store: &public_store,
            lookup: &public_lookup,
            signer: &public_record_signer,
            client: &public_log,
        };
        let public_sink = SinkLayer::new(&public_selection, "acme/publisher", public_ports);
        let public_repo = "acme/secret";
        let (_, refused) = write_approval_statements(
            &public_sink,
            &signer,
            ApprovalStatementWriteRequest {
                pull_request: PullRequestState::Merged(MergedPullRequest {
                    repository: public_repo,
                    pull_request: "acme/secret#41",
                    head_commit: HEAD,
                    merged_commit: MERGED,
                }),
                policy: &policy,
                approvals: &[private_repository_approval],
                account_types: &types,
                approver_names: &private_repo_names,
                record_ids: &[RecordId::from_random_bytes([7; 16])],
            },
        )
        .unwrap();
        assert_eq!(refused.len(), 1);
        assert_eq!(
            refused[0].sink_result.error,
            Some(crate::record_sink::config::ErrorCode::PublicSinkPrivateReference)
        );
        assert_eq!(
            public_store.private.borrow().len(),
            1,
            "private-first retention remains intact"
        );
        assert!(public_store.public.borrow().is_empty());
        assert_eq!(public_log.0.get(), 0);
        assert_eq!(
            public_record_signer.public_calls.get(),
            0,
            "the leak refusal precedes public signing"
        );
    }

    #[test]
    #[trace("TC-117", "NFR-005-AC-2")]
    fn fixture_reader_uses_exact_approval_query_contract() {
        let reader = Reader::default();
        assert!(
            reader
                .lookup_approval_statements("store", lookup())
                .unwrap()
                .is_empty()
        );
    }
}
