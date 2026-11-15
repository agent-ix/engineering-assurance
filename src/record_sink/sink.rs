// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The sink interface, its three implementations and the sink layer (FR-054).
//!
//! The sink layer writes the full private form first, then hands the record to
//! exactly one sink chosen from the repository's selection. A refusal is never
//! rerouted to another sink.

use serde::{Deserialize, Serialize};

#[cfg(feature = "attestation-verify")]
use crate::attestation_signing::{BundleVerifier, SourceWorkflowPolicy, WorkflowIdentityPolicy};

use super::config::{ErrorCode, SinkKind, SinkSelection, valid_repository};
#[cfg(feature = "receipt-record")]
use super::form::Sha256Digest;
use super::form::{PrivateRecord, PublicRecord, Record, RecordId, Signature};
#[cfg(feature = "attestation-verify")]
use super::leak::PublicStatementProof;
use super::leak::{CheckRefusal, Offending, PublicPathProof, VisibilityLookup, leak_check};

/// The only protocol literal of a publish result.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ResultProtocol {
    /// `engineering-assurance.sink-result/v1`.
    #[serde(rename = "engineering-assurance.sink-result/v1")]
    V1,
}

/// The outcome of a publish.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// The sink accepted the record.
    Published,
    /// The sink could not accept the record.
    Refused,
    /// The `none` sink took the record and kept nothing.
    Discarded,
}

/// One publish result. `error` is present on `refused`; `offending` only on the
/// leak check's refusal (FR-057).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawSinkResult")]
pub struct SinkResult {
    /// The protocol literal.
    pub protocol: ResultProtocol,
    /// The sink that handled the publish.
    pub sink: SinkKind,
    /// The outcome.
    pub status: Status,
    /// The record's id.
    pub record_id: RecordId,
    /// The refusal code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorCode>,
    /// Each item the leak check refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offending: Option<Vec<Offending>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSinkResult {
    protocol: ResultProtocol,
    sink: SinkKind,
    status: Status,
    record_id: RecordId,
    #[serde(default, deserialize_with = "super::form::no_null")]
    error: Option<ErrorCode>,
    #[serde(default, deserialize_with = "super::form::no_null")]
    offending: Option<Vec<Offending>>,
}

/// A publish result whose `error` or `offending` members do not fit its status.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[error("publish result members do not fit its status")]
pub struct ResultInvalid;

impl TryFrom<RawSinkResult> for SinkResult {
    type Error = ResultInvalid;

    fn try_from(raw: RawSinkResult) -> Result<Self, ResultInvalid> {
        let refused = raw.status == Status::Refused;
        let offending_fits =
            raw.offending.is_none() || raw.error == Some(ErrorCode::PublicSinkPrivateReference);
        if refused != raw.error.is_some() || !offending_fits {
            return Err(ResultInvalid);
        }
        Ok(Self {
            protocol: raw.protocol,
            sink: raw.sink,
            status: raw.status,
            record_id: raw.record_id,
            error: raw.error,
            offending: raw.offending,
        })
    }
}

impl SinkResult {
    fn new(sink: SinkKind, status: Status, record_id: &RecordId) -> Self {
        Self {
            protocol: ResultProtocol::V1,
            sink,
            status,
            record_id: record_id.clone(),
            error: None,
            offending: None,
        }
    }

    /// A `published` result.
    #[must_use]
    pub fn published(sink: SinkKind, record_id: &RecordId) -> Self {
        Self::new(sink, Status::Published, record_id)
    }

    /// A `discarded` result.
    #[must_use]
    pub fn discarded(sink: SinkKind, record_id: &RecordId) -> Self {
        Self::new(sink, Status::Discarded, record_id)
    }

    /// A `refused` result naming the error.
    #[must_use]
    pub fn refused(sink: SinkKind, record_id: &RecordId, error: ErrorCode) -> Self {
        Self {
            error: Some(error),
            ..Self::new(sink, Status::Refused, record_id)
        }
    }
}

