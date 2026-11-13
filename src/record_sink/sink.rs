// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The sink interface, its three implementations and the sink layer (FR-054).
//!
//! The sink layer writes the full private form first, then hands the record to
//! exactly one sink chosen from the repository's selection. A refusal is never
//! rerouted to another sink.

use serde::{Deserialize, Serialize};

use super::config::{ErrorCode, SinkKind, SinkSelection};
use super::form::{PrivateRecord, PublicRecord, Record, RecordId, Signature};
use super::leak::{Offending, PublicPathProof, VisibilityLookup, leak_check};

/// The protocol literal of a publish result.
pub const RESULT_PROTOCOL: &str = "engineering-assurance.sink-result/v1";

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
#[serde(deny_unknown_fields)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorCode>,
    /// Each item the leak check refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offending: Option<Vec<Offending>>,
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

/// The private store: a destination named in the sink configuration.
pub trait PrivateStore {
    /// Stores the full private form under `destination`.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the write fails.
    fn write_private(&self, destination: &str, private: &PrivateRecord) -> Result<(), StoreError>;

    /// Stores the trimmed public form linked to a private form.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the write fails.
    fn write_public(&self, destination: &str, public: &PublicRecord) -> Result<(), StoreError>;
}

/// The public log rejected the entry or could not be reached.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[error("public log rejected the record")]
pub struct LogRejected;

/// The append-only public transparency log.
pub trait PublicLogClient {
    /// Sends one entry, the signed public form, to the log at `endpoint`.
    ///
    /// # Errors
    ///
    /// [`LogRejected`] when the log does not accept the entry.
    fn send(&self, endpoint: &str, entry: &PublicRecord) -> Result<(), LogRejected>;
}

/// The delegated signing stack failed.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[error("signing failed")]
pub struct SigningFailed;

/// The delegated Sigstore signing and verification stack (NFR-010). This crate
/// implements no signature primitive; it only chooses the path and checks what
/// the bundle signed.
pub trait Signer {
    /// Signs `canonical` and uploads to the public transparency log. It takes a
    /// [`PublicPathProof`], so it is reachable only after the leak check passed.
    ///
    /// # Errors
    ///
    /// [`SigningFailed`] when the stack fails.
    fn sign_for_public_log(
        &self,
        proof: &PublicPathProof<'_>,
        canonical: &[u8],
    ) -> Result<String, SigningFailed>;

    /// Signs `canonical` with a configuration that writes to no public log.
    ///
    /// # Errors
    ///
    /// [`SigningFailed`] when the stack fails.
    fn sign_without_public_log(&self, canonical: &[u8]) -> Result<String, SigningFailed>;

    /// The payload a base64 bundle signs.
    ///
    /// # Errors
    ///
    /// [`SigningFailed`] when the bundle cannot be decoded.
    fn signed_payload(&self, bundle: &str) -> Result<Vec<u8>, SigningFailed>;
}

/// Attaches `bundle` to a copy of `public` after checking it signs exactly the
/// canonical bytes of the public form.
fn attach_signature(
    signer: &dyn Signer,
    public: &PublicRecord,
    canonical: &[u8],
    bundle: Result<String, SigningFailed>,
) -> Result<PublicRecord, ErrorCode> {
    let bundle = bundle.map_err(|_| ErrorCode::SigningFailed)?;
    let signature = Signature::try_from(bundle).map_err(|_| ErrorCode::SignatureInvalid)?;
    let payload = signer
        .signed_payload(signature.as_str())
        .map_err(|_| ErrorCode::SigningFailed)?;
    if payload != canonical {
        return Err(ErrorCode::SignaturePayloadMismatch);
    }
    let mut attached = public.clone();
    attached.signature = Some(signature);
    Ok(attached)
}

fn canonical(public: &PublicRecord) -> Result<Vec<u8>, ErrorCode> {
    public.signed_bytes().map_err(|_| ErrorCode::SigningFailed)
}

/// The public-log sink: leak check, then sign through the public path, then send
/// the trimmed public form and nothing else.
pub struct PublicLogSink<'a> {
    /// The log endpoint.
    pub endpoint: &'a str,
    /// The repository that publishes.
    pub publishing_repo: &'a str,
    /// Linear team keys whose issues count as public.
    pub public_linear_teams: &'a [String],
    /// The visibility source.
    pub lookup: &'a dyn VisibilityLookup,
    /// The signing stack.
    pub signer: &'a dyn Signer,
    /// The log client.
    pub client: &'a dyn PublicLogClient,
}

