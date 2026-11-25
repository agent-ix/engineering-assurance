// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Adapter for the `approval-verify` command (FR-041).
//!
//! The request is checked, then the tag is verified by the `signature-verify`
//! adapter, which runs git, `ssh-keygen` and `gitsign` and reads the tag
//! message's trailer block. This adapter reads the two record trailers from
//! that block with the `signature-verify` trailer reader and hands the verdict
//! and the bindings to the pure decision. It starts no process of its own.

use std::path::{Path, PathBuf};

use engineering_assurance::approval_verify::{
    Approval, ApprovalStatus, Bindings, GESTURE_TRAILER, GestureDeclaration, RECORD_DIGEST_TRAILER,
    RECORD_ID_TRAILER, TrustedAtMissing, TrustedAtSource, TrustedTime, UnverifiableReason, bind,
    decide, is_record_value,
};
use engineering_assurance::signature_verify::{
    Reason, SignatureKind, SignedAt, Status, Verdict, is_plain_revision, trailer_values,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::producer_identity::ProducerIdentity;
use crate::signature_verify_host::{
    AllowedSignersSnapshot, AllowedSignersSource, CaptureFailure, CheckedTag, GitsignInput,
    GitsignRequest, LoadedGitsign, RegistryFailure, capture_allowed_signers, check_tag,
    git_version, load_gitsign, validate_gitsign,
};

const REQUEST_PROTOCOL: &str = "engineering-assurance.approval-verify-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.approval-verify-result/v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    repository: PathBuf,
    tag: String,
    record_id: String,
    record_digest: String,
    allowed_signers: String,
    gitsign: Option<GitsignRequest>,
    identity_registry: Option<String>,
}

/// The trust input a verdict was made against: the request's members, the
/// identity of the bytes and the programs the run used, and the commit the
/// identity registry revision resolved to (FR-041).
#[derive(Clone, Copy, Serialize)]
pub(crate) struct Trust<'a> {
    pub(crate) allowed_signers: &'a str,
    pub(crate) allowed_signers_sha256: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) allowed_signers_commit: Option<&'a str>,
    pub(crate) ea_version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) git_version: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) gitsign: Option<&'a GitsignRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) identity_registry: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) identity_registry_commit: Option<&'a str>,
}

/// The `gitsign` trust of a request as the `trust` member echoes it.
#[derive(Clone, Copy)]
pub(crate) struct GitsignEcho<'a> {
    pub(crate) gitsign: Option<&'a GitsignRequest>,
    pub(crate) identity_registry: Option<&'a str>,
}

impl<'a> GitsignEcho<'a> {
    /// The `gitsign` and `identity_registry` members of a request.
    pub(crate) const fn new(
        gitsign: Option<&'a GitsignRequest>,
        identity_registry: Option<&'a str>,
    ) -> Self {
        Self {
            gitsign,
            identity_registry,
        }
    }
}

/// What a run read of the allowed-signers source, and the git it ran.
#[derive(Clone, Copy)]
pub(crate) struct TrustReading<'a> {
    /// The request's `allowed_signers` value, as requested.
    pub(crate) requested: &'a str,
    /// The bytes the run used.
    pub(crate) snapshot: &'a AllowedSignersSnapshot,
    /// What `git --version` printed.
    pub(crate) git_version: Option<&'a str>,
}

impl<'a> Trust<'a> {
    /// The trust of a run that read `reading` and `loaded`, or no registry
    /// commit when the revision did not resolve or no registry was named.
    pub(crate) fn new(
        reading: TrustReading<'a>,
        echo: GitsignEcho<'a>,
        loaded: Option<&'a LoadedGitsign>,
    ) -> Self {
        Self {
            allowed_signers: reading.requested,
            allowed_signers_sha256: reading.snapshot.sha256(),
            allowed_signers_commit: reading.snapshot.commit(),
            ea_version: env!("CARGO_PKG_VERSION"),
            git_version: reading.git_version,
            gitsign: echo.gitsign,
            identity_registry: echo.identity_registry,
            identity_registry_commit: loaded.map(LoadedGitsign::commit),
        }
    }
}

/// The FR-041 result for one tag, less the members that echo the request.
#[derive(Serialize)]
pub(crate) struct Assessment {
    status: ApprovalStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<UnverifiableReason>,
    signature: Verdict,
    #[serde(flatten)]
    approval: Option<Approval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    binding: Option<Bindings>,
    /// The declared gesture of an `approved` result (FR-086).
    #[serde(skip_serializing_if = "Option::is_none")]
    gesture: Option<GestureDeclaration>,
}