/// The one operation of the sink interface.
pub trait RecordSink {
    /// Publishes a record and returns one result.
    fn publish(&self, record: &Record) -> SinkResult;
}

/// A private store write failed.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[error("private store write failed")]
pub struct StoreError;

/// Result of atomically preparing a record in a retained private store.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorePreparation {
    /// The record was created and one sink invocation is permitted.
    NewRecord,
    /// An identical record has a valid, signed `published` outcome.
    AlreadyPublished,
    /// An identical record has a valid, signed `refused` outcome.
    AlreadyRefused,
    /// The record exists, but does not have a complete trusted final outcome.
    OutcomeIncomplete,
}

/// One exact post-merge query to the private approval-statement store.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApprovalStatementLookup<'a> {
    /// The repository in `owner/name` form.
    pub repository: &'a str,
    /// The merged pull-request reference, such as `owner/name#12`.
    pub merged_pull_request: &'a str,
    /// The merged commit object id.
    pub merged_commit: &'a str,
}

/// Whether the statement's publishing operation succeeded or was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalStatementPublishOutcome {
    /// The sink accepted publication.
    Published,
    /// The sink refused publication, including a leak-check refusal.
    Refused,
}

/// One retained result returned by the approval-specific private-store query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedApprovalStatement {
    /// The exact statement bytes covered by the signature bundle.
    pub statement_bytes: Vec<u8>,
    /// The associated Sigstore bundle in JSON form.
    pub signature_bundle: String,
    /// The sink's outcome for publishing this statement.
    pub publish_outcome: ApprovalStatementPublishOutcome,
}

/// A private store that supports the narrow FR-064 approval lookup.
pub trait ApprovalStatementReader {
    /// Retrieve every retained statement matching the exact query tuple.
    ///
    /// The default refuses the lookup so a write-only private store fails
    /// closed until it implements retained approval reads.
    ///
    /// # Errors
    ///
    /// [`ApprovalStatementReadError`] when the private store cannot complete
    /// the lookup or does not implement it.
    fn lookup_approval_statements(
        &self,
        _destination: &str,
        _lookup: ApprovalStatementLookup<'_>,
    ) -> Result<Vec<RetainedApprovalStatement>, ApprovalStatementReadError> {
        Err(ApprovalStatementReadError)
    }
}

/// The private store could not complete an approval-statement lookup.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("private approval-statement lookup failed")]
pub struct ApprovalStatementReadError;

/// The private store: a destination named in the sink configuration.
pub trait PrivateStore: ApprovalStatementReader {
    /// Stores the full private form under `destination`.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the write fails.
    fn write_private(&self, destination: &str, private: &PrivateRecord) -> Result<(), StoreError>;

    /// Atomically prepares a record before its selected sink runs.
    ///
    /// Implementations with retained state override this method to enforce
    /// create-if-absent and replay semantics. The default preserves the
    /// original write-only adapter behavior.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the store cannot prepare the record.
    fn prepare_record(
        &self,
        destination: &str,
        private: &PrivateRecord,
        _public: &PublicRecord,
        _sink: SinkKind,
    ) -> Result<StorePreparation, StoreError> {
        self.write_private(destination, private)?;
        Ok(StorePreparation::NewRecord)
    }

    /// Stores the trimmed public form linked to a private form.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the write fails.
    fn write_public(&self, destination: &str, public: &PublicRecord) -> Result<(), StoreError>;

    /// Persists the final result after the selected sink returns.
    ///
    /// Concrete retained stores override this operation and sign the outcome
    /// with the delegated record-publisher identity.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the final result cannot be retained.
    fn write_publish_outcome(
        &self,
        _destination: &str,
        _record_id: &RecordId,
        _sink: SinkKind,
        _outcome: ApprovalStatementPublishOutcome,
    ) -> Result<(), StoreError> {
        Ok(())
    }
}

/// The public log rejected the entry or could not be reached.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[error("public log rejected the record")]
pub struct LogRejected;

