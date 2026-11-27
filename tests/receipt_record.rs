// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Receipt statement production and sink selection (FR-074, FR-075).

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    process::Command,
    rc::Rc,
};

use engineering_assurance::{
    approval_gate::{AccountRef, RolePolicy},
    attestation_signing::{
        AuthenticatedWorkflowClaims, BundleVerifier, ExpectedSubject, STATEMENT_V1_TYPE,
        SourceWorkflowPolicy, SubjectKind, TransparencyRequirement, VerificationFailure,
        VerificationStatus, WorkflowIdentityPolicy, verify_statement_with_source,
    },
    attestation_verify::TEST_RESULT_PREDICATE,
    change_record::{
        AgentSession, ApprovalDecision, Plugin, ReviewKind, ReviewVerdict, Status, Ticket,
        TicketKind,
    },
    receipt::ExpectedReceiptSubject,
    receipt_record::{
        RECEIPT_GATE_RESULTS_PROTOCOL, RECEIPT_INPUT_PROTOCOL, ReceiptGateResult,
        ReceiptGateResults, ReceiptInput, ReceiptPublishContext, ReceiptRecordError,
        ReceiptRecordInput, ReceiptReview, build_receipt_statements, publish_receipt,
    },
    record_sink::{
        config::{ErrorCode, SelectionSource, SinkKind, SinkSelection, Visibility},
        form::{PrivateRecord, PublicRecord, RecordId},
        leak::{LookupFailed, PublicPathProof, PublicStatementProof, VisibilityLookup},
        sink::{
            ApprovalStatementLookup, ApprovalStatementPublishOutcome, ApprovalStatementReader,
            CheckedEntry, LogRejected, PrivateStore, PublicLogClient, RetainedApprovalStatement,
            Signer, SigningFailed, SinkLayer, StoreError, StorePreparation,
        },
    },
};
use ix_trace_rs::trace;
use serde_json::{Value, json};

const REPO: &str = "fictional/project";
const PR: &str = "fictional/project#17";
const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const TREE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[derive(Default)]
struct Store {
    events: RefCell<Vec<&'static str>>,
    order: Option<Rc<RefCell<Vec<&'static str>>>>,
    private: RefCell<Vec<PrivateRecord>>,
    public: RefCell<Vec<PublicRecord>>,
}

impl ApprovalStatementReader for Store {}

impl PrivateStore for Store {
    fn write_private(&self, _destination: &str, private: &PrivateRecord) -> Result<(), StoreError> {
        self.events.borrow_mut().push("private-write");
        if let Some(order) = &self.order {
            order.borrow_mut().push("private-write");
        }
        self.private.borrow_mut().push(private.clone());
        Ok(())
    }

    fn prepare_record(
        &self,
        _destination: &str,
        private: &PrivateRecord,
        _public: &PublicRecord,
        _sink: SinkKind,
    ) -> Result<StorePreparation, StoreError> {
        self.events.borrow_mut().push("private-prepare");
        if let Some(order) = &self.order {
            order.borrow_mut().push("private-prepare");
        }
        self.private.borrow_mut().push(private.clone());
        Ok(StorePreparation::NewRecord)
    }

    fn write_public(&self, _destination: &str, public: &PublicRecord) -> Result<(), StoreError> {
        self.events.borrow_mut().push("public-write");
        self.public.borrow_mut().push(public.clone());
        Ok(())
    }

    fn write_publish_outcome(
        &self,
        _destination: &str,
        _record_id: &RecordId,
        _sink: SinkKind,
        _outcome: ApprovalStatementPublishOutcome,
    ) -> Result<(), StoreError> {
        self.events.borrow_mut().push("outcome");
        Ok(())
    }
}

struct Lookup(Visibility);

impl VisibilityLookup for Lookup {
    fn repo_visibility(&self, _repo: &str) -> Result<Visibility, LookupFailed> {
        Ok(self.0)
    }
}

struct Log(
    Rc<RefCell<Vec<&'static str>>>,
    Rc<RefCell<Vec<PublicRecord>>>,
);

impl PublicLogClient for Log {
    fn send(&self, _endpoint: &str, entry: &CheckedEntry) -> Result<(), LogRejected> {
        self.0.borrow_mut().push("log-send");
        self.1.borrow_mut().push(entry.record().clone());
        Ok(())
    }
}

type IdentityRegistry = Rc<RefCell<BTreeMap<String, String>>>;

struct TestSigner {
    events: Rc<RefCell<Vec<&'static str>>>,
    identity: String,
    identities: IdentityRegistry,
    private_signing_fails: bool,
}

struct TestVerifier {
    identities: IdentityRegistry,
    source_ref: String,
}

fn workflow_identity(repository: &str) -> String {
    format!(
        concat!(
            "https:",
            "//github.com/{}/.github/workflows/change-receipt.yml@refs/heads/main"
        ),
        repository
    )
}

fn receipt_signer_policy(
    repository: &str,
    transparency: TransparencyRequirement,
) -> WorkflowIdentityPolicy {
    WorkflowIdentityPolicy {
        repository: repository.to_owned(),
        workflow_path: ".github/workflows/change-receipt.yml".to_owned(),
        references: BTreeSet::from(["refs/heads/main".to_owned()]),
        issuer: concat!("https:", "//token.actions.githubusercontent.com").to_owned(),
        transparency,
    }
}

fn receipt_source_policy() -> SourceWorkflowPolicy {
    SourceWorkflowPolicy {
        repository: REPO.to_owned(),
        workflow_path: ".github/workflows/change-receipt.yml".to_owned(),
        event: "push".to_owned(),
        source_ref: Some("refs/heads/main".to_owned()),
        source_ref_prefix: None,
    }
}

impl TestSigner {
    fn new(events: Rc<RefCell<Vec<&'static str>>>, identity: &str) -> (Self, TestVerifier) {
        let identities = Rc::new(RefCell::new(BTreeMap::new()));
        (
            Self {
                events,
                identity: identity.to_owned(),
                identities: Rc::clone(&identities),
                private_signing_fails: false,
            },
            TestVerifier {
                identities,
                source_ref: "refs/heads/main".to_owned(),
            },
        )
    }