impl Assessment {
    pub(crate) const fn status(&self) -> ApprovalStatus {
        self.status
    }

    pub(crate) const fn reason(&self) -> Option<UnverifiableReason> {
        self.reason
    }

    pub(crate) const fn approval(&self) -> Option<&Approval> {
        self.approval.as_ref()
    }
}

/// Decides the FR-041 result for a tag already verified by `check_tag`.
pub(crate) fn assess(checked: &CheckedTag, record_id: &str, record_digest: &str) -> Assessment {
    let bindings = checked.trailers.as_deref().map(|trailers| Bindings {
        record_id: bind(&trailer_values(trailers, RECORD_ID_TRAILER), record_id),
        record_digest: bind(
            &trailer_values(trailers, RECORD_DIGEST_TRAILER),
            record_digest,
        ),
    });
    let decision = decide(&checked.verdict, bindings, checked.tagger_signed_at.clone());
    let gesture = (decision.status == ApprovalStatus::Approved)
        .then(|| {
            checked.trailers.as_deref().map(|trailers| {
                GestureDeclaration::from_trailer_values(&trailer_values(trailers, GESTURE_TRAILER))
            })
        })
        .flatten();
    Assessment {
        status: decision.status,
        reason: decision.reason,
        signature: checked.verdict.clone(),
        approval: decision.approval,
        binding: bindings,
        gesture,
    }
}

/// The FR-041 result for one tag.
#[derive(Serialize)]
pub(crate) struct ResultLine<'a> {
    protocol: &'static str,
    tag: &'a str,
    record_id: &'a str,
    record_digest: &'a str,
    #[serde(flatten)]
    producer: ProducerIdentity,
    /// Absent only when git failed while the allowed-signers `<rev>:<path>`
    /// source was read, so no digest, commit or versions were established.
    #[serde(skip_serializing_if = "Option::is_none")]
    trust: Option<Trust<'a>>,
    tag_object_id: Option<String>,
    #[serde(flatten)]
    assessment: Assessment,
}

impl<'a> ResultLine<'a> {
    /// Verifies `tag` of `repository` against the record and the trust input.
    pub(crate) fn verify(
        repository: &Path,
        tag: &'a str,
        record: (&'a str, &'a str),
        reading: TrustReading<'a>,
        echo: GitsignEcho<'a>,
        loaded: Option<&'a LoadedGitsign>,
        producer: ProducerIdentity,
    ) -> Self {
        let checked = check_tag(repository, tag, reading.snapshot, loaded);
        Self::from_checked(
            checked,
            tag,
            record,
            Some(Trust::new(reading, echo, loaded)),
            producer,
        )
    }

    /// The result for a tag that was checked, or never read.
    pub(crate) fn from_checked(
        checked: CheckedTag,
        tag: &'a str,
        record: (&'a str, &'a str),
        trust: Option<Trust<'a>>,
        producer: ProducerIdentity,
    ) -> Self {
        Self::from_checked_with_trusted_root(
            checked,
            tag,
            record,
            trust,
            sigstore_verify::trust_root::SIGSTORE_PRODUCTION_TRUSTED_ROOT,
            producer,
        )
    }

    /// Result builder with a caller-supplied trust root for deterministic
    /// verification tests.
    pub(crate) fn from_checked_with_trusted_root(
        checked: CheckedTag,
        tag: &'a str,
        record: (&'a str, &'a str),
        trust: Option<Trust<'a>>,
        trusted_root_json: &str,
        producer: ProducerIdentity,
    ) -> Self {
        let (record_id, record_digest) = record;
        let mut assessment = assess(&checked, record_id, record_digest);
        if checked.verdict.status == Status::SignedValid
            && assessment.status == ApprovalStatus::Approved
            && assessment
                .approval()
                .is_some_and(|approval| approval.signature_kind == SignatureKind::Gitsign)
        {
            let trusted_time = match checked.tag_object.as_deref().map(crate::approval_rekor::extract) {
                None | Some(crate::approval_rekor::EmbeddedEntry::Invalid) => {
                    TrustedTime::Missing {
                        trusted_at_missing: TrustedAtMissing::LogEntryUnverified,
                    }
                }
                Some(crate::approval_rekor::EmbeddedEntry::Missing) => TrustedTime::Missing {
                    trusted_at_missing: TrustedAtMissing::NoLogEntry,
                },
                Some(crate::approval_rekor::EmbeddedEntry::Bundle { artifact, bundle }) => {
                    match crate::approval_rekor::verify(&bundle, &artifact, trusted_root_json) {
                        Some(timestamp) => trusted_time(timestamp, assessment.approval().and_then(|approval| {
                            match &approval.signing_time {
                                engineering_assurance::approval_verify::SigningTime::Known { signed_at, .. } => Some(signed_at),
                                engineering_assurance::approval_verify::SigningTime::Missing { .. } => None,
                            }
                        })),
                        None => TrustedTime::Missing {
                            trusted_at_missing: TrustedAtMissing::LogEntryUnverified,
                        },
                    }
                }
            };
            if let Some(approval) = assessment.approval.as_mut() {
                approval.trusted_time = trusted_time;
            }
        }
        Self {
            protocol: RESULT_PROTOCOL,
            tag: tag.strip_prefix("refs/tags/").unwrap_or(tag),
            record_id,
            record_digest,
            producer,
            trust,
            tag_object_id: checked.tag_object_id.clone(),
            assessment,
        }
    }

