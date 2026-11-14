// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Classification of git's verdict on a signed commit or tag (FR-036) and of
//! `gitsign`'s verdict on a CMS-signed one (FR-040).
//!
//! The module is pure and performs no cryptography: git decides whether an SSH
//! signature is valid, `ssh-keygen` decides, for a tag git rejected, whether
//! the signature is intact, and `gitsign` decides for a CMS signature. The
//! classifier maps their exit statuses and fixed signals to a [`Verdict`]. The `Signed-off-meaning:` trailer (FR-037) is read
//! from the output of `git interpret-trailers --parse`, never from raw message
//! text. The binary's git adapter runs the programs and builds the
//! observation.

use serde::{Deserialize, Serialize};

/// Longest accepted revision in bytes.
pub const MAX_REVISION_BYTES: usize = 256;

/// Name of the trailer that carries the approval meaning (FR-037).
pub const MEANING_TOKEN: &str = "Signed-off-meaning";
const SSH_BEGIN: &[u8] = b"-----BEGIN SSH SIGNATURE-----";
const CMS_BEGIN: &[u8] = b"-----BEGIN SIGNED MESSAGE-----";
const OTHER_BEGIN: [&[u8]; 2] = [
    b"-----BEGIN PGP SIGNATURE-----",
    b"-----BEGIN PGP MESSAGE-----",
];

/// Longest accepted `gitsign` request value in bytes.
pub const MAX_GITSIGN_VALUE_BYTES: usize = 256;

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
    /// A CMS-signed object was given no `gitsign` identity to verify against.
    GitsignIdentityMissing,
    /// `gitsign` could not be started.
    GitsignNotStarted,
    /// `gitsign` did not accept the signature, even with wildcard matchers, or
    /// exited with a status other than 0 or 1.
    GitsignVerificationFailed,
    /// The repository sets a `gitsign.*` configuration key.
    GitsignRepositoryConfig,
}

/// Which verifier a signature belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureKind {
    /// An SSH signature, verified by git and `ssh-keygen`.
    Ssh,
    /// A CMS (X.509) signature, verified by `gitsign`.
    Gitsign,
}

/// Where a signer's person comes from.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignerSource {
    /// The allowed-signers file (SSH).
    AllowedSigners,
    /// The request's `gitsign.principal`.
    Request,
}

/// How a `gitsign` identity or issuer differs from the request's.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mismatch {
    /// Only the certificate identity differs.
    Identity,
    /// Only the OIDC issuer differs.
    Issuer,
    /// Both differ.
    IdentityAndIssuer,
}

/// The certificate identity and issuer `gitsign` accepted.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Certificate {
    /// The certificate identity, copied from the request.
    pub identity: String,
    /// The OIDC issuer, copied from the request.
    pub oidc_issuer: String,
}

/// The `gitsign` member of a request: who the signature must come from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitsignIdentity {
    certificate_identity: String,
    certificate_oidc_issuer: String,
    principal: String,
}

impl GitsignIdentity {
    /// Builds an identity, or `None` when a value is empty, longer than
    /// [`MAX_GITSIGN_VALUE_BYTES`], holds a control character, or (identity and
    /// issuer only) begins with `-`.
    #[must_use]
    pub fn new(
        certificate_identity: String,
        certificate_oidc_issuer: String,
        principal: String,
    ) -> Option<Self> {
        let plain = |value: &str| {
            !value.is_empty()
                && value.len() <= MAX_GITSIGN_VALUE_BYTES
                && !value.chars().any(char::is_control)
        };
        let option_safe = |value: &str| plain(value) && !value.starts_with('-');
        (option_safe(&certificate_identity)
            && option_safe(&certificate_oidc_issuer)
            && plain(&principal))
        .then_some(Self {
            certificate_identity,
            certificate_oidc_issuer,
            principal,
        })
    }

    /// The certificate identity the signature must carry.
    #[must_use]
    pub fn certificate_identity(&self) -> &str {
        &self.certificate_identity
    }