    fn bundle(&self, payload: &[u8], kind: &'static str) -> Result<String, SigningFailed> {
        self.events.borrow_mut().push(kind);
        let envelope = sigstore_types::DsseEnvelope::new(
            "application/vnd.in-toto+json",
            sigstore_types::PayloadBytes::from(payload.to_vec()),
            sigstore_types::DsseSignature::new(
                sigstore_types::SignatureBytes::from(vec![0]),
                sigstore_types::KeyId::default(),
            ),
        );
        let bundle = sigstore_types::Bundle::new(
            sigstore_types::MediaType::Bundle0_3,
            sigstore_types::VerificationMaterial::new(
                sigstore_types::VerificationMaterialContent::PublicKey(
                    sigstore_types::PublicKeyIdentifier::new("fixture"),
                ),
            ),
            sigstore_types::SignatureContent::DsseEnvelope(envelope),
        )
        .to_json()
        .map_err(|_| SigningFailed)?;
        self.identities
            .borrow_mut()
            .insert(bundle.clone(), self.identity.clone());
        Ok(bundle)
    }

    fn record_bundle(&self, payload: &[u8], kind: &'static str) -> Result<String, SigningFailed> {
        use base64::Engine as _;

        let bundle = self.bundle(payload, kind)?;
        Ok(base64::engine::general_purpose::STANDARD.encode(bundle.as_bytes()))
    }

    fn with_private_signing_failure(mut self) -> Self {
        self.private_signing_fails = true;
        self
    }
}

impl BundleVerifier for TestVerifier {
    fn verify(
        &self,
        payload: &[u8],
        bundle_json: &str,
        policy: &WorkflowIdentityPolicy,
    ) -> Result<(), VerificationFailure> {
        let identity = self
            .identities
            .borrow()
            .get(bundle_json)
            .cloned()
            .ok_or(VerificationFailure::InvalidSignature)?;
        let expected = policy.references.iter().any(|reference| {
            identity
                == format!(
                    concat!("https:", "//github.com/{}/{}@{}"),
                    policy.repository, policy.workflow_path, reference
                )
        });
        let parsed = sigstore_types::Bundle::from_json(bundle_json)
            .map_err(|_| VerificationFailure::InvalidSignature)?;
        let sigstore_types::SignatureContent::DsseEnvelope(envelope) = parsed.content else {
            return Err(VerificationFailure::InvalidSignature);
        };
        if envelope.payload.as_ref() != payload {
            return Err(VerificationFailure::InvalidSignature);
        }
        if !expected {
            return Err(VerificationFailure::UntrustedSigner);
        }
        Ok(())
    }

    fn verify_with_claims(
        &self,
        payload: &[u8],
        bundle_json: &str,
        policy: &WorkflowIdentityPolicy,
    ) -> Result<Option<AuthenticatedWorkflowClaims>, VerificationFailure> {
        self.verify(payload, bundle_json, policy)?;
        Ok(Some(AuthenticatedWorkflowClaims {
            source_repository_uri: Some(format!(concat!("https:", "//github.com/{}"), REPO)),
            source_repository_ref: Some(self.source_ref.clone()),
            build_config_uri: Some(format!(
                "https:{}github.com/{}/.github/workflows/change-receipt.yml@{}",
                "//", REPO, self.source_ref
            )),
            build_trigger: Some("push".to_owned()),
        }))
    }
}

impl Signer for TestSigner {
    fn sign_for_public_log(&self, proof: &PublicPathProof<'_>) -> Result<String, SigningFailed> {
        self.record_bundle(proof.signed_bytes(), "public-record-sign")
    }

    fn sign_statement_for_public_log(
        &self,
        proof: &PublicStatementProof<'_>,
    ) -> Result<String, SigningFailed> {
        self.bundle(proof.statement_bytes(), "public-statement-sign")
    }

    fn sign_without_public_log(&self, bytes: &[u8]) -> Result<String, SigningFailed> {
        if self.private_signing_fails {
            self.events.borrow_mut().push("private-sign");
            return Err(SigningFailed);
        }
        self.bundle(bytes, "private-sign")
    }

    fn sign_record_without_public_log(&self, bytes: &[u8]) -> Result<String, SigningFailed> {
        if self.private_signing_fails {
            self.events.borrow_mut().push("private-record-sign");
            return Err(SigningFailed);
        }
        self.record_bundle(bytes, "private-record-sign")
    }

    fn signed_payload(&self, bundle: &str) -> Result<Vec<u8>, SigningFailed> {
        use base64::Engine as _;

        if bundle.starts_with('{') {
            let parsed = sigstore_types::Bundle::from_json(bundle).map_err(|_| SigningFailed)?;
            let sigstore_types::SignatureContent::DsseEnvelope(envelope) = parsed.content else {
                return Err(SigningFailed);
            };
            return Ok(envelope.payload.as_ref().to_vec());
        }
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(bundle)
            .map_err(|_| SigningFailed)?;
        let bundle_json = std::str::from_utf8(&decoded).map_err(|_| SigningFailed)?;
        let parsed = sigstore_types::Bundle::from_json(bundle_json).map_err(|_| SigningFailed)?;
        let sigstore_types::SignatureContent::DsseEnvelope(envelope) = parsed.content else {
            return Err(SigningFailed);
        };
        Ok(envelope.payload.as_ref().to_vec())
    }

    fn verification_bundle(&self, bundle: &str) -> Result<String, SigningFailed> {
        use base64::Engine as _;

        let decoded = base64::engine::general_purpose::STANDARD
            .decode(bundle)
            .map_err(|_| SigningFailed)?;
        String::from_utf8(decoded).map_err(|_| SigningFailed)
    }
}

fn input() -> ReceiptInput {
    ReceiptInput {
        protocol: RECEIPT_INPUT_PROTOCOL.to_owned(),
        pull_request: PR.to_owned(),
        ticket: Some(Ticket {
            kind: TicketKind::LinearIssue,
            id: "EA-108".to_owned(),
        }),
        ticket_text: Some("Fictional receipt input".to_owned()),
        criteria: vec!["AC-1".to_owned()],
        spec_criteria: vec!["FR-074-AC-1".to_owned()],
        reviews: vec![ReceiptReview {
            kind: ReviewKind::CodeReview,
            verdict: ReviewVerdict::Pass,
            artifact: "fictional review result".to_owned(),
        }],
        session: Some(AgentSession {
            harness: "fixture".to_owned(),
            model: "fixture-model".to_owned(),
            plugins: vec![Plugin {
                name: "fixture-plugin".to_owned(),
                purl: "pkg:generic/fixture-plugin@1".to_owned(),
            }],
            mcp_servers: vec![],
            facts: BTreeMap::new(),
        }),
    }
}