/// A public form that passed the leak check and carries a verified signature.
/// Only [`PublicLogSink`] can make one, so the public log client receives
/// nothing that skipped the check.
#[derive(Debug)]
pub struct CheckedEntry {
    record: PublicRecord,
}

impl CheckedEntry {
    /// The signed public form to append to the log.
    #[must_use]
    pub const fn record(&self) -> &PublicRecord {
        &self.record
    }
}

/// The append-only public transparency log.
pub trait PublicLogClient {
    /// Sends one entry to the log at `endpoint`.
    ///
    /// # Errors
    ///
    /// [`LogRejected`] when the log does not accept the entry.
    fn send(&self, endpoint: &str, entry: &CheckedEntry) -> Result<(), LogRejected>;
}

/// The delegated signing stack failed.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[error("signing failed")]
pub struct SigningFailed;

/// The delegated Sigstore signing and verification stack (NFR-010). This crate
/// implements no signature primitive; it only chooses the path and checks what
/// the bundle signed.
pub trait Signer {
    /// Signs the bytes the proof carries ([`PublicPathProof::signed_bytes`])
    /// and uploads to the public transparency log. The proof binds both the
    /// path and the payload: only checked bytes can be signed for the log.
    ///
    /// # Errors
    ///
    /// [`SigningFailed`] when the stack fails.
    fn sign_for_public_log(&self, proof: &PublicPathProof<'_>) -> Result<String, SigningFailed>;

    /// Signs one canonical statement after its digest-bearing public record
    /// passed the fail-closed visibility check.
    ///
    /// Implementations that do not support statement signing for the public
    /// log fail closed by returning [`SigningFailed`].
    ///
    /// # Errors
    ///
    /// Returns [`SigningFailed`] when signing is unavailable or fails.
    #[cfg(feature = "attestation-verify")]
    fn sign_statement_for_public_log(
        &self,
        _proof: &PublicStatementProof<'_>,
    ) -> Result<String, SigningFailed> {
        Err(SigningFailed)
    }

    /// Signs `canonical` with a configuration that writes to no public log.
    ///
    /// # Errors
    ///
    /// [`SigningFailed`] when the stack fails.
    fn sign_without_public_log(&self, canonical: &[u8]) -> Result<String, SigningFailed>;

    /// Signs the public-form payload for a private-store record. The result is
    /// serialized in the public form's padded-base64 signature field.
    ///
    /// Implementations with a distinct bundle representation may override
    /// this boundary. The default preserves existing signer test doubles.
    ///
    /// # Errors
    ///
    /// Returns [`SigningFailed`] when signing is unavailable or fails.
    fn sign_record_without_public_log(&self, canonical: &[u8]) -> Result<String, SigningFailed> {
        self.sign_without_public_log(canonical)
    }

    /// The payload a base64 bundle signs.
    ///
    /// # Errors
    ///
    /// [`SigningFailed`] when the bundle cannot be decoded.
    fn signed_payload(&self, bundle: &str) -> Result<Vec<u8>, SigningFailed>;

    /// Returns the Sigstore bundle JSON used for identity verification.
    /// Implementations with an encoded public-form signature override this
    /// boundary; ordinary statement bundle implementations return themselves.
    ///
    /// # Errors
    ///
    /// Returns [`SigningFailed`] when the encoded bundle cannot be decoded.
    fn verification_bundle(&self, bundle: &str) -> Result<String, SigningFailed> {
        Ok(bundle.to_owned())
    }
}

/// Attaches `bundle` to a copy of `public` after checking it signs exactly the
/// canonical bytes of the public form.
fn attach_signature(
    verifier: &dyn Signer,
    public: &PublicRecord,
    canonical: &[u8],
    bundle: Result<String, SigningFailed>,
) -> Result<PublicRecord, ErrorCode> {
    let bundle = bundle.map_err(|_| ErrorCode::SigningFailed)?;
    let signature = Signature::try_from(bundle).map_err(|_| ErrorCode::SignatureInvalid)?;
    let payload = verifier
        .signed_payload(signature.as_str())
        .map_err(|_| ErrorCode::SigningFailed)?;
    if payload != canonical {
        return Err(ErrorCode::SignaturePayloadMismatch);
    }
    let mut attached = public.clone();
    attached.signature = Some(signature);
    Ok(attached)
}

/// The collaborators a sink layer talks to.
#[derive(Clone, Copy)]
pub struct Ports<'a> {
    /// The private store.
    pub store: &'a dyn PrivateStore,
    /// The visibility source.
    pub lookup: &'a dyn VisibilityLookup,
    /// The signing stack.
    pub signer: &'a dyn Signer,
    /// The public log client.
    pub client: &'a dyn PublicLogClient,
}

