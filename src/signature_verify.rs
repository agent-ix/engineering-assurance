// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Classification of git's verdict on a signed commit or tag (FR-036).
//!
//! The module is pure and performs no cryptography: git decides whether a
//! signature is valid, and `ssh-keygen` decides, for a tag git rejected, whether
//! the signature is intact. The classifier maps their exit statuses and fixed
//! signals to a [`Verdict`] and never reads message text. The binary's git
//! adapter runs the programs and builds the observation.

use serde::{Deserialize, Serialize};

/// Longest accepted revision in bytes.
pub const MAX_REVISION_BYTES: usize = 256;

const SSH_BEGIN: &[u8] = b"-----BEGIN SSH SIGNATURE-----";
const OTHER_BEGIN: [&[u8]; 3] = [
    b"-----BEGIN PGP SIGNATURE-----",
    b"-----BEGIN PGP MESSAGE-----",
    b"-----BEGIN SIGNED MESSAGE-----",
];

/// What the request asks to verify.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    /// A commit object.
    Commit,
    /// An annotated tag object.
    Tag,
}

/// Outcome class of a verification.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Git accepted the signature for a key the allowed-signers file names.
    SignedValid,
    /// A good signature by a key the allowed-signers file does not accept.
    SignedUntrusted,
    /// A signature that does not verify.
    SignedBad,
    /// The object carries no signature.
    Unsigned,
    /// No classification is possible.
    Unverifiable,
}

/// Why a verdict is `unsigned` (lightweight tag) or `unverifiable`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// The tag is a ref directly to a commit and carries no signature itself.
    LightweightTag,
    /// Git could not be started.
    GitNotStarted,
    /// The directory is not a usable repository.
    RepositoryUnavailable,
    /// The revision does not resolve to an object of the requested kind.
    RevisionUnresolved,
    /// A process exceeded its wall-clock limit.
    Timeout,
    /// A process wrote more than the capture bound.
    OutputExceeded,
    /// Git could not check the signature (grade `E`).
    SignatureUnchecked,
    /// `ssh-keygen` was required and could not be started.
    SshKeygenNotStarted,
    /// The signature is not an SSH signature.
    UnsupportedSignatureFormat,
    /// The output matches no known outcome.
    UnrecognizedOutput,
}

/// The result of verifying one object.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Verdict {
    /// Outcome class.
    pub status: Status,
    /// What was verified.
    pub kind: TargetKind,
    /// Signing key fingerprint exactly as git reports it; a commit only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_fingerprint: Option<String>,
    /// Present only for a lightweight tag and for `unverifiable`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
}

impl Verdict {
    /// An `unverifiable` verdict with the reason it could not be classified.
    #[must_use]
    pub const fn unverifiable(kind: TargetKind, reason: Reason) -> Self {
        Self {
            status: Status::Unverifiable,
            kind,
            key_fingerprint: None,
            reason: Some(reason),
        }
    }

    const fn plain(kind: TargetKind, status: Status) -> Self {
        Self {
            status,
            kind,
            key_fingerprint: None,
            reason: None,
        }
    }
}

/// Format of the signature block an object carries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignatureFormat {
    /// An SSH signature.
    Ssh,
    /// Any other format (`OpenPGP`, X.509, gitsign).
    Other,
}

/// What git printed for a commit it was asked to grade.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitGrade {
    /// Whether `git verify-commit` exited with status 0.
    pub verify_accepted: bool,
    /// The `%G?` grade code git printed.
    pub grade: char,
    /// The `%GF` code git printed, when it printed one.
    pub fingerprint: Option<String>,
}

/// What was observed for a commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitObservation {
    /// Format of the commit's signature header; `None` when it has none.
    pub format: Option<SignatureFormat>,
    /// Git's grade, absent when git was not asked to grade the commit.
    pub git: Option<GitGrade>,
}

/// Outcome of the delegated `ssh-keygen -Y check-novalidate` run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SshCheck {
    /// Exit status 0: the signature is intact.
    Accepted,
    /// Nonzero exit status: the signature does not verify.
    Rejected,
    /// `ssh-keygen` could not be started.
    NotStarted,
}

/// What was observed for an annotated tag, or the lack of one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TagObservation {
    /// The revision names a commit directly.
    Lightweight,
    /// The tag object carries no signature block of any format.
    NoSignature,
    /// The tag object carries a signature block.
    Signature {
        /// Format of the block.
        format: SignatureFormat,
        /// Whether `git verify-tag` exited with status 0.
        verify_accepted: bool,
        /// The delegated check, which only a tag git rejected is given.
        check: Option<SshCheck>,
    },
}

