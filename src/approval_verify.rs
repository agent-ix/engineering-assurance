// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Decision of whether a signed annotated tag approves one change-assurance
//! record (FR-041).
//!
//! The module is pure. It takes the `signature-verify` verdict for the tag
//! (FR-036, FR-037, FR-040), which owns every signature rule, and the values
//! the tag message's trailer block holds for the two record trailers, which the
//! `signature-verify` trailer reader extracts. It compares those values to the
//! request and decides the status. It reads no signature, certificate or
//! trailer text of its own.

use serde::Serialize;

use crate::signature_verify::{Presence, Reason, SignatureKind, SignedAt, Status, Verdict};

/// Trailer naming the record the tag approves.
pub const RECORD_ID_TRAILER: &str = "Approved-Record-Id";
/// Trailer carrying the digest of the record the tag approves.
pub const RECORD_DIGEST_TRAILER: &str = "Approved-Record-Digest";

/// Longest accepted `record_id` or `record_digest` in bytes.
pub const MAX_RECORD_VALUE_BYTES: usize = 256;

/// Whether `value` is an acceptable `record_id` or `record_digest`: not empty,
/// at most [`MAX_RECORD_VALUE_BYTES`] bytes, no control character, and not
/// option-like.
#[must_use]
pub fn is_record_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_RECORD_VALUE_BYTES
        && !value.starts_with('-')
        && !value.chars().any(char::is_control)
}

/// How one record trailer relates to the request.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Binding {
    /// Exactly one trailer, byte-equal to the request.
    Match,
    /// Exactly one trailer, different from the request.
    Mismatch,
    /// No such trailer.
    Missing,
    /// More than one such trailer, whatever their values.
    Repeated,
}

/// Compares the values of one trailer to the requested value, byte for byte.
#[must_use]
pub fn bind(values: &[String], requested: &str) -> Binding {
    match values {
        [] => Binding::Missing,
        [only] if only == requested => Binding::Match,
        [_] => Binding::Mismatch,
        _ => Binding::Repeated,
    }
}

/// The bindings of both record trailers.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct Bindings {
    /// The `Approved-Record-Id` binding.
    pub record_id: Binding,
    /// The `Approved-Record-Digest` binding.
    pub record_digest: Binding,
}

impl Bindings {
    const fn both_match(self) -> bool {
        matches!(
            (self.record_id, self.record_digest),
            (Binding::Match, Binding::Match)
        )
    }
}

/// The approval status.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    /// A mapped signer approved exactly the requested record, with a meaning.
    Approved,
    /// A valid, mapped signature whose tag does not bind the requested record.
    WrongRecord,
    /// A good signature by a key or identity the trust input does not accept.
    UntrustedSigner,
    /// A valid, mapped, bound tag with no `Signed-off-meaning` trailer.
    MeaningMissing,
    /// A signature that does not verify.
    SignedBad,
    /// No signature, or not an annotated tag object.
    Unsigned,
    /// No decision is possible.
    Unverifiable,
}

/// The reason for an `unverifiable` approval.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum UnverifiableReason {
    /// The reason the `signature-verify` verdict carries.
    Signature(Reason),
    /// A validly signed tag whose message could not be read.
    Trailer(TrailerUnreadable),
}

/// The single value of the reason `trailer_unreadable`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrailerUnreadable {
    /// The tag message could not be read.
    TrailerUnreadable,
}

/// Where the signing time comes from.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignedAtSource {
    /// The tag's tagger line: inside the signed payload, and the signer's claim.
    TaggerDate,
}

/// Why an `approved` result has no signing time.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignedAtMissing {
    /// The tag has no readable, well-formed tagger line.
    TaggerDateUnreadable,
}

/// The signing time of an approval, or the explicit statement that none is
/// obtainable. It is never defaulted to the verification time.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SigningTime {
    /// A UTC instant and where it comes from.
    Known {
        /// The signing time, `YYYY-MM-DDTHH:MM:SSZ`.
        signed_at: SignedAt,
        /// Where the time comes from.
        signed_at_source: SignedAtSource,
    },
    /// No time is obtainable.
    Missing {
        /// Why there is no time.
        signed_at_missing: SignedAtMissing,
    },
}