/// The public-log sink: leak check, then sign through the public path, then send
/// the trimmed public form and nothing else. Endpoint and Linear teams come from
/// the repository's selection.
pub struct PublicLogSink<'a> {
    selection: &'a SinkSelection,
    publishing_repo: &'a str,
    ports: Ports<'a>,
}

impl<'a> PublicLogSink<'a> {
    /// A public-log sink for the publishing repository.
    #[must_use]
    pub const fn new(
        selection: &'a SinkSelection,
        publishing_repo: &'a str,
        ports: Ports<'a>,
    ) -> Self {
        Self {
            selection,
            publishing_repo,
            ports,
        }
    }
}

impl RecordSink for PublicLogSink<'_> {
    fn publish(&self, record: &Record) -> SinkResult {
        let kind = SinkKind::PublicLog;
        let refuse = |code| SinkResult::refused(kind, record.id(), code);
        let Some(endpoint) = self.selection.public_log_endpoint.as_deref() else {
            return refuse(ErrorCode::SinkConfigIncomplete);
        };
        let public = record.public();
        let proof = match leak_check(
            self.publishing_repo,
            public,
            &self.selection.public_linear_teams,
            self.ports.lookup,
        ) {
            Ok(proof) => proof,
            Err(CheckRefusal::Offending(offending)) => {
                return SinkResult {
                    offending: Some(offending),
                    ..refuse(ErrorCode::PublicSinkPrivateReference)
                };
            }
            Err(CheckRefusal::Unencodable) => return refuse(ErrorCode::SigningFailed),
        };
        let entry = match attach_signature(
            self.ports.signer,
            public,
            proof.signed_bytes(),
            self.ports.signer.sign_for_public_log(&proof),
        ) {
            Ok(record) => CheckedEntry { record },
            Err(code) => return refuse(code),
        };
        match self.ports.client.send(endpoint, &entry) {
            Ok(()) => SinkResult::published(kind, record.id()),
            Err(LogRejected) => refuse(ErrorCode::PublicLogRejected),
        }
    }
}

impl PublicLogSink<'_> {
    #[cfg(feature = "attestation-verify")]
    fn publish_with_identity(
        &self,
        record: &Record,
        identity: Option<(
            &dyn BundleVerifier,
            &WorkflowIdentityPolicy,
            &SourceWorkflowPolicy,
        )>,
    ) -> SinkResult {
        let kind = SinkKind::PublicLog;
        let refuse = |code| SinkResult::refused(kind, record.id(), code);
        let Some(endpoint) = self.selection.public_log_endpoint.as_deref() else {
            return refuse(ErrorCode::SinkConfigIncomplete);
        };
        let public = record.public();
        let proof = match leak_check(
            self.publishing_repo,
            public,
            &self.selection.public_linear_teams,
            self.ports.lookup,
        ) {
            Ok(proof) => proof,
            Err(CheckRefusal::Offending(offending)) => {
                return SinkResult {
                    offending: Some(offending),
                    ..refuse(ErrorCode::PublicSinkPrivateReference)
                };
            }
            Err(CheckRefusal::Unencodable) => return refuse(ErrorCode::SigningFailed),
        };
        let Ok(bundle) = self.ports.signer.sign_for_public_log(&proof) else {
            return refuse(ErrorCode::SigningFailed);
        };
        if let Some((verifier, policy, source_policy)) = identity {
            let Ok(raw_bundle) = self.ports.signer.verification_bundle(&bundle) else {
                return refuse(ErrorCode::SigningFailed);
            };
            if !verified_receipt_identity(
                verifier,
                proof.signed_bytes(),
                &raw_bundle,
                policy,
                source_policy,
            ) {
                return refuse(ErrorCode::SigningFailed);
            }
        }
        let entry =
            match attach_signature(self.ports.signer, public, proof.signed_bytes(), Ok(bundle)) {
                Ok(record) => CheckedEntry { record },
                Err(code) => return refuse(code),
            };
        match self.ports.client.send(endpoint, &entry) {
            Ok(()) => SinkResult::published(kind, record.id()),
            Err(LogRejected) => refuse(ErrorCode::PublicLogRejected),
        }
    }
}