    /// The OIDC issuer the certificate must name.
    #[must_use]
    pub fn certificate_oidc_issuer(&self) -> &str {
        &self.certificate_oidc_issuer
    }
}

/// One `gitsign` invocation of the exit-status procedure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Probe {
    /// The request's identity and issuer.
    Exact,
    /// Wildcard identity and wildcard issuer.
    AnyIdentityAnyIssuer,
    /// The request's identity and a wildcard issuer.
    ExactIdentityAnyIssuer,
    /// A wildcard identity and the request's issuer.
    AnyIdentityExactIssuer,
}

/// What was observed of one delegated run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Run {
    /// The program ran to an exit status.
    Exit(i32),
    /// The program could not be started.
    NotStarted,
    /// The program exceeded its wall-clock limit.
    TimedOut,
    /// The program wrote more than the capture bound.
    CutShort,
    /// The program was ended by a signal and has no exit status.
    Signaled,
    /// The run could not be observed for another reason.
    Unobserved,
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
    /// Where the person comes from.
    pub source: SignerSource,
    /// Whether the key is mapped.
    pub mapping: Mapping,
    /// The principal named in the allowed-signers file; only when mapped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principal: Option<String>,
}

impl Signer {
    const fn unmapped(source: SignerSource) -> Self {
        Self {
            source,
            mapping: Mapping::Unmapped,
            principal: None,
        }
    }

    const fn mapped(source: SignerSource, principal: String) -> Self {
        Self {
            source,
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
    /// The canonical commit object id whose signature was checked. Always
    /// present after a commit revision resolves; absent for tags and before
    /// commit resolution succeeds.
    pub verified_commit_id: Option<String>,
    /// Which verifier the identified signature block belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_kind: Option<SignatureKind>,
    /// Signing key fingerprint exactly as git reports it; a commit only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_fingerprint: Option<String>,
    /// Present only for a lightweight tag and for `unverifiable`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
    /// The mapped person; present only for `signed_valid` and `signed_untrusted`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signer: Option<Signer>,
    /// The identity and issuer `gitsign` accepted; `signed_valid` gitsign only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate: Option<Certificate>,
    /// How a gitsign signature differs; `signed_untrusted` gitsign only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mismatch: Option<Mismatch>,
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
            verified_commit_id: None,
            signature_kind: None,
            key_fingerprint: None,
            reason: Some(reason),
            signer: None,
            certificate: None,
            mismatch: None,
            signed_off_meaning: None,
        }
    }

    /// The verdict with the meaning read from the object's message.
    #[must_use]
    pub fn with_meaning(mut self, meaning: Option<SignedOffMeaning>) -> Self {
        self.signed_off_meaning = meaning;
        self
    }

    /// The canonical commit object id whose signature was checked.
    #[must_use]
    pub fn with_verified_commit_id(mut self, id: Option<String>) -> Self {
        self.verified_commit_id = id;
        self
    }

    /// The verdict with the verifier of the identified signature block.
    #[must_use]
    pub const fn with_signature_kind(mut self, kind: Option<SignatureKind>) -> Self {
        self.signature_kind = kind;
        self
    }