fn role_policy() -> RolePolicy {
    RolePolicy {
        protocol: "engineering-assurance.approval-policy/v1".to_owned(),
        required_roles: vec!["engineer".to_owned()],
        individuals: BTreeMap::from([(
            "alice".to_owned(),
            vec![AccountRef {
                source: "github".to_owned(),
                id: "123".to_owned(),
            }],
        )]),
        roles: BTreeMap::from([("engineer".to_owned(), vec!["alice".to_owned()])]),
        agents: vec![],
    }
}

fn test_result(failed: bool) -> Value {
    json!({
        "_type": STATEMENT_V1_TYPE,
        "subject": [{
            "name": REPO,
            "digest": {"gitCommit": COMMIT, "gitTree": TREE}
        }],
        "predicateType": TEST_RESULT_PREDICATE,
        "predicate": {
            "result": if failed { "FAILED" } else { "PASSED" },
            "configuration": {},
            "passedTests": if failed { vec![] } else { vec!["tc_receipt"] },
            "warnedTests": [],
            "failedTests": if failed { vec!["tc_receipt"] } else { vec![] }
        }
    })
}

fn statements_for_input(
    subject: &ExpectedReceiptSubject,
    receipt_input: Option<&ReceiptInput>,
) -> engineering_assurance::receipt_record::ReceiptStatements {
    let gate_results = ReceiptGateResults {
        protocol: RECEIPT_GATE_RESULTS_PROTOCOL.to_owned(),
        commit: COMMIT.to_owned(),
        results: vec![ReceiptGateResult {
            name: "focused".to_owned(),
            exit_code: 0,
        }],
    };
    build_receipt_statements(ReceiptRecordInput {
        subject,
        gate_results: &gate_results,
        expected_gate_names: &["focused"],
        test_result: &test_result(false),
        receipt_input,
        role_policy: &role_policy(),
        approvals: approvals(),
        record_id: record_id(),
        created_at: "2026-11-14T12:00:00Z",
        verifier_id: concat!(
            "https:",
            "//github.com/fictional/project/.github/workflows/change-receipt.yml@refs/heads/main"
        ),
        policy_uri: concat!(
            "https:",
            "//github.com/fictional/project/blob/main/.engineering-assurance/approval-policy.json"
        ),
    })
    .expect("receipt statements are valid")
}

fn subject() -> ExpectedReceiptSubject {
    ExpectedReceiptSubject {
        repository: REPO.to_owned(),
        commit: COMMIT.to_owned(),
        tree: TREE.to_owned(),
        merged_pull_request: PR.to_owned(),
    }
}

fn approvals() -> Vec<(String, ApprovalDecision)> {
    vec![(
        "engineer".to_owned(),
        ApprovalDecision::Covered {
            individual: "alice".to_owned(),
            is_human: true,
            statement: json!({"fixture": "verified approval statement"}),
        },
    )]
}

fn record_id() -> RecordId {
    RecordId::from_random_bytes([9; 16])
}

fn receipt_statements_for_gate(
    subject: &ExpectedReceiptSubject,
    exit_code: i32,
) -> engineering_assurance::receipt_record::ReceiptStatements {
    let gate_results = ReceiptGateResults {
        protocol: RECEIPT_GATE_RESULTS_PROTOCOL.to_owned(),
        commit: COMMIT.to_owned(),
        results: vec![ReceiptGateResult {
            name: "focused".to_owned(),
            exit_code,
        }],
    };
    build_receipt_statements(ReceiptRecordInput {
        subject,
        gate_results: &gate_results,
        expected_gate_names: &["focused"],
        test_result: &test_result(false),
        receipt_input: Some(&input()),
        role_policy: &role_policy(),
        approvals: approvals(),
        record_id: record_id(),
        created_at: "2026-11-14T12:00:00Z",
        verifier_id: concat!(
            "https:",
            "//github.com/fictional/project/.github/workflows/change-receipt.yml@refs/heads/main"
        ),
        policy_uri: concat!(
            "https:",
            "//github.com/fictional/project/blob/main/.engineering-assurance/approval-policy.json"
        ),
    })
    .expect("receipt statements")
}

fn publish_private_receipt(
    subject: &ExpectedReceiptSubject,
    statements: engineering_assurance::receipt_record::ReceiptStatements,
) -> engineering_assurance::receipt_record::ReceiptRecordResult {
    publish_private_receipt_with_evidence(subject, statements).0
}

fn publish_private_receipt_with_evidence(
    subject: &ExpectedReceiptSubject,
    statements: engineering_assurance::receipt_record::ReceiptStatements,
) -> (
    engineering_assurance::receipt_record::ReceiptRecordResult,
    Vec<PrivateRecord>,
    Vec<&'static str>,
) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let store = Store {
        order: Some(Rc::clone(&events)),
        ..Store::default()
    };
    let (signer, verifier) = TestSigner::new(events.clone(), &workflow_identity(REPO));
    let lookup = Lookup(Visibility::Private);
    let log = Log(Rc::clone(&events), Rc::default());
    let selection = SinkSelection {
        sink: SinkKind::PrivateStore,
        source: SelectionSource::Configured,
        public_log_endpoint: None,
        private_store_destination: Some("fictional/records".to_owned()),
        public_linear_teams: vec![],
    };
    let ports = engineering_assurance::record_sink::sink::Ports {
        store: &store,
        lookup: &lookup,
        signer: &signer,
        client: &log,
    };
    let layer = SinkLayer::new(&selection, REPO, ports);
    let signer_policy = receipt_signer_policy(REPO, TransparencyRequirement::TimestampOnly);
    let source_policy = receipt_source_policy();
    let result = publish_receipt(
        &layer,
        &ReceiptPublishContext {
            verifier: &verifier,
            signer_policy: &signer_policy,
            source_policy: &source_policy,
            subject,
            record_id: &record_id(),
            created_at: "2026-11-14T12:00:00Z",
        },
        statements,
    )
    .expect("private signed receipt");
    assert_eq!(store.events.borrow()[0], "private-prepare");
    assert!(!events.borrow().contains(&"public-record-sign"));
    assert!(!events.borrow().contains(&"public-statement-sign"));
    assert!(events.borrow().contains(&"private-sign"));
    assert!(events.borrow().contains(&"private-record-sign"));
    assert!(events.borrow().iter().all(|event| *event != "log-send"));
    (
        result,
        store.private.borrow().clone(),
        events.borrow().clone(),
    )
}

fn statement_payload(bundle: &Value) -> Vec<u8> {
    let bundle_json = serde_json::to_string(bundle).expect("bundle JSON");
    let parsed = sigstore_types::Bundle::from_json(&bundle_json).expect("valid DSSE bundle");
    let sigstore_types::SignatureContent::DsseEnvelope(envelope) = parsed.content else {
        panic!("DSSE envelope");
    };
    envelope.payload.as_ref().to_vec()
}