/// The private-store sink: signs without a public log and stores the public
/// form linked to the private form the sink layer already wrote, at the
/// selection's destination.
pub struct PrivateStoreSink<'a> {
    selection: &'a SinkSelection,
    ports: Ports<'a>,
}

impl<'a> PrivateStoreSink<'a> {
    /// A private-store sink.
    #[must_use]
    pub const fn new(selection: &'a SinkSelection, ports: Ports<'a>) -> Self {
        Self { selection, ports }
    }
}

impl RecordSink for PrivateStoreSink<'_> {
    fn publish(&self, record: &Record) -> SinkResult {
        let kind = SinkKind::PrivateStore;
        let refuse = |code| SinkResult::refused(kind, record.id(), code);
        let Some(destination) = self.selection.private_store_destination.as_deref() else {
            return refuse(ErrorCode::SinkConfigIncomplete);
        };
        let public = record.public();
        let Ok(canonical) = public.signed_bytes() else {
            return refuse(ErrorCode::SigningFailed);
        };
        match attach_signature(
            self.ports.signer,
            public,
            &canonical,
            self.ports.signer.sign_record_without_public_log(&canonical),
        ) {
            Err(code) => refuse(code),
            Ok(signed) => match self.ports.store.write_public(destination, &signed) {
                Ok(()) => SinkResult::published(kind, record.id()),
                Err(StoreError) => refuse(ErrorCode::PrivateStorePublicFormFailed),
            },
        }
    }
}

impl PrivateStoreSink<'_> {
    #[cfg(feature = "attestation-verify")]
    fn publish_with_identity(
        &self,
        record: &Record,
        identity: Option<(
            &dyn BundleVerifier,
            &WorkflowIdentityPolicy,
            &SourceWorkflowPolicy,
        )>,
    ) -> SinkResult {
        let kind = SinkKind::PrivateStore;
        let refuse = |code| SinkResult::refused(kind, record.id(), code);
        let Some(destination) = self.selection.private_store_destination.as_deref() else {
            return refuse(ErrorCode::SinkConfigIncomplete);
        };
        let public = record.public();
        let Ok(canonical) = public.signed_bytes() else {
            return refuse(ErrorCode::SigningFailed);
        };
        let Ok(bundle) = self.ports.signer.sign_record_without_public_log(&canonical) else {
            return refuse(ErrorCode::SigningFailed);
        };
        if let Some((verifier, policy, source_policy)) = identity {
            let Ok(raw_bundle) = self.ports.signer.verification_bundle(&bundle) else {
                return refuse(ErrorCode::SigningFailed);
            };
            if !verified_receipt_identity(verifier, &canonical, &raw_bundle, policy, source_policy)
            {
                return refuse(ErrorCode::SigningFailed);
            }
        }
        match attach_signature(self.ports.signer, public, &canonical, Ok(bundle)) {
            Err(code) => refuse(code),
            Ok(signed) => match self.ports.store.write_public(destination, &signed) {
                Ok(()) => SinkResult::published(kind, record.id()),
                Err(StoreError) => refuse(ErrorCode::PrivateStorePublicFormFailed),
            },
        }
    }
}