    const fn plain(kind: TargetKind, status: Status) -> Self {
        Self {
            status,
            kind,
            verified_commit_id: None,
            signature_kind: None,
            key_fingerprint: None,
            reason: None,
            signer: None,
            certificate: None,
            mismatch: None,
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
    /// A CMS (X.509) signature, which is what gitsign produces.
    Cms,
    /// Any other format (`OpenPGP`).
    Other,
}

impl SignatureFormat {
    /// The verifier of an identified SSH or CMS block.
    #[must_use]
    pub const fn signature_kind(self) -> Option<SignatureKind> {
        match self {
            Self::Ssh => Some(SignatureKind::Ssh),
            Self::Cms => Some(SignatureKind::Gitsign),
            Self::Other => None,
        }
    }
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
    let verdict = classify_ssh_commit(observation);
    if verdict.status == Status::Unsigned {
        return verdict;
    }
    let signature_kind = observation.format.and_then(SignatureFormat::signature_kind);
    verdict.with_signature_kind(signature_kind)
}

fn classify_ssh_commit(observation: &CommitObservation) -> Verdict {
    let kind = TargetKind::Commit;
    match observation.format {
        Some(SignatureFormat::Other) => {
            return Verdict::unverifiable(kind, Reason::UnsupportedSignatureFormat);
        }
        // A CMS object is classified by `classify_gitsign`, never by git's grade.
        Some(SignatureFormat::Cms) => {
            return Verdict::unverifiable(kind, Reason::UnrecognizedOutput);
        }
        Some(SignatureFormat::Ssh) | None => {}
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
            Some(principal) => signed(
                Status::SignedValid,
                Some(Signer::mapped(SignerSource::AllowedSigners, principal)),
            ),
            None => Verdict::unverifiable(kind, Reason::UnrecognizedOutput),
        },
        'U' => signed(
            Status::SignedUntrusted,
            Some(Signer::unmapped(SignerSource::AllowedSigners)),
        ),
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
    let verdict = classify_ssh_tag(observation);
    match observation {
        TagObservation::Signature { format, .. } => {
            verdict.with_signature_kind(format.signature_kind())
        }
        TagObservation::Lightweight | TagObservation::NoSignature => verdict,
    }
}

fn classify_ssh_tag(observation: &TagObservation) -> Verdict {
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
        // A CMS object is classified by `classify_gitsign`, never by git.
        TagObservation::Signature {
            format: SignatureFormat::Cms,
            ..
        } => Verdict::unverifiable(kind, Reason::UnrecognizedOutput),
        TagObservation::Signature {
            verify_accepted: true,
            principal,
            ..
        } => match principal {
            PrincipalLookup::Found(name) => Verdict::signed(
                kind,
                Status::SignedValid,
                Signer::mapped(SignerSource::AllowedSigners, name.clone()),
            ),
            PrincipalLookup::NotStarted => Verdict::unverifiable(kind, Reason::SshKeygenNotStarted),
            PrincipalLookup::NotRun | PrincipalLookup::Rejected => {
                Verdict::unverifiable(kind, Reason::UnrecognizedOutput)
            }
        },
        TagObservation::Signature { check, .. } => match check {
            Some(SshCheck::Accepted) => Verdict::signed(
                kind,
                Status::SignedUntrusted,
                Signer::unmapped(SignerSource::AllowedSigners),
            ),
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
/// algorithm, so the commit counts as SSH-signed (or CMS-signed) only when every
/// signature header is of that one format; a mix is [`SignatureFormat::Other`].
#[must_use]
pub fn commit_signature_format(object: &[u8]) -> Option<SignatureFormat> {
    let mut found: Option<SignatureFormat> = None;
    for line in object.split(|b| *b == b'\n') {
        if line.is_empty() {
            break;
        }
        let value = line
            .strip_prefix(b"gpgsig ")
            .or_else(|| line.strip_prefix(b"gpgsig-sha256 "));
        if let Some(value) = value {
            let format = header_format(value).unwrap_or(SignatureFormat::Other);
            if format == SignatureFormat::Other || found.is_some_and(|seen| seen != format) {
                return Some(SignatureFormat::Other);
            }
            found = Some(format);
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
    } else if is_header(CMS_BEGIN) {
        Some(SignatureFormat::Cms)
    } else if OTHER_BEGIN.iter().any(|header| is_header(header)) {
        Some(SignatureFormat::Other)
    } else {
        None
    }
}

/// Whether the output of `git config --get-regexp .*` holds a `gitsign.` key,
/// or a key that `gitsign` might read as one.
///
/// `gitsign` reads that output line by line and takes the text before the first
/// space of each line as a key, so a value that holds a newline can inject a
/// key. It compares the `gitsign.` prefix with Unicode simple case folding, so
/// `gitſign.` (long s) and a Kelvin sign fold to ASCII letters. This does not
/// implement that folding: it fails closed and counts a line whose key, up to
/// its first `.`, holds a non-ASCII byte, besides the ASCII case-insensitive
/// `gitsign.` prefix.
#[must_use]
pub fn config_sets_gitsign_key(output: &[u8]) -> bool {
    const PREFIX: &[u8] = b"gitsign.";
    output.split(|b| *b == b'\n').any(|line| {
        let key = line.split(|b| *b == b' ').next().unwrap_or_default();
        let section = key.split(|b| *b == b'.').next().unwrap_or_default();
        !section.is_ascii()
            || (key.len() >= PREFIX.len() && key[..PREFIX.len()].eq_ignore_ascii_case(PREFIX))
    })
}

/// Classifies a CMS-signed object by delegating to `gitsign` through `run`.
///
/// `run` performs one [`Probe`] and reports only what was observed; the
/// classifier asks for at most four, each only when the earlier exit statuses
/// require it, and reads nothing but exit statuses. Without an `identity` no
/// probe runs and the object is never valid. With `repository_sets_key`, the
/// repository could choose the roots that verify it, so no probe runs.
#[must_use]
pub fn classify_gitsign(
    kind: TargetKind,
    identity: Option<&GitsignIdentity>,
    repository_sets_key: bool,
    mut run: impl FnMut(Probe) -> Run,
) -> Verdict {
    let verdict = |reason| Verdict::unverifiable(kind, reason);
    let Some(identity) = identity else {
        return verdict(Reason::GitsignIdentityMissing)
            .with_signature_kind(Some(SignatureKind::Gitsign));
    };
    let outcome = if repository_sets_key {
        Err(Reason::GitsignRepositoryConfig)
    } else {
        probe_gitsign(kind, identity, &mut run)
    };
    outcome
        .unwrap_or_else(verdict)
        .with_signature_kind(Some(SignatureKind::Gitsign))
}

fn probe_gitsign(
    kind: TargetKind,
    identity: &GitsignIdentity,
    run: &mut impl FnMut(Probe) -> Run,
) -> Result<Verdict, Reason> {
    let mut accepted = |probe| match run(probe) {
        Run::Exit(0) => Ok(true),
        Run::Exit(1) => Ok(false),
        Run::Exit(_) | Run::Signaled => Err(Reason::GitsignVerificationFailed),
        Run::NotStarted => Err(Reason::GitsignNotStarted),
        Run::TimedOut => Err(Reason::Timeout),
        Run::CutShort => Err(Reason::OutputExceeded),
        Run::Unobserved => Err(Reason::UnrecognizedOutput),
    };
    if accepted(Probe::Exact)? {
        let signer = Signer::mapped(SignerSource::Request, identity.principal.clone());
        return Ok(Verdict {
            certificate: Some(Certificate {
                identity: identity.certificate_identity.clone(),
                oidc_issuer: identity.certificate_oidc_issuer.clone(),
            }),
            ..Verdict::signed(kind, Status::SignedValid, signer)
        });
    }
    if !accepted(Probe::AnyIdentityAnyIssuer)? {
        return Err(Reason::GitsignVerificationFailed);
    }
    let mismatch = if accepted(Probe::ExactIdentityAnyIssuer)? {
        Mismatch::Issuer
    } else if accepted(Probe::AnyIdentityExactIssuer)? {
        Mismatch::Identity
    } else {
        Mismatch::IdentityAndIssuer
    };
    Ok(Verdict {
        mismatch: Some(mismatch),
        ..Verdict::signed(
            kind,
            Status::SignedUntrusted,
            Signer::unmapped(SignerSource::Request),
        )
    })
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

/// Latest accepted tagger time in seconds since the epoch, 9999-12-31T23:59:59Z.
const MAX_TAGGER_EPOCH: u64 = 253_402_300_799;

/// A signing time in UTC (`YYYY-MM-DDTHH:MM:SSZ`) that [`tagger_signed_at`]
/// read from a tagger line. Only that parser builds one.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct SignedAt(String);

impl SignedAt {
    /// The instant as `YYYY-MM-DDTHH:MM:SSZ`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The time the tag's tagger line claims, as a UTC instant
/// (`YYYY-MM-DDTHH:MM:SSZ`), read from the header block of a tag object.
///
/// The tagger line is `tagger <name> <email> <epoch> <zone>`. The epoch counts
/// seconds since 1970-01-01T00:00:00Z whatever the zone, so the instant is the
/// epoch formatted in UTC and the zone only records the signer's local offset.
/// Only the header block is read (never message text), the object must have
/// exactly one `tagger` header, the epoch must be 1 to 12 ASCII digits no later
/// than year 9999, and the zone must be a sign and four digits with minutes
/// below 60. Anything else yields `None`, and no time is guessed.
///
/// The tagger line is part of the payload the signature covers, so a validly
/// signed tag cannot have it altered; it is still the signer's own claim.
#[must_use]
pub fn tagger_signed_at(tag_payload: &[u8]) -> Option<SignedAt> {
    let headers = tag_payload
        .windows(2)
        .position(|pair| pair == b"\n\n")
        .map_or(tag_payload, |at| &tag_payload[..at]);
    let mut taggers = headers
        .split(|b| *b == b'\n')
        .filter_map(|line| line.strip_prefix(b"tagger "));
    let line = taggers.next()?;
    if taggers.next().is_some() {
        return None;
    }
    let after_email = &line[line.iter().rposition(|b| *b == b'>')? + 1..];
    let text = std::str::from_utf8(after_email).ok()?;
    let mut fields = text.strip_prefix(' ')?.split(' ');
    let (epoch, zone) = (fields.next()?, fields.next()?);
    if fields.next().is_some() {
        return None;
    }
    let zone = zone.as_bytes();
    let zone_ok = zone.len() == 5
        && matches!(zone[0], b'+' | b'-')
        && zone[1..].iter().all(u8::is_ascii_digit)
        && zone[3] < b'6';
    let epoch_ok = (1..=12).contains(&epoch.len()) && epoch.bytes().all(|b| b.is_ascii_digit());
    if !zone_ok || !epoch_ok {
        return None;
    }
    let seconds: u64 = epoch.parse().ok().filter(|s| *s <= MAX_TAGGER_EPOCH)?;
    Some(SignedAt(format_utc(seconds)))
}

/// Formats seconds since the epoch as `YYYY-MM-DDTHH:MM:SSZ` (proleptic
/// Gregorian calendar, years 1970 to 9999).
fn format_utc(seconds: u64) -> String {
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    // Days since 1970-01-01 to a civil date, counting years from 1 March.
    let shifted = days + 719_468;
    let era = shifted / 146_097;
    let day_of_era = shifted % 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3_600,
        rest % 3_600 / 60,
        rest % 60
    )
}

/// Reads the `Signed-off-meaning:` trailers from the output of
/// `git interpret-trailers --parse`, which prints one `token: value` line per
/// trailer of the message's trailer block.
#[must_use]
pub fn signed_off_meaning(parsed_trailers: &str) -> SignedOffMeaning {
    let values = trailer_values(parsed_trailers, MEANING_TOKEN);
    SignedOffMeaning {
        presence: if values.is_empty() {
            Presence::Missing
        } else {
            Presence::Present
        },
        values,
    }
}

/// Every value of the trailers named `token` in the output of
/// `git interpret-trailers --parse`, in order.
///
/// That output is one unfolded `token: value` line per trailer. The token
/// matches ASCII case-insensitively and exactly, and the value is trimmed of
/// ASCII whitespace only, as git trims it.
#[must_use]
pub fn trailer_values(parsed_trailers: &str, token: &str) -> Vec<String> {
    const SPACE: [char; 6] = [' ', '\t', '\n', '\r', '\x0b', '\x0c'];
    parsed_trailers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, _)| name.eq_ignore_ascii_case(token))
        .map(|(_, value)| value.trim_matches(SPACE).to_owned())
        .collect()
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