/// Classifies what was observed for a commit.
///
/// The grade code is the commit mechanism only. `G` counts as valid only when
/// `git verify-commit` agreed.
#[must_use]
pub fn classify_commit(observation: &CommitObservation) -> Verdict {
    let kind = TargetKind::Commit;
    if observation.format == Some(SignatureFormat::Other) {
        return Verdict::unverifiable(kind, Reason::UnsupportedSignatureFormat);
    }
    let Some(git) = &observation.git else {
        return Verdict::unverifiable(kind, Reason::UnrecognizedOutput);
    };
    let signed = |status| Verdict {
        status,
        kind,
        key_fingerprint: git.fingerprint.clone(),
        reason: None,
    };
    match git.grade {
        'G' if git.verify_accepted => signed(Status::SignedValid),
        'U' => signed(Status::SignedUntrusted),
        'B' => signed(Status::SignedBad),
        'N' => Verdict::plain(kind, Status::Unsigned),
        'E' => Verdict::unverifiable(kind, Reason::SignatureUnchecked),
        _ => Verdict::unverifiable(kind, Reason::UnrecognizedOutput),
    }
}

/// Classifies what was observed for a tag, in the order the tag is graded:
/// no signature block, a non-SSH block, `git verify-tag`, then the delegated
/// signature check.
#[must_use]
pub fn classify_tag(observation: &TagObservation) -> Verdict {
    let kind = TargetKind::Tag;
    match observation {
        TagObservation::Lightweight => Verdict {
            reason: Some(Reason::LightweightTag),
            ..Verdict::plain(kind, Status::Unsigned)
        },
        TagObservation::NoSignature => Verdict::plain(kind, Status::Unsigned),
        TagObservation::Signature {
            format: SignatureFormat::Other,
            ..
        } => Verdict::unverifiable(kind, Reason::UnsupportedSignatureFormat),
        TagObservation::Signature {
            verify_accepted: true,
            ..
        } => Verdict::plain(kind, Status::SignedValid),
        TagObservation::Signature { check, .. } => match check {
            Some(SshCheck::Accepted) => Verdict::plain(kind, Status::SignedUntrusted),
            Some(SshCheck::Rejected) => Verdict::plain(kind, Status::SignedBad),
            Some(SshCheck::NotStarted) => Verdict::unverifiable(kind, Reason::SshKeygenNotStarted),
            None => Verdict::unverifiable(kind, Reason::UnrecognizedOutput),
        },
    }
}

/// A tag object split at its signature block header line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitTag<'a> {
    /// The tag object with the signature block removed.
    pub payload: &'a [u8],
    /// The signature block, starting at its header line.
    pub signature: &'a [u8],
    /// Format named by the header line.
    pub format: SignatureFormat,
}

/// Splits a tag object at its signature block header line.
///
/// Git reads the last line that starts with a header of any format as the start
/// of the signature, so earlier message text that looks like a header stays in
/// the payload.
/// Returns `None` for an object with no signature block of any format.
#[must_use]
pub fn split_tag_object(object: &[u8]) -> Option<SplitTag<'_>> {
    let mut found = None;
    let mut start = 0;
    while start < object.len() {
        let rest = &object[start..];
        if let Some(format) = header_format(rest) {
            found = Some((start, format));
        }
        start += rest
            .iter()
            .position(|b| *b == b'\n')
            .map_or(rest.len(), |n| n + 1);
    }
    found.map(|(start, format)| SplitTag {
        payload: &object[..start],
        signature: &object[start..],
        format,
    })
}

/// Format of the signature headers of a commit object.
///
/// Git verifies one of the `gpgsig` headers depending on the repository's hash
/// algorithm, so the commit counts as SSH-signed only when every signature
/// header is an SSH signature.
#[must_use]
pub fn commit_signature_format(object: &[u8]) -> Option<SignatureFormat> {
    let mut found = None;
    for line in object.split(|b| *b == b'\n') {
        if line.is_empty() {
            break;
        }
        let value = line
            .strip_prefix(b"gpgsig ")
            .or_else(|| line.strip_prefix(b"gpgsig-sha256 "));
        if let Some(value) = value {
            match header_format(value).unwrap_or(SignatureFormat::Other) {
                SignatureFormat::Other => return Some(SignatureFormat::Other),
                SignatureFormat::Ssh => found = Some(SignatureFormat::Ssh),
            }
        }
    }
    found
}

/// Format named by a line that starts with a signature header, as git's own
/// signature split reads it: the header is a prefix, not the whole line.
fn header_format(line: &[u8]) -> Option<SignatureFormat> {
    let is_header = |header: &[u8]| line.starts_with(header);
    if is_header(SSH_BEGIN) {
        Some(SignatureFormat::Ssh)
    } else if OTHER_BEGIN.iter().any(|header| is_header(header)) {
        Some(SignatureFormat::Other)
    } else {
        None
    }
}

/// Whether `revision` is a plain revision name: not empty, at most
/// [`MAX_REVISION_BYTES`] bytes, not option-like, and free of whitespace and
/// control characters.
#[must_use]
pub fn is_plain_revision(revision: &str) -> bool {
    !revision.is_empty()
        && revision.len() <= MAX_REVISION_BYTES
        && !revision.starts_with('-')
        && !revision
            .chars()
            .any(|c| c.is_control() || c.is_whitespace())
}