#[cfg(feature = "attestation-verify")]
fn verified_receipt_identity(
    verifier: &dyn BundleVerifier,
    payload: &[u8],
    bundle_json: &str,
    signer_policy: &WorkflowIdentityPolicy,
    source_policy: &SourceWorkflowPolicy,
) -> bool {
    verifier
        .verify_with_claims(payload, bundle_json, signer_policy)
        .ok()
        .flatten()
        .is_some_and(|claims| source_policy.matches(&claims))
}

/// The `none` sink: sends and stores nothing.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoneSink;

impl RecordSink for NoneSink {
    fn publish(&self, record: &Record) -> SinkResult {
        SinkResult::discarded(SinkKind::None, record.id())
    }
}

/// The sink layer: selects exactly one implementation per publish and writes
/// the private form first. Both built-in sinks are built from the one
/// selection. The public-log sink is not replaceable, so the leak check cannot
/// be bypassed; the private-store implementation can be replaced through
/// [`SinkLayer::register_private_store`].
pub struct SinkLayer<'a> {
    selection: &'a SinkSelection,
    publishing_repository: &'a str,
    store: &'a dyn PrivateStore,
    public_log: PublicLogSink<'a>,
    private_store: PrivateStoreSink<'a>,
    registered_private_store: Option<&'a dyn RecordSink>,
}

impl<'a> SinkLayer<'a> {
    /// A layer for the publishing repository's resolved selection.
    #[must_use]
    pub const fn new(
        selection: &'a SinkSelection,
        publishing_repo: &'a str,
        ports: Ports<'a>,
    ) -> Self {
        Self {
            selection,
            publishing_repository: publishing_repo,
            store: ports.store,
            public_log: PublicLogSink::new(selection, publishing_repo, ports),
            private_store: PrivateStoreSink::new(selection, ports),
            registered_private_store: None,
        }
    }

    /// Replaces the private-store implementation with another one.
    pub const fn register_private_store(&mut self, sink: &'a dyn RecordSink) {
        self.registered_private_store = Some(sink);
    }

    /// The sink selected by protected repository configuration.
    #[must_use]
    pub const fn sink_kind(&self) -> SinkKind {
        self.selection.sink
    }

    /// Publishes a record and signs its associated canonical statements using
    /// the same resolved sink. The private form is retained before any
    /// statement or public-record signing occurs.
    ///
    /// For `public_log`, every statement digest is placed in the associated
    /// public record and checked by FR-057 before the statement reaches the
    /// public Sigstore instance. `none` refuses because a receipt requires
    /// retention.
    ///
    /// # Errors
    ///
    /// Returns a refused [`SinkResult`] if retention, statement signing, or
    /// publication fails. No statement bundles are returned on refusal.
    #[cfg(feature = "receipt-record")]
    pub fn publish_receipt(
        &self,
        record: &Record,
        canonical_statements: &[Vec<u8>],
        verifier: &dyn BundleVerifier,
        signer_policy: &WorkflowIdentityPolicy,
        source_policy: &SourceWorkflowPolicy,
    ) -> Result<Vec<String>, SinkResult> {
        let kind = self.selection.sink;
        let refuse = |code| SinkResult::refused(kind, record.id(), code);
        let Some(destination) = self.selection.private_store_destination.as_deref() else {
            return Err(refuse(ErrorCode::SinkConfigIncomplete));
        };
        if canonical_statements.is_empty() || kind == SinkKind::None {
            return Err(refuse(ErrorCode::PrivateStoreWriteFailed));
        }
        if kind == SinkKind::PublicLog && self.selection.public_log_endpoint.is_none() {
            return Err(refuse(ErrorCode::SinkConfigIncomplete));
        }
        if !valid_repository(destination)
            || destination.eq_ignore_ascii_case(self.publishing_repository)
        {
            return Err(refuse(ErrorCode::SinkConfigInvalid));
        }
        match self
            .store
            .prepare_record(destination, record.private(), record.public(), kind)
        {
            Ok(StorePreparation::NewRecord) => {}
            Ok(StorePreparation::AlreadyPublished | StorePreparation::OutcomeIncomplete) => {
                return Err(refuse(ErrorCode::PrivateStoreOutcomeIncomplete));
            }
            Ok(StorePreparation::AlreadyRefused) => {
                return Err(refuse(ErrorCode::PrivateStoreRecordPreviouslyRefused));
            }
            Err(StoreError) => return Err(refuse(ErrorCode::PrivateStoreWriteFailed)),
        }

        let bundles = self.sign_receipt_statements(
            record,
            destination,
            canonical_statements,
            verifier,
            signer_policy,
            source_policy,
        )?;
        let result =
            self.publish_prepared_with_identity(record, verifier, signer_policy, source_policy);
        if result.status != Status::Published {
            return Err(result);
        }
        Ok(bundles)
    }