impl RecordSink for PublicLogSink<'_> {
    fn publish(&self, record: &Record) -> SinkResult {
        let kind = SinkKind::PublicLog;
        let public = record.public();
        let refuse = |code| SinkResult::refused(kind, record.id(), code);
        let proof = match leak_check(
            self.publishing_repo,
            public,
            self.public_linear_teams,
            self.lookup,
        ) {
            Ok(proof) => proof,
            Err(offending) => {
                return SinkResult {
                    offending: Some(offending),
                    ..refuse(ErrorCode::PublicSinkPrivateReference)
                };
            }
        };
        let signed = canonical(public).and_then(|bytes| {
            let bundle = self.signer.sign_for_public_log(&proof, &bytes);
            attach_signature(self.signer, public, &bytes, bundle)
        });
        match signed {
            Err(code) => refuse(code),
            Ok(signed) => match self.client.send(self.endpoint, &signed) {
                Ok(()) => SinkResult::published(kind, record.id()),
                Err(LogRejected) => refuse(ErrorCode::PublicLogRejected),
            },
        }
    }
}

/// The private-store sink: signs without a public log and stores the public
/// form linked to the private form the sink layer already wrote.
pub struct PrivateStoreSink<'a> {
    /// The store destination.
    pub destination: &'a str,
    /// The store.
    pub store: &'a dyn PrivateStore,
    /// The signing stack.
    pub signer: &'a dyn Signer,
}

impl RecordSink for PrivateStoreSink<'_> {
    fn publish(&self, record: &Record) -> SinkResult {
        let kind = SinkKind::PrivateStore;
        let public = record.public();
        let signed = canonical(public).and_then(|bytes| {
            let bundle = self.signer.sign_without_public_log(&bytes);
            attach_signature(self.signer, public, &bytes, bundle)
        });
        match signed {
            Err(code) => SinkResult::refused(kind, record.id(), code),
            Ok(signed) => match self.store.write_public(self.destination, &signed) {
                Ok(()) => SinkResult::published(kind, record.id()),
                Err(StoreError) => {
                    SinkResult::refused(kind, record.id(), ErrorCode::PrivateStorePublicFormFailed)
                }
            },
        }
    }
}

/// The `none` sink: sends and stores nothing.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoneSink;

impl RecordSink for NoneSink {
    fn publish(&self, record: &Record) -> SinkResult {
        SinkResult::discarded(SinkKind::None, record.id())
    }
}

/// The implementation registered for each sink kind.
pub struct Sinks<'a> {
    /// The public-log implementation.
    pub public_log: &'a dyn RecordSink,
    /// The private-store implementation.
    pub private_store: &'a dyn RecordSink,
    /// The `none` implementation.
    pub none: &'a dyn RecordSink,
}

/// The sink layer: selects exactly one implementation per publish and writes
/// the private form first.
pub struct SinkLayer<'a> {
    /// The repository's resolved selection.
    pub selection: &'a SinkSelection,
    /// The private store the full private form is written to.
    pub store: &'a dyn PrivateStore,
    /// The registered implementations.
    pub sinks: Sinks<'a>,
}

impl RecordSink for SinkLayer<'_> {
    fn publish(&self, record: &Record) -> SinkResult {
        let kind = self.selection.sink;
        let sink = match kind {
            SinkKind::None => self.sinks.none,
            SinkKind::PublicLog => self.sinks.public_log,
            SinkKind::PrivateStore => self.sinks.private_store,
        };
        if kind == SinkKind::PublicLog && self.selection.public_log_endpoint.is_none() {
            return SinkResult::refused(kind, record.id(), ErrorCode::SinkConfigIncomplete);
        }
        if kind != SinkKind::None {
            let Some(destination) = self.selection.private_store_destination.as_deref() else {
                return SinkResult::refused(kind, record.id(), ErrorCode::SinkConfigIncomplete);
            };
            if self
                .store
                .write_private(destination, record.private())
                .is_err()
            {
                return SinkResult::refused(kind, record.id(), ErrorCode::PrivateStoreWriteFailed);
            }
        }
        sink.publish(record)
    }
}