    /// The decision part of the result.
    pub(crate) const fn assessment(&self) -> &Assessment {
        &self.assessment
    }
}

fn trusted_time(trusted_seconds: i64, signed_at: Option<&SignedAt>) -> TrustedTime {
    let Ok(format) = time::format_description::parse_borrowed::<2>(
        "[year]-[month]-[day]T[hour]:[minute]:[second]Z",
    ) else {
        return TrustedTime::Missing {
            trusted_at_missing: TrustedAtMissing::LogEntryUnverified,
        };
    };
    let Ok(utc) = time::OffsetDateTime::from_unix_timestamp(trusted_seconds) else {
        return TrustedTime::Missing {
            trusted_at_missing: TrustedAtMissing::LogEntryUnverified,
        };
    };
    let Ok(formatted) = utc.format(&format) else {
        return TrustedTime::Missing {
            trusted_at_missing: TrustedAtMissing::LogEntryUnverified,
        };
    };
    let skew = signed_at.and_then(|signed_at| {
        let parsed = time::OffsetDateTime::parse(
            signed_at.as_str(),
            &time::format_description::well_known::Rfc3339,
        )
        .ok()?;
        let difference = parsed.unix_timestamp().checked_sub(trusted_seconds)?;
        (difference != 0).then_some(difference)
    });
    TrustedTime::Known {
        trusted_at: formatted,
        trusted_at_source: TrustedAtSource::TransparencyLog,
        time_skew: skew,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::trusted_time;
    use engineering_assurance::signature_verify::{SignedAt, tagger_signed_at};
    use ix_trace_rs::trace;

    fn signed_at(epoch: &str) -> SignedAt {
        let object = format!(
            "object 0000000000000000000000000000000000000000\ntype commit\ntag verified\ntagger Alice <alice@example.test> {epoch} +0000\n\nmessage\n"
        );
        tagger_signed_at(object.as_bytes()).expect("fixture tagger date is valid")
    }

    #[trace("FR-041-AC-28")]
    #[test]
    fn trusted_time_is_utc_and_time_skew_is_signed_minus_integrated_time() {
        let later = trusted_time(1_700_000_000, Some(&signed_at("1700000010")));
        let later = serde_json::to_value(later).expect("trusted time serializes");
        assert_eq!(later["trusted_at"], "2023-11-14T22:13:20Z");
        assert_eq!(later["trusted_at_source"], "transparency_log");
        assert_eq!(later["time_skew"], 10);

        let equal = trusted_time(1_700_000_000, Some(&signed_at("1700000000")));
        let equal = serde_json::to_value(equal).expect("trusted time serializes");
        assert_eq!(equal["trusted_at"], "2023-11-14T22:13:20Z");
        assert!(equal.get("time_skew").is_none());

        let earlier = trusted_time(1_700_000_000, Some(&signed_at("1699999990")));
        let earlier = serde_json::to_value(earlier).expect("trusted time serializes");
        assert_eq!(earlier["time_skew"], -10);
    }
}

/// Typed refusal that produces no verdict.
#[derive(Debug, Error)]
pub(crate) enum ApprovalVerifyError {
    /// The request is not readable JSON.
    #[error("approval-verify request is not readable JSON")]
    Unreadable,
    /// The request is JSON that is not an exact approval-verify request.
    #[error("approval-verify request is not an exact approval-verify request")]
    Invalid,
    /// The allowed-signers file is not a readable regular UTF-8 file of at most 1 MiB.
    #[error(
        "approval-verify allowed-signers file is not a readable UTF-8 regular file of at most 1 MiB"
    )]
    AllowedSigners,
    /// The `gitsign` member names a person, which only the identity registry may do.
    #[error("approval-verify gitsign member must not carry a principal")]
    GitsignPrincipal,
    /// The identity registry source is invalid or its content is unusable.
    #[error("approval-verify identity registry is not a usable committed registry")]
    IdentityRegistry,
    /// The result cannot be encoded.
    #[error("approval-verify result cannot be encoded")]
    ResultEncoding,
}