    #[cfg(feature = "receipt-record")]
    fn sign_receipt_statements(
        &self,
        record: &Record,
        destination: &str,
        statements: &[Vec<u8>],
        verifier: &dyn BundleVerifier,
        signer_policy: &WorkflowIdentityPolicy,
        source_policy: &SourceWorkflowPolicy,
    ) -> Result<Vec<String>, SinkResult> {
        let kind = self.selection.sink;
        let refuse = |code| SinkResult::refused(kind, record.id(), code);
        let mut bundles = Vec::with_capacity(statements.len());
        for statement in statements {
            let result = self.sign_receipt_statement(record, statement);
            let bundle = match result {
                Ok(bundle) => bundle,
                Err(SinkResult {
                    offending: Some(offending),
                    ..
                }) => {
                    self.refuse_receipt_publication(destination, record, kind);
                    return Err(SinkResult {
                        offending: Some(offending),
                        ..refuse(ErrorCode::PublicSinkPrivateReference)
                    });
                }
                Err(result) => {
                    self.refuse_receipt_publication(destination, record, kind);
                    return Err(result);
                }
            };
            if !self
                .public_log
                .ports
                .signer
                .signed_payload(&bundle)
                .is_ok_and(|payload| payload == *statement)
            {
                self.refuse_receipt_publication(destination, record, kind);
                return Err(refuse(ErrorCode::SignaturePayloadMismatch));
            }
            if !verified_receipt_identity(
                verifier,
                statement,
                &bundle,
                signer_policy,
                source_policy,
            ) {
                self.refuse_receipt_publication(destination, record, kind);
                return Err(refuse(ErrorCode::SigningFailed));
            }
            bundles.push(bundle);
        }
        Ok(bundles)
    }

    #[cfg(feature = "receipt-record")]
    fn refuse_receipt_publication(&self, destination: &str, record: &Record, kind: SinkKind) {
        let _ = self.store.write_publish_outcome(
            destination,
            record.id(),
            kind,
            ApprovalStatementPublishOutcome::Refused,
        );
    }

    #[cfg(feature = "receipt-record")]
    fn sign_receipt_statement(
        &self,
        record: &Record,
        statement: &[u8],
    ) -> Result<String, SinkResult> {
        let kind = self.selection.sink;
        let refuse = |code| SinkResult::refused(kind, record.id(), code);
        if kind != SinkKind::PublicLog {
            return self
                .public_log
                .ports
                .signer
                .sign_without_public_log(statement)
                .map_err(|_| refuse(ErrorCode::SigningFailed));
        }

        let mut public = record.public().clone();
        public.hashes.statement = Some(
            Sha256Digest::try_from(format!(
                "sha256:{}",
                sigstore_verify::crypto::sha256(statement).to_hex()
            ))
            .map_err(|_| refuse(ErrorCode::SigningFailed))?,
        );
        let proof = super::leak::leak_check_statement(
            self.publishing_repository,
            &public,
            statement,
            &self.selection.public_linear_teams,
            self.public_log.ports.lookup,
        )
        .map_err(|refusal| match refusal {
            super::leak::CheckRefusal::Offending(offending) => SinkResult {
                offending: Some(offending),
                ..refuse(ErrorCode::PublicSinkPrivateReference)
            },
            super::leak::CheckRefusal::Unencodable => refuse(ErrorCode::SigningFailed),
        })?;
        self.public_log
            .ports
            .signer
            .sign_statement_for_public_log(&proof)
            .map_err(|_| refuse(ErrorCode::SigningFailed))
    }