impl SigningTime {
    fn from_tagger(tagger_signed_at: Option<SignedAt>) -> Self {
        match tagger_signed_at {
            Some(signed_at) => Self::Known {
                signed_at,
                signed_at_source: SignedAtSource::TaggerDate,
            },
            None => Self::Missing {
                signed_at_missing: SignedAtMissing::TaggerDateUnreadable,
            },
        }
    }
}

/// The members only an `approved` result carries.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Approval {
    /// The principal the trust input names for the signer.
    pub approver: String,
    /// The `Signed-off-meaning` values in order.
    pub meaning: Vec<String>,
    /// Which verifier checked the signature.
    pub signature_kind: SignatureKind,
    /// When the approval was signed, or why that is not known.
    #[serde(flatten)]
    pub signing_time: SigningTime,
}

/// What the status decision yields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decision {
    /// The approval status.
    pub status: ApprovalStatus,
    /// Present only for `unverifiable`.
    pub reason: Option<UnverifiableReason>,
    /// Present only for `approved`.
    pub approval: Option<Approval>,
}

impl Decision {
    const fn status(status: ApprovalStatus) -> Self {
        Self {
            status,
            reason: None,
            approval: None,
        }
    }

    const fn unverifiable(reason: UnverifiableReason) -> Self {
        Self {
            status: ApprovalStatus::Unverifiable,
            reason: Some(reason),
            approval: None,
        }
    }
}

/// Decides the status from the signature verdict and the bindings, in order:
/// the signature status, then the record binding, then the meaning. `bindings`
/// is `None` when the tag message could not be read. `tagger_signed_at` is the
/// UTC instant the tag's tagger line claims, or `None` when it has none; it
/// only fills the signing time of an `approved` result and never changes a
/// status.
#[must_use]
pub fn decide(
    verdict: &Verdict,
    bindings: Option<Bindings>,
    tagger_signed_at: Option<SignedAt>,
) -> Decision {
    match verdict.status {
        Status::Unsigned => Decision::status(ApprovalStatus::Unsigned),
        Status::SignedBad => Decision::status(ApprovalStatus::SignedBad),
        Status::Unverifiable => verdict.reason.map_or_else(
            || Decision::status(ApprovalStatus::Unverifiable),
            |reason| Decision::unverifiable(UnverifiableReason::Signature(reason)),
        ),
        Status::SignedUntrusted => Decision::status(ApprovalStatus::UntrustedSigner),
        Status::SignedValid => decide_valid(verdict, bindings, tagger_signed_at),
    }
}

fn decide_valid(
    verdict: &Verdict,
    bindings: Option<Bindings>,
    tagger_signed_at: Option<SignedAt>,
) -> Decision {
    let Some(bindings) = bindings else {
        return Decision::unverifiable(UnverifiableReason::Trailer(
            TrailerUnreadable::TrailerUnreadable,
        ));
    };
    if !bindings.both_match() {
        return Decision::status(ApprovalStatus::WrongRecord);
    }
    let Some(meaning) = verdict
        .signed_off_meaning
        .as_ref()
        .filter(|meaning| meaning.presence == Presence::Present)
    else {
        return Decision::status(ApprovalStatus::MeaningMissing);
    };
    let approver = verdict
        .signer
        .as_ref()
        .and_then(|signer| signer.principal.clone());
    match (approver, verdict.signature_kind) {
        (Some(approver), Some(signature_kind)) => Decision {
            approval: Some(Approval {
                approver,
                meaning: meaning.values.clone(),
                signature_kind,
                signing_time: SigningTime::from_tagger(tagger_signed_at),
            }),
            ..Decision::status(ApprovalStatus::Approved)
        },
        // A valid verdict always names a mapped principal and its verifier.
        _ => Decision::unverifiable(UnverifiableReason::Signature(Reason::UnrecognizedOutput)),
    }
}