impl ApprovalVerifyError {
    /// Stable machine error code.
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Unreadable => "approval_verify_request_unreadable",
            Self::Invalid => "approval_verify_request_invalid",
            Self::AllowedSigners => "approval_verify_allowed_signers_invalid",
            Self::GitsignPrincipal => "approval_verify_gitsign_principal_refused",
            Self::IdentityRegistry => "approval_verify_identity_registry_invalid",
            Self::ResultEncoding => "approval_verify_result_encoding_failed",
        }
    }
}

/// What reading the identity registry of a request yielded.
enum Registry {
    /// The registry was read, or the request names none.
    Loaded(Option<LoadedGitsign>),
    /// Git could not answer while the registry was read.
    GitFailed(Reason),
}

fn load_registry(
    repository: &Path,
    input: Option<GitsignInput>,
) -> Result<Registry, ApprovalVerifyError> {
    match input
        .map(|input| load_gitsign(repository, input))
        .transpose()
    {
        Ok(loaded) => Ok(Registry::Loaded(loaded)),
        Err(RegistryFailure::Invalid) => Err(ApprovalVerifyError::IdentityRegistry),
        Err(RegistryFailure::Git(reason)) => Ok(Registry::GitFailed(reason)),
    }
}

/// Runs one request document and returns the result line.
pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, ApprovalVerifyError> {
    let producer = ProducerIdentity::sample().ok_or(ApprovalVerifyError::ResultEncoding)?;
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| ApprovalVerifyError::Unreadable)?;
    let request: Request =
        serde_json::from_value(value).map_err(|_| ApprovalVerifyError::Invalid)?;
    if request.protocol != REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !is_plain_revision(&request.tag)
        || !is_record_value(&request.record_id)
        || !is_record_value(&request.record_digest)
    {
        return Err(ApprovalVerifyError::Invalid);
    }
    let source = AllowedSignersSource::parse(&request.allowed_signers)
        .ok_or(ApprovalVerifyError::Invalid)?;
    let input = validate_gitsign(
        request.gitsign.as_ref(),
        request.identity_registry.as_deref(),
    )
    .map_err(|error| {
        error.refusal(
            ApprovalVerifyError::Invalid,
            ApprovalVerifyError::GitsignPrincipal,
        )
    })?;
    let record = (request.record_id.as_str(), request.record_digest.as_str());
    let allowed_signers = match capture_allowed_signers(&request.repository, &source) {
        Ok(snapshot) => snapshot,
        Err(CaptureFailure::Invalid) => return Err(ApprovalVerifyError::AllowedSigners),
        // Git could not answer while the source was read: no verifier starts,
        // and no trust identity was established.
        Err(CaptureFailure::Git(reason)) => {
            let result = ResultLine::from_checked(
                CheckedTag::unread(reason),
                &request.tag,
                record,
                None,
                producer,
            );
            return encode(&result);
        }
    };
    let registry = load_registry(&request.repository, input)?;
    let git_version = git_version(&request.repository);
    let reading = TrustReading {
        requested: &request.allowed_signers,
        snapshot: &allowed_signers,
        git_version: git_version.as_deref(),
    };
    let echo = GitsignEcho::new(
        request.gitsign.as_ref(),
        request.identity_registry.as_deref(),
    );
    let result = match &registry {
        Registry::Loaded(loaded) => ResultLine::verify(
            &request.repository,
            &request.tag,
            record,
            reading,
            echo,
            loaded.as_ref(),
            producer.clone(),
        ),
        // Git could not answer while the registry was read: no verifier starts.
        Registry::GitFailed(reason) => ResultLine::from_checked(
            CheckedTag::unread(*reason),
            &request.tag,
            record,
            Some(Trust::new(reading, echo, None)),
            producer,
        ),
    };
    encode(&result)
}

fn encode(result: &ResultLine<'_>) -> Result<Vec<u8>, ApprovalVerifyError> {
    let mut line = serde_json::to_vec(result).map_err(|_| ApprovalVerifyError::ResultEncoding)?;
    line.push(b'\n');
    Ok(line)
}