    fn publish_prepared(&self, record: &Record) -> SinkResult {
        let kind = self.selection.sink;
        let sink: &dyn RecordSink = match kind {
            SinkKind::None => &NoneSink,
            SinkKind::PublicLog => &self.public_log,
            SinkKind::PrivateStore => self.registered_private_store.unwrap_or(&self.private_store),
        };
        let result = sink.publish(record);
        self.write_outcome(record, kind, result)
    }

    #[cfg(feature = "receipt-record")]
    fn publish_prepared_with_identity(
        &self,
        record: &Record,
        verifier: &dyn BundleVerifier,
        signer_policy: &WorkflowIdentityPolicy,
        source_policy: &SourceWorkflowPolicy,
    ) -> SinkResult {
        let kind = self.selection.sink;
        let result = match kind {
            SinkKind::None => NoneSink.publish(record),
            SinkKind::PublicLog => self
                .public_log
                .publish_with_identity(record, Some((verifier, signer_policy, source_policy))),
            SinkKind::PrivateStore => match self.registered_private_store {
                Some(_) => SinkResult::refused(kind, record.id(), ErrorCode::SigningFailed),
                None => self
                    .private_store
                    .publish_with_identity(record, Some((verifier, signer_policy, source_policy))),
            },
        };
        self.write_outcome(record, kind, result)
    }

    fn write_outcome(&self, record: &Record, kind: SinkKind, result: SinkResult) -> SinkResult {
        if kind != SinkKind::None {
            let Some(destination) = self.selection.private_store_destination.as_deref() else {
                return SinkResult::refused(kind, record.id(), ErrorCode::SinkConfigIncomplete);
            };
            let outcome = if result.status == Status::Published {
                ApprovalStatementPublishOutcome::Published
            } else {
                ApprovalStatementPublishOutcome::Refused
            };
            if self
                .store
                .write_publish_outcome(destination, record.id(), kind, outcome)
                .is_err()
            {
                return SinkResult::refused(
                    kind,
                    record.id(),
                    ErrorCode::PrivateStoreOutcomeWriteFailed,
                );
            }
        }
        result
    }
}

impl RecordSink for SinkLayer<'_> {
    fn publish(&self, record: &Record) -> SinkResult {
        let kind = self.selection.sink;
        if kind == SinkKind::PublicLog && self.selection.public_log_endpoint.is_none() {
            return SinkResult::refused(kind, record.id(), ErrorCode::SinkConfigIncomplete);
        }
        if kind != SinkKind::None {
            let Some(destination) = self.selection.private_store_destination.as_deref() else {
                return SinkResult::refused(kind, record.id(), ErrorCode::SinkConfigIncomplete);
            };
            if !valid_repository(destination)
                || destination.eq_ignore_ascii_case(self.publishing_repository)
            {
                return SinkResult::refused(kind, record.id(), ErrorCode::SinkConfigInvalid);
            }
            match self
                .store
                .prepare_record(destination, record.private(), record.public(), kind)
            {
                Ok(StorePreparation::NewRecord) => {}
                Ok(StorePreparation::AlreadyPublished) => {
                    return SinkResult::published(kind, record.id());
                }
                Ok(StorePreparation::AlreadyRefused) => {
                    return SinkResult::refused(
                        kind,
                        record.id(),
                        ErrorCode::PrivateStoreRecordPreviouslyRefused,
                    );
                }
                Ok(StorePreparation::OutcomeIncomplete) => {
                    return SinkResult::refused(
                        kind,
                        record.id(),
                        ErrorCode::PrivateStoreOutcomeIncomplete,
                    );
                }
                Err(StoreError) => {
                    return SinkResult::refused(
                        kind,
                        record.id(),
                        ErrorCode::PrivateStoreWriteFailed,
                    );
                }
            }
        }
        self.publish_prepared(record)
    }
}
