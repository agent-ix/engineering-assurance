// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Release-attestation record routing through the configured sinks (FR-071).

use std::{cell::RefCell, collections::BTreeSet};

use engineering_assurance::record_sink::{
    config::{ErrorCode, SelectionSource, SinkKind, SinkSelection, Visibility},
    form::{PrivateRecord, PublicRecord, Record, RecordId, Reference, References},
    leak::{LookupFailed, VisibilityLookup},
    sink::{
        ApprovalStatementReader, CheckedEntry, LogRejected, Ports, PrivateStore, PublicLogClient,
        Signer, SigningFailed, SinkLayer, StoreError,
    },
};
use engineering_assurance::release_attestation_record::{
    ReleaseOutcome, build_release_attestation_record, publish_release_attestation,
};
use ix_trace_rs::trace;
use serde_json::json;

const REPOSITORY: &str = "fictional/release";
const COMMIT: &str = "0123456789012345678901234567890123456789";
const SUMMARY_DIGEST: &str =
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[derive(Default)]
struct Store {
    private: RefCell<Vec<PrivateRecord>>,
    public: RefCell<Vec<PublicRecord>>,
}

impl ApprovalStatementReader for Store {}

impl PrivateStore for Store {
    fn write_private(&self, _: &str, record: &PrivateRecord) -> Result<(), StoreError> {
        self.private.borrow_mut().push(record.clone());
        Ok(())
    }

    fn write_public(&self, _: &str, record: &PublicRecord) -> Result<(), StoreError> {
        self.public.borrow_mut().push(record.clone());
        Ok(())
    }
}

struct Lookup(Visibility);

impl VisibilityLookup for Lookup {
    fn repo_visibility(&self, _: &str) -> Result<Visibility, LookupFailed> {
        Ok(self.0)
    }
}

#[derive(Default)]
struct Log {
    entries: RefCell<Vec<PublicRecord>>,
}

impl PublicLogClient for Log {
    fn send(&self, _: &str, entry: &CheckedEntry) -> Result<(), LogRejected> {
        self.entries.borrow_mut().push(entry.record().clone());
        Ok(())
    }
}

#[derive(Default)]
struct TestSigner {
    signed_payload: RefCell<Option<Vec<u8>>>,
    calls: RefCell<Vec<&'static str>>,
    private_signing_fails: bool,
}

impl TestSigner {
    fn bundle(&self, bytes: &[u8], call: &'static str) -> String {
        self.calls.borrow_mut().push(call);
        *self.signed_payload.borrow_mut() = Some(bytes.to_vec());
        "c2ln".to_owned()
    }
}

impl Signer for TestSigner {
    fn sign_for_public_log(
        &self,
        proof: &engineering_assurance::record_sink::leak::PublicPathProof<'_>,
    ) -> Result<String, SigningFailed> {
        Ok(self.bundle(proof.signed_bytes(), "public"))
    }

    fn sign_statement_for_public_log(
        &self,
        proof: &engineering_assurance::record_sink::leak::PublicStatementProof<'_>,
    ) -> Result<String, SigningFailed> {
        Ok(self.bundle(proof.statement_bytes(), "public-statement"))
    }

    fn sign_without_public_log(&self, bytes: &[u8]) -> Result<String, SigningFailed> {
        if self.private_signing_fails {
            self.calls.borrow_mut().push("private-failed");
            return Err(SigningFailed);
        }
        Ok(self.bundle(bytes, "private"))
    }

    fn sign_record_without_public_log(&self, bytes: &[u8]) -> Result<String, SigningFailed> {
        if self.private_signing_fails {
            self.calls.borrow_mut().push("private-failed");
            return Err(SigningFailed);
        }
        Ok(self.bundle(bytes, "private-record"))
    }

    fn signed_payload(&self, _: &str) -> Result<Vec<u8>, SigningFailed> {
        self.signed_payload.borrow().clone().ok_or(SigningFailed)
    }
}

fn record(references: References) -> Record {
    build_release_attestation_record(
        REPOSITORY,
        RecordId::from_random_bytes([1; 16]),
        SUMMARY_DIGEST,
        COMMIT,
        "2026-11-26T12:00:00Z",
        references,
        json!({
            "summary": {"verificationResult": "PASSED"},
            "inputAttestations": ["provenance", "sbom", "test-result"]
        }),
    )
    .expect("valid release-attestation record")
}