#[trace("TC-583", "FR-074-AC-1", "FR-076-AC-5")]
#[test]
fn passing_input_produces_three_public_log_signed_statements_with_linked_summary() {
    let subject = subject();
    let retained_input = input();
    let retained_bytes = serde_json::to_vec(&retained_input).expect("receipt input JSON");
    let parsed_input = ReceiptInput::parse(&retained_bytes, PR).expect("exact PR input parses");
    assert_eq!(parsed_input, retained_input);
    assert!(ReceiptInput::parse(&retained_bytes, "fictional/project#18").is_err());
    assert!(ReceiptInput::parse(&vec![b' '; 8 * 1024 * 1024 + 1], PR).is_err());
    let statements = statements_for_input(&subject, Some(&parsed_input));
    assert_eq!(statements.status, Status::Passed);
    let change: Value = serde_json::from_slice(&statements.change_record).expect("change record");
    let test_digest = format!(
        "sha256:{}",
        engineering_assurance::change_record::canonical_digest(&test_result(false))
            .expect("canonical Test Result digest")
            .strip_prefix("sha256:")
            .expect("digest prefix")
    );
    assert_eq!(
        change
            .pointer("/predicate/test_results")
            .and_then(Value::as_str),
        Some(test_digest.as_str())
    );

    let (result, events, private_records, public_records) =
        publish_public_receipt(&subject, statements);
    assert_eq!(result.status, Status::Passed);
    assert!(result.bundles.change_record.is_object());
    assert!(result.bundles.test_result.is_object());
    assert!(result.bundles.summary.is_object());
    assert_receipt_summary_links(&result);
    assert_eq!(
        events,
        [
            "public-statement-sign",
            "public-statement-sign",
            "public-statement-sign",
            "public-record-sign",
            "log-send",
        ]
    );
    assert_eq!(private_records.len(), 1);
    assert_eq!(public_records.len(), 1);
    let public = &public_records[0];
    let private = &private_records[0];
    assert_eq!(
        public.kind,
        engineering_assurance::record_sink::form::RecordKind::ChangeRecord
    );
    assert_eq!(public.record_id, private.record_id);
    assert_eq!(
        public
            .hashes
            .subject_commit
            .as_ref()
            .map(engineering_assurance::record_sink::form::GitObjectId::as_str),
        Some(COMMIT)
    );
    let summary_bytes = statement_payload(&result.bundles.summary);
    let summary_statement: Value =
        serde_json::from_slice(&summary_bytes).expect("summary statement");
    let summary_digest = format!(
        "sha256:{}",
        sigstore_verify::crypto::sha256(
            &serde_json_canonicalizer::to_vec(&summary_statement).expect("canonical summary")
        )
        .to_hex()
    );
    assert_eq!(
        public
            .hashes
            .statement
            .as_ref()
            .map(engineering_assurance::record_sink::form::Sha256Digest::as_str),
        Some(summary_digest.as_str())
    );
    assert_eq!(
        public.verdicts.change_record,
        Some(engineering_assurance::record_sink::form::Verdict::Recorded)
    );
    assert!(public.timestamps.created.is_some());
    assert_eq!(
        private.ticket_text.as_deref(),
        Some("Fictional receipt input")
    );
    assert!(private.change_record.is_some());
    assert!(
        result
            .bundles
            .encode_note()
            .expect("three receipt bundles encode as a note")
            .ends_with(b"\n")
    );
}

