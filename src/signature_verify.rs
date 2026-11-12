// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Classification of git's verdict on a signed commit or tag (FR-036).
//!
//! The module is pure and performs no cryptography: git decides whether a
//! signature is valid, and `ssh-keygen` decides, for a tag git rejected, whether
//! the signature is intact. The classifier maps their exit statuses and fixed
//! signals to a [`Verdict`]. The `Signed-off-meaning:` trailer (FR-037) is read
//! from the output of `git interpret-trailers --parse`, never from raw message
//! text. The binary's git adapter runs the programs and builds the
//! observation.

use serde::{Deserialize, Serialize};

/// Longest accepted revision in bytes.
pub const MAX_REVISION_BYTES: usize = 256;

const MEANING_TOKEN: &str = "Signed-off-meaning";
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

/// Whether the allowed-signers file maps the signing key to a person.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mapping {
    /// Git accepted the key for a principal named in the file.
    Mapped,
    /// Git did not accept the key for any principal in the file.
    Unmapped,
}

/// The person a signature is attributed to (FR-037).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Signer {
    /// Whether the key is mapped.
    pub mapping: Mapping,
    /// The principal named in the allowed-signers file; only when mapped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principal: Option<String>,
}

impl Signer {
    const fn unmapped() -> Self {
        Self {
            mapping: Mapping::Unmapped,
            principal: None,
        }
    }

    const fn mapped(principal: String) -> Self {
        Self {
            mapping: Mapping::Mapped,
            principal: Some(principal),
        }
    }
}

/// Whether a message carries a `Signed-off-meaning:` trailer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Presence {
    /// At least one trailer is in the message's trailer block.
    Present,
    /// No such trailer is in the message's trailer block.
    Missing,
}

/// The meaning a signer recorded in `Signed-off-meaning:` trailers (FR-037),
/// as recorded text.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SignedOffMeaning {
    /// Whether the trailer is present.
    pub presence: Presence,
    /// Every trailer value in order; empty when the trailer is missing.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
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
    /// The mapped person; present only for `signed_valid` and `signed_untrusted`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signer: Option<Signer>,
    /// The recorded meaning, when the object's message was readable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signed_off_meaning: Option<SignedOffMeaning>,
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
            signer: None,
            signed_off_meaning: None,
        }
    }

    /// The verdict with the meaning read from the object's message.
    #[must_use]
    pub fn with_meaning(mut self, meaning: Option<SignedOffMeaning>) -> Self {
        self.signed_off_meaning = meaning;
        self
    }

    const fn plain(kind: TargetKind, status: Status) -> Self {
        Self {
            status,
            kind,
            key_fingerprint: None,
            reason: None,
            signer: None,
            signed_off_meaning: None,
        }
    }

    fn signed(kind: TargetKind, status: Status, signer: Signer) -> Self {
        Self {
            signer: Some(signer),
            ..Self::plain(kind, status)
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
    /// The `%GS` code git printed, the principal it matched, when it printed one.
    pub principal: Option<String>,
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

/// Outcome of the delegated `ssh-keygen -Y find-principals` run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrincipalLookup {
    /// The lookup was not run.
    NotRun,
    /// Exit status 0: the first line printed.
    Found(String),
    /// Nonzero exit status, or no usable line.
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
        /// The principal lookup, which only a tag git accepted is given.
        principal: PrincipalLookup,
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
    let signed = |status, signer| Verdict {
        key_fingerprint: git.fingerprint.clone(),
        signer,
        ..Verdict::plain(kind, status)
    };
    match git.grade {
        'G' if git.verify_accepted => match git.principal.clone() {
            Some(principal) => signed(Status::SignedValid, Some(Signer::mapped(principal))),
            None => Verdict::unverifiable(kind, Reason::UnrecognizedOutput),
        },
        'U' => signed(Status::SignedUntrusted, Some(Signer::unmapped())),
        'B' => signed(Status::SignedBad, None),
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
            principal,
            ..
        } => match principal {
            PrincipalLookup::Found(name) => {
                Verdict::signed(kind, Status::SignedValid, Signer::mapped(name.clone()))
            }
            PrincipalLookup::NotStarted => Verdict::unverifiable(kind, Reason::SshKeygenNotStarted),
            PrincipalLookup::NotRun | PrincipalLookup::Rejected => {
                Verdict::unverifiable(kind, Reason::UnrecognizedOutput)
            }
        },
        TagObservation::Signature { check, .. } => match check {
            Some(SshCheck::Accepted) => {
                Verdict::signed(kind, Status::SignedUntrusted, Signer::unmapped())
            }
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

/// The message of a commit or tag object: everything after its header block.
///
/// Git ends the headers at the first blank line, and a signature header
/// continues on lines that start with a space, so the first `\n\n` is the end.
#[must_use]
pub fn object_message(object: &[u8]) -> &[u8] {
    object
        .windows(2)
        .position(|pair| pair == b"\n\n")
        .map_or(&[], |at| &object[at + 2..])
}

/// Reads the `Signed-off-meaning:` trailers from the output of
/// `git interpret-trailers --parse`, which prints one `token: value` line per
/// trailer of the message's trailer block.
#[must_use]
pub fn signed_off_meaning(parsed_trailers: &str) -> SignedOffMeaning {
    let values: Vec<String> = parsed_trailers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .filter(|(token, _)| token.trim().eq_ignore_ascii_case(MEANING_TOKEN))
        .map(|(_, value)| value.trim().to_owned())
        .collect();
    SignedOffMeaning {
        presence: if values.is_empty() {
            Presence::Missing
        } else {
            Presence::Present
        },
        values,
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
