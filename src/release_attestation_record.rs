// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Release-attestation record construction and sink routing (FR-071).

use ix_trace_rs::implements;
use serde_json::Value;

use crate::record_sink::{
    config::{ErrorCode, SinkKind, Visibility},
    form::{
        FormError, GitObjectId, Hashes, PrivateRecord, PublicProtocol, PublicRecord, Record,
        RecordId, RecordKind, References, Sha256Digest, Subject, Timestamp, Timestamps, Verdict,
        Verdicts,
    },
    sink::{RecordSink, SinkLayer, SinkResult, Status},
};

/// Whether release verification admitted a release-attestation record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseOutcome {
    /// `release-verify` reported `released`.
    Released,
    /// `release-verify` reported any other result.
    NotReleased,
}

/// Build the public and private forms for one verified release.
///
/// # Errors
///
/// Returns [`FormError`] when a supplied digest, commit, timestamp, or
/// repository does not match its record-form grammar.
#[implements("FR-071-AC-1")]
pub fn build_release_attestation_record(
    repository: &str,
    record_id: RecordId,
    summary_digest: &str,
    release_commit: &str,
    created_at: &str,
    references: References,
    private_summary: Value,
) -> Result<Record, FormError> {
    let mut private = PrivateRecord::new(record_id.clone());
    private.statement = Some(private_summary);
    let public = PublicRecord {
        protocol: PublicProtocol::V1,
        record_id,
        kind: RecordKind::ReleaseAttestation,
        subject: Subject::repo(repository)?,
        references,
        hashes: Hashes {
            private_form: None,
            statement: Some(Sha256Digest::try_from(summary_digest)?),
            subject_commit: Some(GitObjectId::try_from(release_commit)?),
        },
        verdicts: Verdicts {
            approval: None,
            change_record: None,
            release: Some(Verdict::Released),
        },
        timestamps: Timestamps {
            created: Some(Timestamp::try_from(created_at)?),
            signed: None,
            published: None,
        },
        signature: None,
    };
    Record::new(public, private)
}

/// Route one release record through the resolved sink after release
/// verification. `None` means that the release was not admitted and no sink
/// was called.
#[implements(
    "FR-071-AC-2",
    "FR-071-AC-3",
    "FR-071-AC-4",
    "FR-071-AC-5",
    "FR-071-AC-6"
)]
#[must_use]
pub fn publish_release_attestation(
    sink: &SinkLayer<'_>,
    visibility: Visibility,
    outcome: ReleaseOutcome,
    record: &Record,
) -> Option<SinkResult> {
    if outcome == ReleaseOutcome::NotReleased {
        return None;
    }

    let kind = sink.sink_kind();
    if record.public().kind != crate::record_sink::form::RecordKind::ReleaseAttestation {
        return Some(SinkResult::refused(
            kind,
            record.id(),
            ErrorCode::SinkConfigInvalid,
        ));
    }
    if kind == SinkKind::None {
        return Some(SinkResult::refused(
            kind,
            record.id(),
            ErrorCode::ReleaseAttestationPrivateStoreRequired,
        ));
    }
    if visibility == Visibility::Public && kind != SinkKind::PublicLog {
        return Some(SinkResult::refused(
            kind,
            record.id(),
            ErrorCode::ReleaseAttestationPublicLogRequired,
        ));
    }
    if visibility != Visibility::Public && kind != SinkKind::PrivateStore {
        return Some(SinkResult::refused(
            kind,
            record.id(),
            ErrorCode::ReleaseAttestationPrivateSigningUnavailable,
        ));
    }

    let result = sink.publish(record);
    if visibility != Visibility::Public
        && result.status == Status::Refused
        && result.error == Some(ErrorCode::SigningFailed)
    {
        Some(SinkResult::refused(
            kind,
            record.id(),
            ErrorCode::ReleaseAttestationPrivateSigningUnavailable,
        ))
    } else {
        Some(result)
    }
}