#[trace("FR-075-AC-1")]
#[test]
fn squash_merged_pull_request_with_three_covering_approvals_passes_on_merge_commit() {
    fn git_text(repository: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(repository)
            .args(args)
            .output()
            .expect("git starts");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("git output is UTF-8")
            .trim()
            .to_owned()
    }

    let temp = tempfile::tempdir().expect("temporary fixture");
    let repository = temp.path().join("project");
    fs::create_dir(&repository).expect("fixture repository directory");
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&repository)
            .status()
            .expect("git init")
            .success()
    );
    git_text(&repository, &["config", "user.name", "Receipt Fixture"]);
    git_text(
        &repository,
        &["config", "user.email", "receipt@example.invalid"],
    );
    fs::write(repository.join("release.txt"), "base release\n").expect("base file");
    git_text(&repository, &["add", "release.txt"]);
    git_text(&repository, &["commit", "-qm", "base release"]);
    git_text(&repository, &["branch", "-M", "main"]);
    git_text(&repository, &["switch", "-qc", "feature/receipt-fixture"]);
    fs::write(repository.join("release.txt"), "squash-merged release\n").expect("feature file");
    git_text(&repository, &["commit", "-qam", "release change"]);
    git_text(&repository, &["switch", "-q", "main"]);
    git_text(
        &repository,
        &["merge", "--squash", "feature/receipt-fixture"],
    );
    git_text(
        &repository,
        &["commit", "-qm", "squash merge pull request 91"],
    );

    let commit = git_text(&repository, &["rev-parse", "HEAD"]);
    let tree = git_text(&repository, &["rev-parse", "HEAD^{tree}"]);
    let parents = git_text(&repository, &["rev-list", "--parents", "-n", "1", "HEAD"]);
    assert_eq!(
        parents.split_whitespace().count(),
        2,
        "squash merge has one parent"
    );

    let pull_request = "fictional/project#91".to_owned();
    let subject = ExpectedReceiptSubject {
        repository: REPO.to_owned(),
        commit: commit.clone(),
        tree: tree.clone(),
        merged_pull_request: pull_request.clone(),
    };
    let mut retained_input = input();
    retained_input.pull_request = pull_request.clone();
    let policy = RolePolicy {
        protocol: "engineering-assurance.approval-policy/v1".to_owned(),
        required_roles: vec![
            "engineer".to_owned(),
            "reviewer".to_owned(),
            "security".to_owned(),
        ],
        individuals: BTreeMap::from([
            (
                "alice".to_owned(),
                vec![AccountRef {
                    source: "github".to_owned(),
                    id: "123".to_owned(),
                }],
            ),
            (
                "bob".to_owned(),
                vec![AccountRef {
                    source: "github".to_owned(),
                    id: "456".to_owned(),
                }],
            ),
            (
                "carol".to_owned(),
                vec![AccountRef {
                    source: "github".to_owned(),
                    id: "789".to_owned(),
                }],
            ),
        ]),
        roles: BTreeMap::from([
            ("engineer".to_owned(), vec!["alice".to_owned()]),
            ("reviewer".to_owned(), vec!["bob".to_owned()]),
            ("security".to_owned(), vec!["carol".to_owned()]),
        ]),
        agents: vec![],
    };
    let role_approvals = [
        ("engineer", "alice"),
        ("reviewer", "bob"),
        ("security", "carol"),
    ];
    let approval_identity = concat!(
        "https:",
        "//github.com/fictional/project/.github/workflows/review-gate.yml@refs/heads/main"
    );
    let approval_events = Rc::new(RefCell::new(Vec::new()));
    let (approval_signer, approval_verifier) =
        TestSigner::new(approval_events, approval_identity);
    let retained_approvals = role_approvals
        .iter()
        .map(|(role, individual)| {
            let account_id = match *individual {
                "alice" => "123",
                "bob" => "456",
                "carol" => "789",
                _ => unreachable!("fixture policy individual"),
            };
            let statement = json!({
                "_type": STATEMENT_V1_TYPE,
                "subject": [{"name": REPO, "digest": {"gitCommit": commit}}],
                "predicateType": "https://schemas.agent-ix.ai/attestation/approval/v1",
                "predicate": {
                    "source": "github",
                    "approver": format!("github:{account_id}"),
                    "approver_name": individual,
                    "role": role,
                    "meaning": "approve",
                    "signed_at": "2026-11-14T11:00:00Z",
                    "head_commit": "cccccccccccccccccccccccccccccccccccccccc",
                    "pull_request": pull_request,
                    "merged_commit": commit
                }
            });
            let bytes = serde_json::to_vec(&statement).expect("approval statement JSON");
            let bundle = approval_signer
                .bundle(&bytes, "approval-statement-fixture")
                .expect("fixture approval bundle");
            RetainedApprovalStatement {
                statement_bytes: bytes,
                signature_bundle: bundle,
                publish_outcome: ApprovalStatementPublishOutcome::Published,
            }
        })
        .collect::<Vec<_>>();
    struct ApprovalFixture {
        statements: Vec<RetainedApprovalStatement>,
        pull_request: String,
        commit: String,
    }
    impl ApprovalStatementReader for ApprovalFixture {
        fn lookup_approval_statements(
            &self,
            _destination: &str,
            lookup: ApprovalStatementLookup<'_>,
        ) -> Result<Vec<RetainedApprovalStatement>, engineering_assurance::approval_statement::ApprovalStatementReadError> {
            if lookup.repository == REPO
                && lookup.merged_pull_request == self.pull_request
                && lookup.merged_commit == self.commit
            {
                Ok(self.statements.clone())
            } else {
                Ok(Vec::new())
            }
        }
    }
    let approval_reader = ApprovalFixture {
        statements: retained_approvals,
        pull_request: subject.merged_pull_request.clone(),
        commit: subject.commit.clone(),
    };
    let approval_identity_policy = WorkflowIdentityPolicy {
        repository: REPO.to_owned(),
        workflow_path: ".github/workflows/review-gate.yml".to_owned(),
        references: BTreeSet::from(["refs/heads/main".to_owned()]),
        issuer: concat!("https:", "//token.actions.githubusercontent.com").to_owned(),
        transparency: TransparencyRequirement::PublicLog,
    };
    let approvals = engineering_assurance::receipt_record::verified_receipt_approvals(
        &approval_reader,
        "fictional/approvals",
        &subject,
        &approval_identity_policy,
        &policy,
        &approval_verifier,
    );
    assert_eq!(approvals.len(), 3);
    assert!(
        approvals.iter().all(|(_, decision)| matches!(
            decision,
            ApprovalDecision::Covered { is_human: true, .. }
        )),
        "verified approvals should cover each distinct role: {approvals:?}"
    );
    let mut exported_test_result = test_result(false);
    exported_test_result["subject"][0]["digest"]["gitCommit"] = json!(commit);
    exported_test_result["subject"][0]["digest"]["gitTree"] = json!(tree);
    let gates = ReceiptGateResults {
        protocol: RECEIPT_GATE_RESULTS_PROTOCOL.to_owned(),
        commit: subject.commit.clone(),
        results: vec![ReceiptGateResult {
            name: "focused".to_owned(),
            exit_code: 0,
        }],
    };
    let statements = build_receipt_statements(ReceiptRecordInput {
        subject: &subject,
        gate_results: &gates,
        expected_gate_names: &["focused"],
        test_result: &exported_test_result,
        receipt_input: Some(&retained_input),
        role_policy: &policy,
        approvals,
        record_id: record_id(),
        created_at: "2026-11-14T12:00:00Z",
        verifier_id: concat!(
            "https:",
            "//github.com/fictional/project/.github/workflows/change-receipt.yml@refs/heads/main"
        ),
        policy_uri: concat!(
            "https:",
            "//github.com/fictional/project/blob/main/.engineering-assurance/approval-policy.json"
        ),
    })
    .expect("receipt statements build from the squash-merged subject");

    assert_eq!(statements.status, Status::Passed);
    let change: Value = serde_json::from_slice(&statements.change_record).expect("change record");
    assert_eq!(change["subject"][0]["digest"]["gitCommit"], commit);
    assert_eq!(change["subject"][0]["digest"]["gitTree"], tree);
    assert_eq!(change["predicate"]["status"], "passed");
    assert_eq!(
        change["predicate"]["approvals"].as_array().map(Vec::len),
        Some(3)
    );
    assert!(
        change["predicate"]["approvals"]
            .as_array()
            .expect("approval entries")
            .iter()
            .all(|approval| approval["verdict"] == "covered")
    );

    let (receipt, _private_records, _events) =
        publish_private_receipt_with_evidence(&subject, statements);
    assert_eq!(receipt.status, Status::Passed);
    assert_eq!(receipt.commit, subject.commit);
    let summary_statement: Value =
        serde_json::from_slice(&statement_payload(&receipt.bundles.summary))
            .expect("receipt summary statement");
    assert_eq!(
        summary_statement["subject"][0]["digest"]["gitCommit"],
        subject.commit
    );
    assert_eq!(
        summary_statement["predicate"]["verificationResult"],
        "PASSED"
    );
}