fn with_sink(
    kind: SinkKind,
    visibility: Visibility,
    private_signing_fails: bool,
    run: impl FnOnce(&SinkLayer<'_>, &Store, &Log, &TestSigner),
) {
    let store = Store::default();
    let lookup = Lookup(visibility);
    let signer = TestSigner {
        private_signing_fails,
        ..TestSigner::default()
    };
    let log = Log::default();
    let selection = SinkSelection {
        sink: kind,
        source: SelectionSource::Configured,
        public_log_endpoint: (kind == SinkKind::PublicLog)
            .then(|| concat!("https:", "//transparency.invalid").to_owned()),
        private_store_destination: (kind != SinkKind::None).then(|| "fictional/records".to_owned()),
        public_linear_teams: Vec::new(),
    };
    let layer = SinkLayer::new(
        &selection,
        REPOSITORY,
        Ports {
            store: &store,
            lookup: &lookup,
            signer: &signer,
            client: &log,
        },
    );
    run(&layer, &store, &log, &signer);
}

#[trace("TC-564", "FR-071-AC-1")]
#[test]
fn tc_564_public_release_record_contains_only_the_summary_facts() {
    with_sink(
        SinkKind::PublicLog,
        Visibility::Public,
        false,
        |sink, store, log, _| {
            let private_summary = json!({"summary": {"verificationResult": "PASSED"}});
            let record = build_release_attestation_record(
                REPOSITORY,
                RecordId::from_random_bytes([2; 16]),
                SUMMARY_DIGEST,
                COMMIT,
                "2026-11-26T12:00:00Z",
                References::default(),
                private_summary.clone(),
            )
            .expect("release record");
            let result = publish_release_attestation(
                sink,
                Visibility::Public,
                ReleaseOutcome::Released,
                &record,
            )
            .expect("released record is routed");
            assert_eq!(
                result.status,
                engineering_assurance::record_sink::sink::Status::Published
            );
            assert_eq!(log.entries.borrow().len(), 1);
            let public = serde_json::to_value(&log.entries.borrow()[0]).expect("public JSON");
            assert_eq!(public["kind"], "release_attestation");
            assert_eq!(public["hashes"]["statement"], SUMMARY_DIGEST);
            assert_eq!(public["hashes"]["subject_commit"], COMMIT);
            assert_eq!(public["verdicts"]["release"], "released");
            assert_eq!(public["timestamps"]["created"], "2026-11-26T12:00:00Z");
            assert_eq!(
                public["hashes"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .cloned()
                    .collect::<BTreeSet<_>>(),
                BTreeSet::from(["statement".to_owned(), "subject_commit".to_owned()])
            );
            assert_eq!(public["verdicts"].as_object().unwrap().len(), 1);
            assert_eq!(public["timestamps"].as_object().unwrap().len(), 1);
            assert_eq!(
                store.private.borrow()[0].statement.as_ref(),
                Some(&private_summary)
            );
        },
    );
}

#[trace("TC-565", "FR-071-AC-2")]
#[test]
fn tc_565_private_linear_reference_refuses_before_public_signing() {
    let private_issue = Reference::linear_issue("SECRET-7").expect("private issue ref");
    let references = References::new(vec![private_issue]).expect("reference list");
    with_sink(
        SinkKind::PublicLog,
        Visibility::Public,
        false,
        |sink, store, log, signer| {
            let record = record(references);
            let result = publish_release_attestation(
                sink,
                Visibility::Public,
                ReleaseOutcome::Released,
                &record,
            )
            .expect("release route result");
            assert_eq!(result.error, Some(ErrorCode::PublicSinkPrivateReference));
            assert_eq!(store.private.borrow().len(), 1);
            assert!(signer.calls.borrow().is_empty());
            assert!(log.entries.borrow().is_empty());
        },
    );
}

#[trace("TC-566", "FR-071-AC-3")]
#[test]
fn tc_566_public_repository_refuses_the_private_store_before_any_write() {
    with_sink(
        SinkKind::PrivateStore,
        Visibility::Public,
        false,
        |sink, store, log, signer| {
            let record = record(References::default());
            let result = publish_release_attestation(
                sink,
                Visibility::Public,
                ReleaseOutcome::Released,
                &record,
            )
            .expect("release route result");
            assert_eq!(
                result.error,
                Some(ErrorCode::ReleaseAttestationPublicLogRequired)
            );
            assert!(store.private.borrow().is_empty());
            assert!(store.public.borrow().is_empty());
            assert!(signer.calls.borrow().is_empty());
            assert!(log.entries.borrow().is_empty());
        },
    );
}

#[trace("TC-567", "FR-071-AC-4")]
#[test]
fn tc_567_private_repository_uses_private_signing_and_never_the_public_log() {
    with_sink(
        SinkKind::PrivateStore,
        Visibility::Private,
        false,
        |sink, store, log, signer| {
            let record = record(References::default());
            let result = publish_release_attestation(
                sink,
                Visibility::Private,
                ReleaseOutcome::Released,
                &record,
            )
            .expect("release route result");
            assert_eq!(
                result.status,
                engineering_assurance::record_sink::sink::Status::Published
            );
            assert_eq!(signer.calls.borrow().as_slice(), ["private-record"]);
            assert_eq!(store.private.borrow().len(), 1);
            assert_eq!(store.public.borrow().len(), 1);
            assert!(log.entries.borrow().is_empty());
        },
    );

    with_sink(
        SinkKind::PrivateStore,
        Visibility::Private,
        true,
        |sink, store, log, _| {
            let record = record(References::default());
            let result = publish_release_attestation(
                sink,
                Visibility::Private,
                ReleaseOutcome::Released,
                &record,
            )
            .expect("release route result");
            assert_eq!(
                result.error,
                Some(ErrorCode::ReleaseAttestationPrivateSigningUnavailable)
            );
            assert_eq!(store.private.borrow().len(), 1);
            assert!(store.public.borrow().is_empty());
            assert!(log.entries.borrow().is_empty());
        },
    );
}

#[trace("TC-568", "FR-071-AC-5")]
#[test]
fn tc_568_none_sink_refuses_for_both_visibility_values_without_calls() {
    for visibility in [Visibility::Public, Visibility::Private] {
        with_sink(
            SinkKind::None,
            visibility,
            false,
            |sink, store, log, signer| {
                let record = record(References::default());
                let result = publish_release_attestation(
                    sink,
                    visibility,
                    ReleaseOutcome::Released,
                    &record,
                )
                .expect("release route result");
                assert_eq!(
                    result.error,
                    Some(ErrorCode::ReleaseAttestationPrivateStoreRequired)
                );
                assert!(store.private.borrow().is_empty());
                assert!(store.public.borrow().is_empty());
                assert!(signer.calls.borrow().is_empty());
                assert!(log.entries.borrow().is_empty());
            },
        );
    }
}

#[trace("TC-569", "FR-071-AC-6")]
#[test]
fn tc_569_refused_release_retains_private_form_and_not_released_skips_the_sink() {
    let private_issue = Reference::linear_issue("SECRET-7").expect("private issue ref");
    let references = References::new(vec![private_issue]).expect("reference list");
    with_sink(
        SinkKind::PublicLog,
        Visibility::Public,
        false,
        |sink, store, log, signer| {
            let refused = record(references);
            let result = publish_release_attestation(
                sink,
                Visibility::Public,
                ReleaseOutcome::Released,
                &refused,
            )
            .expect("release route result");
            assert_eq!(result.error, Some(ErrorCode::PublicSinkPrivateReference));
            assert_eq!(
                store.private.borrow()[0].statement,
                refused.private().statement
            );
            let calls = signer.calls.borrow().len();
            let retained = store.private.borrow().len();
            assert!(
                publish_release_attestation(
                    sink,
                    Visibility::Public,
                    ReleaseOutcome::NotReleased,
                    &refused,
                )
                .is_none()
            );
            assert_eq!(store.private.borrow().len(), retained);
            assert_eq!(signer.calls.borrow().len(), calls);
            assert!(log.entries.borrow().is_empty());
        },
    );
}