fn publish_public_receipt(
    subject: &ExpectedReceiptSubject,
    statements: engineering_assurance::receipt_record::ReceiptStatements,
) -> (
    engineering_assurance::receipt_record::ReceiptRecordResult,
    Vec<&'static str>,
    Vec<PrivateRecord>,
    Vec<PublicRecord>,
) {
    let (result, events, private, public) =
        publish_public_receipt_for_teams(subject, statements, vec!["EA".to_owned()]);
    (
        result.expect("public receipt is published"),
        events,
        private,
        public,
    )
}

fn publish_public_receipt_for_teams(
    subject: &ExpectedReceiptSubject,
    statements: engineering_assurance::receipt_record::ReceiptStatements,
    public_linear_teams: Vec<String>,
) -> (
    Result<engineering_assurance::receipt_record::ReceiptRecordResult, ReceiptRecordError>,
    Vec<&'static str>,
    Vec<PrivateRecord>,
    Vec<PublicRecord>,
) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let store = Store::default();
    let selection = SinkSelection {
        sink: SinkKind::PublicLog,
        source: SelectionSource::Configured,
        public_log_endpoint: Some(concat!("https:", "//log.example.test/api/v1").to_owned()),
        private_store_destination: Some("fictional/records".to_owned()),
        public_linear_teams,
    };
    let (signer, verifier) = TestSigner::new(events.clone(), &workflow_identity(REPO));
    let lookup = Lookup(Visibility::Public);
    let log = Log(Rc::clone(&events), Rc::default());
    let ports = engineering_assurance::record_sink::sink::Ports {
        store: &store,
        lookup: &lookup,
        signer: &signer,
        client: &log,
    };
    let layer = SinkLayer::new(&selection, REPO, ports);
    let signer_policy = receipt_signer_policy(REPO, TransparencyRequirement::PublicLog);
    let source_policy = receipt_source_policy();
    let publication_record_id = statements.private_form.record_id.clone();
    let result = publish_receipt(
        &layer,
        &ReceiptPublishContext {
            verifier: &verifier,
            signer_policy: &signer_policy,
            source_policy: &source_policy,
            subject,
            record_id: &publication_record_id,
            created_at: "2026-11-14T12:00:00Z",
        },
        statements,
    );
    assert_eq!(store.events.borrow()[0], "private-prepare");
    let events = events.borrow().clone();
    let private_records = store.private.borrow().clone();
    let public_records = log.1.borrow().clone();
    (result, events, private_records, public_records)
}

#[trace("TC-600", "FR-076-AC-4")]
#[test]
fn a_private_ticket_reference_is_retained_but_refused_before_publication() {
    let subject = subject();
    let mut private_input = input();
    private_input.ticket.as_mut().expect("fixture ticket").id = "SECRET-42".to_owned();
    let statements = statements_for_input(&subject, Some(&private_input));

    let (result, events, private_records, public_records) =
        publish_public_receipt_for_teams(&subject, statements, vec!["EA".to_owned()]);

    assert_eq!(
        result,
        Err(ReceiptRecordError::Sink(
            ErrorCode::PublicSinkPrivateReference
        ))
    );
    assert_eq!(private_records.len(), 1);
    assert_eq!(public_records.len(), 0);
    assert_eq!(
        private_records[0].ticket_text.as_deref(),
        Some("Fictional receipt input")
    );
    assert!(private_records[0].change_record.is_some());
    assert!(!events.contains(&"log-send"));
    assert!(!events.contains(&"public-record-sign"));
    assert!(!events.contains(&"public-statement-sign"));
}

fn assert_receipt_summary_links(
    result: &engineering_assurance::receipt_record::ReceiptRecordResult,
) {
    let change_bytes = statement_payload(&result.bundles.change_record);
    let test_bytes = statement_payload(&result.bundles.test_result);
    let summary_bytes = statement_payload(&result.bundles.summary);
    let summary: Value = serde_json::from_slice(&summary_bytes).expect("summary statement");
    let digest = |bytes: &[u8]| {
        let statement: Value = serde_json::from_slice(bytes).expect("statement JSON");
        format!(
            "sha256:{}",
            sigstore_verify::crypto::sha256(
                &serde_json_canonicalizer::to_vec(&statement).expect("canonical statement")
            )
            .to_hex()
        )
    };
    let linked = summary
        .pointer("/predicate/inputAttestations")
        .and_then(Value::as_array)
        .expect("summary input attestations")
        .iter()
        .filter_map(|item| item.pointer("/digest/sha256").and_then(Value::as_str))
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        linked,
        [digest(&change_bytes), digest(&test_bytes)]
            .into_iter()
            .collect()
    );
}

#[trace("TC-998", "FR-064-AC-25")]
#[test]
fn receipt_publication_refuses_a_valid_signature_from_the_wrong_workflow_identity() {
    let subject = subject();
    let gate_results = ReceiptGateResults {
        protocol: RECEIPT_GATE_RESULTS_PROTOCOL.to_owned(),
        commit: COMMIT.to_owned(),
        results: vec![ReceiptGateResult {
            name: "focused".to_owned(),
            exit_code: 0,
        }],
    };
    let statements = build_receipt_statements(ReceiptRecordInput {
        subject: &subject,
        gate_results: &gate_results,
        expected_gate_names: &["focused"],
        test_result: &test_result(false),
        receipt_input: Some(&input()),
        role_policy: &role_policy(),
        approvals: approvals(),
        record_id: record_id(),
        created_at: "2026-11-14T12:00:00Z",
        verifier_id: concat!(
            "https:",
            "//github.com/fictional/project/.github/workflows/change-receipt.yml@refs/heads/main"
        ),
        policy_uri: concat!(
            "https:",
            "//github.com/fictional/project/blob/main/.engineering-assurance/approval-policy.json"
        ),
    })
    .expect("receipt statements are valid");

    let events = Rc::new(RefCell::new(Vec::new()));
    let store = Store::default();
    let (signer, verifier) = TestSigner::new(
        events.clone(),
        &workflow_identity("fictional/other-workflow"),
    );
    let signer_policy = receipt_signer_policy(REPO, TransparencyRequirement::PublicLog);
    let source_policy = receipt_source_policy();
    let lookup = Lookup(Visibility::Public);
    let log = Log(Rc::clone(&events), Rc::default());
    let selection = SinkSelection {
        sink: SinkKind::PublicLog,
        source: SelectionSource::Configured,
        public_log_endpoint: Some(concat!("https:", "//log.example.test/api/v1").to_owned()),
        private_store_destination: Some("fictional/records".to_owned()),
        public_linear_teams: vec!["EA".to_owned()],
    };
    let ports = engineering_assurance::record_sink::sink::Ports {
        store: &store,
        lookup: &lookup,
        signer: &signer,
        client: &log,
    };
    let layer = SinkLayer::new(&selection, REPO, ports);
    let result = publish_receipt(
        &layer,
        &ReceiptPublishContext {
            verifier: &verifier,
            signer_policy: &signer_policy,
            source_policy: &source_policy,
            subject: &subject,
            record_id: &record_id(),
            created_at: "2026-11-14T12:00:00Z",
        },
        statements,
    );

    assert_eq!(
        result,
        Err(ReceiptRecordError::Sink(ErrorCode::SigningFailed))
    );
    assert_eq!(store.private.borrow().len(), 1);
    assert!(store.public.borrow().is_empty());
    assert!(!events.borrow().contains(&"public-record-sign"));
    assert!(!events.borrow().contains(&"log-send"));
}

#[trace("TC-1002", "FR-064-AC-14")]
#[test]
fn receipt_source_policy_accepts_default_branch_and_rejects_tag_for_same_signer() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let (signer, mut verifier) = TestSigner::new(events, &workflow_identity(REPO));
    let policy = receipt_signer_policy(REPO, TransparencyRequirement::PublicLog);
    let source_policy = receipt_source_policy();
    let payload = serde_json::to_vec(&json!({
        "_type": STATEMENT_V1_TYPE,
        "subject": [{"name": REPO, "digest": {"gitCommit": COMMIT}}],
        "predicateType": concat!("https:", "//example.invalid/receipt/v1"),
        "predicate": {}
    }))
    .expect("statement bytes");
    let bundle = signer
        .bundle(&payload, "source-policy-test")
        .expect("signed bundle");
    let subject = ExpectedSubject {
        kind: SubjectKind::Commit,
        value: COMMIT,
    };

    assert_eq!(
        verify_statement_with_source(
            &verifier,
            &payload,
            &bundle,
            &policy,
            Some(&source_policy),
            subject,
        ),
        Ok(VerificationStatus::Verified),
        "the protected default-branch source ref is accepted"
    );

    verifier.source_ref = "refs/tags/v0.10.0".to_owned();
    assert_eq!(
        verify_statement_with_source(
            &verifier,
            &payload,
            &bundle,
            &policy,
            Some(&source_policy),
            subject,
        ),
        Err(VerificationFailure::UntrustedSigner),
        "the same signer from a tag source ref is rejected"
    );
}

#[trace("TC-584", "FR-074-AC-2")]
#[test]
fn failed_gate_is_recorded_with_failed_summary() {
    let subject = subject();
    let exported_test_result = test_result(false);
    let gate_results = ReceiptGateResults {
        protocol: RECEIPT_GATE_RESULTS_PROTOCOL.to_owned(),
        commit: COMMIT.to_owned(),
        results: vec![ReceiptGateResult {
            name: "focused".to_owned(),
            exit_code: 1,
        }],
    };
    let statements = build_receipt_statements(ReceiptRecordInput {
        subject: &subject,
        gate_results: &gate_results,
        expected_gate_names: &["focused"],
        test_result: &exported_test_result,
        receipt_input: Some(&input()),
        role_policy: &role_policy(),
        approvals: approvals(),
        record_id: record_id(),
        created_at: "2026-11-14T12:00:00Z",
        verifier_id: concat!(
            "https:",
            "//github.com/fictional/project/.github/workflows/change-receipt.yml@refs/heads/main"
        ),
        policy_uri: concat!(
            "https:",
            "//github.com/fictional/project/blob/main/.engineering-assurance/approval-policy.json"
        ),
    })
    .expect("failed statements still produce receipt content");
    let omitted_gate = build_receipt_statements(ReceiptRecordInput {
        subject: &subject,
        gate_results: &gate_results,
        expected_gate_names: &["protected-gate"],
        test_result: &test_result(true),
        receipt_input: Some(&input()),
        role_policy: &role_policy(),
        approvals: approvals(),
        record_id: record_id(),
        created_at: "2026-11-14T12:00:00Z",
        verifier_id: concat!(
            "https:",
            "//github.com/fictional/project/.github/workflows/change-receipt.yml@refs/heads/main"
        ),
        policy_uri: concat!(
            "https:",
            "//github.com/fictional/project/blob/main/.engineering-assurance/approval-policy.json"
        ),
    });
    assert!(matches!(omitted_gate, Err(ReceiptRecordError::GateResults)));
    assert_eq!(statements.status, Status::Failed);
    assert_eq!(
        statements.test_result,
        serde_json_canonicalizer::to_vec(&exported_test_result)
            .expect("canonical Quoin Test Result bytes")
    );
    let change: Value = serde_json::from_slice(&statements.change_record).expect("change record");
    assert_eq!(
        change.pointer("/predicate/failures/0"),
        Some(&json!("gate_failed"))
    );
    let summary: Value = serde_json::from_slice(&statements.summary).expect("summary");
    assert_eq!(
        summary.pointer("/predicate/verificationResult"),
        Some(&json!("FAILED"))
    );
}

#[trace("FR-074-AC-5", "FR-074-AC-6")]
#[test]
fn private_store_retains_full_private_form_before_signing_passed_and_failed_receipts() {
    let subject = subject();
    for (exit_code, expected_status) in [(0, Status::Passed), (1, Status::Failed)] {
        let (result, private_records, order) = publish_private_receipt_with_evidence(
            &subject,
            receipt_statements_for_gate(&subject, exit_code),
        );
        assert_eq!(result.status, expected_status);
        assert_eq!(private_records.len(), 1);
        assert_eq!(
            private_records[0].ticket_text.as_deref(),
            Some("Fictional receipt input")
        );
        assert_eq!(
            private_records[0].model_names.as_deref(),
            Some(["fixture-model".to_owned()].as_slice())
        );
        assert!(private_records[0].change_record.is_some());
        assert!(private_records[0].agent_bom.is_some());
        assert_eq!(order.first(), Some(&"private-prepare"));
        assert!(
            order.iter().position(|event| *event == "private-prepare")
                < order.iter().position(|event| *event == "private-sign")
        );
    }
}

#[trace("FR-074-AC-6")]
#[test]
fn private_receipt_signer_failure_never_calls_the_public_log() {
    let subject = subject();
    let statements = receipt_statements_for_gate(&subject, 0);
    let events = Rc::new(RefCell::new(Vec::new()));
    let store = Store {
        order: Some(Rc::clone(&events)),
        ..Store::default()
    };
    let (signer, verifier) = TestSigner::new(events.clone(), &workflow_identity(REPO));
    let signer = signer.with_private_signing_failure();
    let lookup = Lookup(Visibility::Private);
    let log = Log(Rc::clone(&events), Rc::default());
    let selection = SinkSelection {
        sink: SinkKind::PrivateStore,
        source: SelectionSource::Configured,
        public_log_endpoint: None,
        private_store_destination: Some("fictional/records".to_owned()),
        public_linear_teams: vec![],
    };
    let ports = engineering_assurance::record_sink::sink::Ports {
        store: &store,
        lookup: &lookup,
        signer: &signer,
        client: &log,
    };
    let layer = SinkLayer::new(&selection, REPO, ports);
    let signer_policy = receipt_signer_policy(REPO, TransparencyRequirement::TimestampOnly);
    let source_policy = receipt_source_policy();
    let result = publish_receipt(
        &layer,
        &ReceiptPublishContext {
            verifier: &verifier,
            signer_policy: &signer_policy,
            source_policy: &source_policy,
            subject: &subject,
            record_id: &record_id(),
            created_at: "2026-11-14T12:00:00Z",
        },
        statements,
    );

    assert_eq!(result, Err(ReceiptRecordError::PrivateSigningUnavailable));
    assert_eq!(store.private.borrow().len(), 1);
    assert_eq!(
        store.private.borrow()[0].ticket_text.as_deref(),
        Some("Fictional receipt input")
    );
    assert_eq!(events.borrow().first(), Some(&"private-prepare"));
    assert!(!events.borrow().contains(&"log-send"));
    assert!(!events.borrow().contains(&"public-record-sign"));
}

#[trace("FR-074-AC-7", "FR-075-AC-7")]
#[test]
fn public_private_store_and_none_sinks_are_refused_before_retention() {
    let subject = subject();
    for (visibility, sink, expected_error) in [
        (
            Visibility::Public,
            SinkKind::PrivateStore,
            ReceiptRecordError::PublicLogRequired,
        ),
        (
            Visibility::Public,
            SinkKind::None,
            ReceiptRecordError::PrivateStoreRequired,
        ),
        (
            Visibility::Private,
            SinkKind::None,
            ReceiptRecordError::PrivateStoreRequired,
        ),
    ] {
        let store = Store::default();
        let events = Rc::new(RefCell::new(Vec::new()));
        let (signer, verifier) = TestSigner::new(events.clone(), &workflow_identity(REPO));
        let lookup = Lookup(visibility);
        let log = Log(Rc::clone(&events), Rc::default());
        let selection = SinkSelection {
            sink,
            source: SelectionSource::Configured,
            public_log_endpoint: None,
            private_store_destination: (sink != SinkKind::None)
                .then(|| "fictional/records".to_owned()),
            public_linear_teams: vec![],
        };
        let ports = engineering_assurance::record_sink::sink::Ports {
            store: &store,
            lookup: &lookup,
            signer: &signer,
            client: &log,
        };
        let layer = SinkLayer::new(&selection, REPO, ports);
        let signer_policy = receipt_signer_policy(REPO, TransparencyRequirement::TimestampOnly);
        let source_policy = receipt_source_policy();
        let result = publish_receipt(
            &layer,
            &ReceiptPublishContext {
                verifier: &verifier,
                signer_policy: &signer_policy,
                source_policy: &source_policy,
                subject: &subject,
                record_id: &record_id(),
                created_at: "2026-11-14T12:00:00Z",
            },
            receipt_statements_for_gate(&subject, 0),
        );

        assert_eq!(result, Err(expected_error));
        assert!(store.private.borrow().is_empty());
        assert!(events.borrow().is_empty());
    }
}

#[trace("TC-585", "FR-074-AC-3", "FR-075-AC-2")]
#[test]
fn absent_receipt_input_and_approvals_yield_a_failed_record() {
    let subject = subject();
    let gate_results = ReceiptGateResults {
        protocol: RECEIPT_GATE_RESULTS_PROTOCOL.to_owned(),
        commit: COMMIT.to_owned(),
        results: vec![ReceiptGateResult {
            name: "focused".to_owned(),
            exit_code: 0,
        }],
    };
    let statements = build_receipt_statements(ReceiptRecordInput {
        subject: &subject,
        gate_results: &gate_results,
        expected_gate_names: &["focused"],
        test_result: &test_result(false),
        receipt_input: None,
        role_policy: &role_policy(),
        approvals: approvals(),
        record_id: record_id(),
        created_at: "2026-11-14T12:00:00Z",
        verifier_id: concat!(
            "https:",
            "//github.com/fictional/project/.github/workflows/change-receipt.yml@refs/heads/main"
        ),
        policy_uri: concat!(
            "https:",
            "//github.com/fictional/project/blob/main/.engineering-assurance/approval-policy.json"
        ),
    })
    .expect("missing contract still produces receipt content");
    assert_eq!(statements.status, Status::Failed);
    let change: Value = serde_json::from_slice(&statements.change_record).expect("change record");
    let failures = change
        .pointer("/predicate/failures")
        .and_then(Value::as_array)
        .expect("failures");
    assert!(failures.contains(&json!("contract_missing")));
    assert!(!failures.contains(&json!("approvals_incomplete")));
    let published = publish_private_receipt(&subject, statements);
    assert_eq!(published.status, Status::Failed);

    let mut direct_push = subject;
    direct_push.merged_pull_request.clear();
    let direct_statements = build_receipt_statements(ReceiptRecordInput {
        subject: &direct_push,
        gate_results: &gate_results,
        expected_gate_names: &["focused"],
        test_result: &test_result(false),
        receipt_input: None,
        role_policy: &role_policy(),
        approvals: vec![],
        record_id: record_id(),
        created_at: "2026-11-14T12:00:00Z",
        verifier_id: concat!(
            "https:",
            "//github.com/fictional/project/.github/workflows/change-receipt.yml@refs/heads/main"
        ),
        policy_uri: concat!(
            "https:",
            "//github.com/fictional/project/blob/main/.engineering-assurance/approval-policy.json"
        ),
    })
    .expect("direct push still produces a failed receipt");
    let direct_change: Value =
        serde_json::from_slice(&direct_statements.change_record).expect("direct change record");
    assert_eq!(direct_statements.status, Status::Failed);
    assert!(
        direct_change
            .pointer("/predicate/failures")
            .and_then(Value::as_array)
            .is_some_and(|failures| failures.contains(&json!("approvals_incomplete")))
    );
    let direct_result = publish_private_receipt(&direct_push, direct_statements);
    assert_eq!(direct_result.status, Status::Failed);
}
