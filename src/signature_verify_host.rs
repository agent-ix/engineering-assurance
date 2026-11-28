// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Git, `ssh-keygen` and `gitsign` adapter for the `signature-verify` command
//! (FR-036, FR-040).
//!
//! A CMS-signed object is never given to git: the adapter checks that the
//! repository sets no `gitsign.*` key, then runs `gitsign verify` or
//! `gitsign verify-tag` in the exit-status procedure the classifier drives.
//!
//! The adapter reads the allowed-signers file, then runs `git verify-commit`,
//! `git log --format=%G?%x00%GF%x00%GS`, `git verify-tag`, `git
//! interpret-trailers --parse` and, for a tag, `ssh-keygen -Y find-principals`
//! (git accepted it) or `ssh-keygen -Y check-novalidate` (git rejected it),
//! through the shared process host. It reads exit statuses
//! and fixed signals only, hands them to the pure classifier, and never writes
//! to the repository.

use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

use engineering_assurance::signature_verify::{
    CommitObservation, EvidencedGesture, GitGrade, GitsignIdentity, GitsignTrust,
    IDENTITY_REGISTRY_PROTOCOL, IdentityRegistry, PrincipalLookup, Probe, Reason, RegistryEntry,
    Run, SignatureFormat, SignedAt, SplitTag, SshCheck, Status, TagObservation, TargetKind,
    TrustSourceAddress, Verdict, classify_commit, classify_gitsign, classify_tag,
    commit_signature_format, config_sets_gitsign_key, is_plain_revision, object_message,
    signed_off_meaning, split_tag_object, tagger_signed_at, trailer_values,
};
use ix_trace_rs::implements;
use serde::{Deserialize, Deserializer, Serialize, de::IgnoredAny};
use thiserror::Error;

use crate::process_host::{self, CompletedProcess, ProcessError, ProcessLimits};
use crate::producer_identity::ProducerIdentity;

const REQUEST_PROTOCOL: &str = "engineering-assurance.signature-verify-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.signature-verify-result/v1";
const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
/// Largest accepted allowed-signers file in bytes.
const MAX_ALLOWED_SIGNERS_BYTES: u64 = 1024 * 1024;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(60);
/// Largest accepted trust source (identity registry) in bytes.
const MAX_TRUST_SOURCE_BYTES: u64 = 1024 * 1024;

/// Repository-selecting and configuration variables git must not inherit.
pub(crate) const REMOVED_ENVIRONMENT: [&str; 12] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_EXTERNAL_DIFF",
    "GIT_DIFF_OPTS",
    "GIT_NAMESPACE",
    "GIT_COMMON_DIR",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_REPLACE_REF_BASE",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    repository: PathBuf,
    revision: String,
    kind: TargetKind,
    allowed_signers: String,
    gitsign: Option<GitsignRequest>,
    identity_registry: Option<String>,
    trailer_keys: Option<Vec<String>>,
}

/// The `gitsign` member of a request, as the request documents spell it. It
/// names no person: a `principal` member is recognised only to be refused.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GitsignRequest {
    pub(crate) certificate_identity: String,
    pub(crate) certificate_oidc_issuer: String,
    /// Whether the member carried a `principal`, of any type. The value is
    /// dropped as it is read, so it is never used or echoed (FR-040).
    #[serde(
        default,
        rename = "principal",
        deserialize_with = "member_present",
        skip_serializing
    )]
    principal_supplied: bool,
}

fn member_present<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    IgnoredAny::deserialize(deserializer)?;
    Ok(true)
}

/// Why the `gitsign` and `identity_registry` members of a request are refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GitsignInputError {
    /// The members are malformed, or only one of the two is present.
    Invalid,
    /// A request that otherwise validates names a person in `gitsign`.
    PrincipalRefused,
}

impl GitsignInputError {
    /// The refusal a capability reports: `invalid` for a request error and
    /// `principal` for a refused person's name.
    pub(crate) fn refusal<E>(self, invalid: E, principal: E) -> E {
        match self {
            Self::Invalid => invalid,
            Self::PrincipalRefused => principal,
        }
    }
}

/// A validated `gitsign` identity and the address of the registry that names
/// the person.
pub(crate) struct GitsignInput {
    identity: GitsignIdentity,
    address: TrustSourceAddress,
}

/// Validates the `gitsign` and `identity_registry` members together, before
/// anything is read: `identity_registry` is required with `gitsign` and refused
/// without it, and a `principal` is refused only after everything else is valid.
pub(crate) fn validate_gitsign(
    gitsign: Option<&GitsignRequest>,
    identity_registry: Option<&str>,
) -> Result<Option<GitsignInput>, GitsignInputError> {
    let (gitsign, registry) = match (gitsign, identity_registry) {
        (None, None) => return Ok(None),
        (Some(gitsign), Some(registry)) => (gitsign, registry),
        _ => return Err(GitsignInputError::Invalid),
    };
    let identity = GitsignIdentity::new(
        gitsign.certificate_identity.clone(),
        gitsign.certificate_oidc_issuer.clone(),
    )
    .ok_or(GitsignInputError::Invalid)?;
    let address = TrustSourceAddress::parse(registry).ok_or(GitsignInputError::Invalid)?;
    if gitsign.principal_supplied {
        return Err(GitsignInputError::PrincipalRefused);
    }
    Ok(Some(GitsignInput { identity, address }))
}

/// The `gitsign` trust of a run: the identity, the registry read for it and the
/// commit that registry was read from.
pub(crate) struct LoadedGitsign {
    trust: GitsignTrust,
    commit: String,
}

impl LoadedGitsign {
    /// The full commit id the registry revision resolved to.
    pub(crate) fn commit(&self) -> &str {
        &self.commit
    }
}

/// Why the identity registry could not be used.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RegistryFailure {
    /// The source is `invalid` (FR-037) or its content is not a usable registry.
    Invalid,
    /// Git could not answer (FR-037): the FR-036 reason for the failure.
    Git(Reason),
}

/// Reads the identity registry of `input` once, from the committed revision it
/// names, and validates its content (FR-037, FR-040).
pub(crate) fn load_gitsign(
    repository: &Path,
    input: GitsignInput,
) -> Result<LoadedGitsign, RegistryFailure> {
    let git = Git {
        repository,
        allowed_signers: PathBuf::from("/dev/null"),
        gitsign: None,
    };
    let (commit, content) = git.read_trust_source(&input.address)?;
    let registry = parse_registry(&content).ok_or(RegistryFailure::Invalid)?;
    Ok(LoadedGitsign {
        trust: GitsignTrust::new(input.identity, registry),
        commit,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryDocument {
    protocol: String,
    entries: Vec<RegistryEntryDocument>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryEntryDocument {
    certificate_identity: String,
    certificate_oidc_issuer: String,
    principal: String,
}

/// The identity registry the UTF-8 JSON `content` holds, or `None`.
fn parse_registry(content: &str) -> Option<IdentityRegistry> {
    let document: RegistryDocument = serde_json::from_str(content).ok()?;
    if document.protocol != IDENTITY_REGISTRY_PROTOCOL {
        return None;
    }
    let entries = document
        .entries
        .into_iter()
        .map(|entry| {
            RegistryEntry::new(
                entry.certificate_identity,
                entry.certificate_oidc_issuer,
                entry.principal,
            )
        })
        .collect::<Option<Vec<_>>>()?;
    IdentityRegistry::new(entries)
}

#[derive(Serialize)]
struct ResultLine<'a> {
    protocol: &'static str,
    #[serde(flatten)]
    producer: ProducerIdentity,
    #[serde(flatten)]
    verdict: &'a Verdict,
    #[serde(skip_serializing_if = "Option::is_none")]
    trailers: Option<BTreeMap<String, Vec<String>>>,
}

struct CheckedMessage {
    verdict: Verdict,
    trailers: Option<String>,
}

/// Typed refusal that produces no verdict.
#[derive(Debug, Error)]
pub(crate) enum SignatureVerifyError {
    /// The request is not readable JSON.
    #[error("signature-verify request is not readable JSON")]
    Unreadable,
    /// The request is JSON that is not an exact request document.
    #[error("signature-verify request is not an exact signature-verify request")]
    Invalid,
    /// The revision is empty, too long, option-like or contains whitespace or controls.
    #[error("signature-verify revision is not a plain revision name")]
    Revision,
    /// The allowed-signers file is not a readable regular UTF-8 file of at most 1 MiB.
    #[error(
        "signature-verify allowed-signers file is not a readable UTF-8 regular file of at most 1 MiB"
    )]
    AllowedSigners,
    /// The `gitsign` member names a person, which only the identity registry may do.
    #[error("signature-verify gitsign member must not carry a principal")]
    GitsignPrincipal,
    /// The identity registry source is invalid or its content is unusable.
    #[error("signature-verify identity registry is not a usable committed registry")]
    IdentityRegistry,
    /// The requested trailer keys are not a valid bounded set.
    #[error("signature-verify trailer_keys is not a valid bounded set of trailer keys")]
    TrailerKeys,
    /// The verdict cannot be encoded.
    #[error("signature-verify result cannot be encoded")]
    ResultEncoding,
}

impl SignatureVerifyError {
    /// Stable machine error code.
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Unreadable => "signature_verify_request_unreadable",
            Self::Invalid => "signature_verify_request_invalid",
            Self::Revision => "signature_verify_revision_invalid",
            Self::AllowedSigners => "signature_verify_allowed_signers_invalid",
            Self::GitsignPrincipal => "signature_verify_gitsign_principal_refused",
            Self::IdentityRegistry => "signature_verify_identity_registry_invalid",
            Self::TrailerKeys => "signature_verify_trailer_keys_invalid",
            Self::ResultEncoding => "signature_verify_result_encoding_failed",
        }
    }
}

/// Runs one request document and returns the result line.
pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, SignatureVerifyError> {
    let producer = ProducerIdentity::sample().ok_or(SignatureVerifyError::ResultEncoding)?;
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| SignatureVerifyError::Unreadable)?;
    validate_trailer_keys(value.get("trailer_keys"))?;
    let request: Request =
        serde_json::from_value(value).map_err(|_| SignatureVerifyError::Invalid)?;
    let trailer_keys = request.trailer_keys.as_deref();
    if request.protocol != REQUEST_PROTOCOL || !request.repository.is_absolute() {
        return Err(SignatureVerifyError::Invalid);
    }
    let source = AllowedSignersSource::parse(&request.allowed_signers)
        .ok_or(SignatureVerifyError::Invalid)?;
    if !is_plain_revision(&request.revision) {
        return Err(SignatureVerifyError::Revision);
    }
    let input = validate_gitsign(
        request.gitsign.as_ref(),
        request.identity_registry.as_deref(),
    )
    .map_err(|error| {
        error.refusal(
            SignatureVerifyError::Invalid,
            SignatureVerifyError::GitsignPrincipal,
        )
    })?;
    let allowed_signers = match capture_allowed_signers(&request.repository, &source) {
        Ok(snapshot) => snapshot,
        Err(CaptureFailure::Invalid) => return Err(SignatureVerifyError::AllowedSigners),
        // Git could not answer: a verdict, and no verifier is started.
        Err(CaptureFailure::Git(reason)) => {
            let verdict = Verdict::unverifiable(request.kind, reason);
            return encode_line(&verdict, None, producer);
        }
    };
    let gitsign = match input
        .map(|input| load_gitsign(&request.repository, input))
        .transpose()
    {
        Ok(gitsign) => gitsign,
        Err(RegistryFailure::Invalid) => return Err(SignatureVerifyError::IdentityRegistry),
        // Git could not answer: a verdict, and no verifier is started.
        Err(RegistryFailure::Git(reason)) => {
            let verdict = Verdict::unverifiable(request.kind, reason);
            return encode_line(&verdict, None, producer);
        }
    };
    let git = Git {
        repository: &request.repository,
        allowed_signers: allowed_signers.path.clone(),
        gitsign: gitsign.as_ref(),
    };
    let checked = match request.kind {
        TargetKind::Commit => git.commit_checked(&request.revision),
        TargetKind::Tag => {
            let checked = git.tag_checked(&request.revision);
            CheckedMessage {
                verdict: checked.verdict,
                trailers: checked.trailers,
            }
        }
    };
    let trailers = requested_trailers(checked.trailers.as_deref(), trailer_keys);
    encode_line(&checked.verdict, trailers, producer)
}

fn encode_line(
    verdict: &Verdict,
    trailers: Option<BTreeMap<String, Vec<String>>>,
    producer: ProducerIdentity,
) -> Result<Vec<u8>, SignatureVerifyError> {
    let mut line = serde_json::to_vec(&ResultLine {
        protocol: RESULT_PROTOCOL,
        producer,
        verdict,
        trailers,
    })
    .map_err(|_| SignatureVerifyError::ResultEncoding)?;
    line.push(b'\n');
    Ok(line)
}

fn validate_trailer_keys(
    value: Option<&serde_json::Value>,
) -> Result<Option<Vec<String>>, SignatureVerifyError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(values) = value.as_array() else {
        return Err(SignatureVerifyError::TrailerKeys);
    };
    if values.len() > 16 {
        return Err(SignatureVerifyError::TrailerKeys);
    }
    let mut keys = Vec::with_capacity(values.len());
    for value in values {
        let Some(key) = value.as_str() else {
            return Err(SignatureVerifyError::TrailerKeys);
        };
        let bytes = key.as_bytes();
        if bytes.is_empty()
            || bytes.len() > 64
            || !bytes[0].is_ascii_alphabetic()
            || !bytes
                .iter()
                .all(|byte| byte.is_ascii_alphabetic() || byte.is_ascii_digit() || *byte == b'-')
            || keys
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(key))
        {
            return Err(SignatureVerifyError::TrailerKeys);
        }
        keys.push(key.to_owned());
    }
    Ok(Some(keys))
}

fn requested_trailers(
    parsed: Option<&str>,
    keys: Option<&[String]>,
) -> Option<BTreeMap<String, Vec<String>>> {
    let parsed = parsed?;
    let keys = keys.filter(|keys| !keys.is_empty())?;
    Some(
        keys.iter()
            .map(|key| (key.clone(), trailer_values(parsed, key)))
            .collect(),
    )
}

/// Where the allowed-signers bytes of a request come from (FR-037).
pub(crate) enum AllowedSignersSource {
    /// An absolute file path, read once.
    File(PathBuf),
    /// A `<rev>:<path>` trust source, read from a committed revision.
    Committed(TrustSourceAddress),
}

impl AllowedSignersSource {
    /// The source `value` names: an absolute path when it begins with `/`, a
    /// well-formed `<rev>:<path>` address otherwise; `None` for anything else.
    pub(crate) fn parse(value: &str) -> Option<Self> {
        if value.starts_with('/') {
            Some(Self::File(PathBuf::from(value)))
        } else {
            TrustSourceAddress::parse(value).map(Self::Committed)
        }
    }
}

/// Why the allowed-signers source could not be captured.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CaptureFailure {
    /// The source is `invalid` (FR-037): not a readable regular UTF-8 file of
    /// at most 1 MiB, or a committed source that is absent or unusable.
    Invalid,
    /// Git could not answer (FR-037): the FR-036 reason for the failure.
    Git(Reason),
}

/// Reads the allowed-signers source once, before git is started on the target,
/// digests exactly the bytes read, and writes them to a private temporary file
/// that is the only allowed-signers file `git` and `ssh-keygen` are given
/// (FR-037). A file git should not be given is refused here: not a regular
/// file, over 1 MiB, unreadable, or not UTF-8. The grammar of the content is
/// git's and `ssh-keygen`'s to read.
#[implements("FR-037-CON-3")]
pub(crate) fn capture_allowed_signers(
    repository: &Path,
    source: &AllowedSignersSource,
) -> Result<AllowedSignersSnapshot, CaptureFailure> {
    let (bytes, commit) = match source {
        AllowedSignersSource::File(path) => (
            read_allowed_signers(path).ok_or(CaptureFailure::Invalid)?,
            None,
        ),
        AllowedSignersSource::Committed(address) => {
            let git = Git {
                repository,
                allowed_signers: PathBuf::from("/dev/null"),
                gitsign: None,
            };
            let (commit, content) =
                git.read_trust_source(address)
                    .map_err(|failure| match failure {
                        RegistryFailure::Invalid => CaptureFailure::Invalid,
                        RegistryFailure::Git(reason) => CaptureFailure::Git(reason),
                    })?;
            (content.into_bytes(), Some(commit))
        }
    };
    let directory = tempfile::tempdir().map_err(|_| CaptureFailure::Invalid)?;
    let path = directory.path().join("allowed_signers");
    std::fs::write(&path, &bytes).map_err(|_| CaptureFailure::Invalid)?;
    let sha256 = sigstore_verify::crypto::sha256(&bytes);
    let sha256 = format!("sha256:{}", sha256.to_hex());
    Ok(AllowedSignersSnapshot {
        sha256,
        commit,
        _directory: directory,
        path,
    })
}

fn read_allowed_signers(path: &Path) -> Option<Vec<u8>> {
    // The type is checked before the open: opening a FIFO would block.
    let before = std::fs::metadata(path).ok()?;
    if !before.is_file() || before.len() > MAX_ALLOWED_SIGNERS_BYTES {
        return None;
    }
    let file = File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > MAX_ALLOWED_SIGNERS_BYTES {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_ALLOWED_SIGNERS_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if u64::try_from(bytes.len()).map_or(true, |len| len > MAX_ALLOWED_SIGNERS_BYTES)
        || std::str::from_utf8(&bytes).is_err()
    {
        return None;
    }
    Some(bytes)
}

/// Maps a process-mechanics failure to the reason it makes the object
/// unverifiable; `not_started` is the reason for a program that cannot start.
const fn mechanics_reason(error: &ProcessError, not_started: Reason) -> Reason {
    match error {
        ProcessError::Unavailable { .. } => not_started,
        ProcessError::TimedOut { .. } => Reason::Timeout,
        ProcessError::OutputTooLarge { .. } => Reason::OutputExceeded,
        ProcessError::PipeUnavailable { .. }
        | ProcessError::Observation { .. }
        | ProcessError::OutputUnreadable { .. } => Reason::UnrecognizedOutput,
    }
}

/// A tag's verdict and the trailer block its message was read to hold.
pub(crate) struct CheckedTag {
    /// The FR-036, FR-037 and FR-040 verdict.
    pub(crate) verdict: Verdict,
    /// The output of `git interpret-trailers --parse` for the tag message;
    /// `None` when the tag has no message to read or git could not read it.
    pub(crate) trailers: Option<String>,
    /// The UTC instant the tag's tagger line claims; `None` when the tag has no
    /// readable tagger line.
    pub(crate) tagger_signed_at: Option<SignedAt>,
    /// Immutable annotated tag object whose message and signature were checked.
    pub(crate) tag_object_id: Option<String>,
    /// The immutable object bytes from which the signature and trailers were
    /// checked, retained for FR-041's Rekor extraction.
    pub(crate) tag_object: Option<Vec<u8>>,
}

impl CheckedTag {
    /// A tag that was never read, because git could not answer while the
    /// identity registry was read (FR-037): `unverifiable` with `reason`.
    pub(crate) const fn unread(reason: Reason) -> Self {
        Self {
            verdict: Verdict::unverifiable(TargetKind::Tag, reason),
            trailers: None,
            tagger_signed_at: None,
            tag_object_id: None,
            tag_object: None,
        }
    }
}

/// Verifies the tag `revision` exactly as `signature-verify` does for kind
/// `tag`, and returns the parsed trailer block read for the same verdict.
pub(crate) fn check_tag(
    repository: &Path,
    revision: &str,
    allowed_signers: &AllowedSignersSnapshot,
    gitsign: Option<&LoadedGitsign>,
) -> CheckedTag {
    Git {
        repository,
        allowed_signers: allowed_signers.path.clone(),
        gitsign,
    }
    .tag_checked(revision)
}

/// The allowed-signers bytes of one run, held as the private file the verifiers
/// are given, with the identity of what that file holds.
pub(crate) struct AllowedSignersSnapshot {
    _directory: tempfile::TempDir,
    pub(crate) path: PathBuf,
    sha256: String,
    commit: Option<String>,
}

impl AllowedSignersSnapshot {
    /// `sha256:` and 64 lowercase hexadecimal digits of the bytes the run used.
    pub(crate) fn sha256(&self) -> &str {
        &self.sha256
    }

    /// The full commit id a `<rev>:<path>` source resolved to; `None` for a file.
    pub(crate) fn commit(&self) -> Option<&str> {
        self.commit.as_deref()
    }
}

/// What `git --version` printed for the git a run used, as printed without its
/// final newline; `None` when git could not answer.
pub(crate) fn git_version(repository: &Path) -> Option<String> {
    let output = run_git(repository, &["--version"]).ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.strip_suffix('\n').unwrap_or(&text);
    (!text.is_empty()).then(|| text.to_owned())
}

/// The refs under `refs/tags`, as `git for-each-ref` lists them.
pub(crate) struct TagListing {
    /// The refs in git's ref-name (byte) order, at most the limit.
    pub(crate) refs: Vec<ListedRef>,
    /// Whether more than `limit` refs exist.
    pub(crate) truncated: bool,
}

/// One ref under `refs/tags`.
pub(crate) struct ListedRef {
    /// The name without `refs/tags/`, as git printed it.
    pub(crate) name: Vec<u8>,
    /// Whether the ref names an annotated tag object.
    pub(crate) annotated: bool,
}

/// Lists up to `limit` refs under `refs/tags` in ref-name order, with the type
/// of the object each names, through the same isolated git as the verification.
/// `None` when the repository cannot be listed.
pub(crate) fn list_tags(
    repository: &Path,
    allowed_signers: &Path,
    limit: usize,
) -> Option<TagListing> {
    let git = Git {
        repository,
        allowed_signers: allowed_signers.to_path_buf(),
        gitsign: None,
    };
    git.require_repository().ok()?;
    let count = format!("--count={}", limit + 1);
    let output = git
        .successful(&[
            "for-each-ref",
            &count,
            "--format=%(refname)%00%(objecttype)",
            "refs/tags",
        ])
        .ok()?;
    let mut lines: Vec<&[u8]> = output
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect();
    let truncated = lines.len() > limit;
    lines.truncate(limit);
    let mut refs = Vec::with_capacity(lines.len());
    for line in lines {
        let mut parts = line.splitn(2, |byte| *byte == 0);
        let (Some(name), Some(kind)) = (parts.next(), parts.next()) else {
            return None;
        };
        let name = name.strip_prefix(b"refs/tags/")?;
        refs.push(ListedRef {
            name: name.to_vec(),
            annotated: kind == b"tag",
        });
    }
    Some(TagListing { refs, truncated })
}

/// An annotated tag read but not yet verified.
pub(crate) struct LoadedTag {
    id: String,
    object: Vec<u8>,
    trailers: Option<String>,
}

impl LoadedTag {
    /// The tag object id.
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    /// The output of `git interpret-trailers --parse` for the tag message;
    /// `None` when git could not read it.
    pub(crate) fn trailers(&self) -> Option<&str> {
        self.trailers.as_deref()
    }
}

/// Loads a tag name returned by `for-each-ref`; its fully qualified ref is
/// already known to be under `refs/tags`, so option-like names remain usable.
pub(crate) fn load_listed_tag(
    repository: &Path,
    name: &str,
    allowed_signers: &Path,
) -> Result<Option<LoadedTag>, Reason> {
    Git {
        repository,
        allowed_signers: allowed_signers.to_path_buf(),
        gitsign: None,
    }
    .load_listed(name)
}

/// Loads one exact tag ref, retaining its immutable object and parsed trailers
/// so another host command can check the expected object id before delegating
/// signature verification.
pub(crate) fn load_tag_ref(
    repository: &Path,
    name: &str,
    allowed_signers: &Path,
) -> Result<Option<LoadedTag>, Reason> {
    Git {
        repository,
        allowed_signers: allowed_signers.to_path_buf(),
        gitsign: None,
    }
    .load(&format!("refs/tags/{name}"))
}

/// Runs git in `repository` with the isolation the verification uses, for
/// object reads that decide nothing about a signature.
pub(crate) fn run_git(repository: &Path, arguments: &[&str]) -> Result<CompletedProcess, Reason> {
    Git {
        repository,
        allowed_signers: PathBuf::from("/dev/null"),
        gitsign: None,
    }
    .run(arguments)
}

/// Verifies a tag read by [`load_listed_tag`], exactly as [`check_tag`] verifies it.
pub(crate) fn verify_loaded_tag(
    repository: &Path,
    allowed_signers: &AllowedSignersSnapshot,
    gitsign: Option<&LoadedGitsign>,
    loaded: LoadedTag,
) -> CheckedTag {
    Git {
        repository,
        allowed_signers: allowed_signers.path.clone(),
        gitsign,
    }
    .verify_loaded(loaded)
}

struct Git<'a> {
    repository: &'a Path,
    allowed_signers: PathBuf,
    gitsign: Option<&'a LoadedGitsign>,
}

/// Name of the verifier program for CMS signatures; never taken from input.
const GITSIGN_PROGRAM: &str = "gitsign";

/// The tag name gitsign verifies in the isolated snapshot repository.
const SNAPSHOT_TAG: &str = "verified";

/// An isolated, non-bare repository whose only tag ref is bound to one
/// resolved tag object id for the whole verification.
struct TagSnapshot {
    directory: tempfile::TempDir,
    object_id: String,
}

impl TagSnapshot {
    /// Whether `refs/tags/verified` still names exactly the resolved object.
    fn binds_object(&self) -> bool {
        let reference = self
            .directory
            .path()
            .join(".git/refs/tags")
            .join(SNAPSHOT_TAG);
        std::fs::read_to_string(reference).is_ok_and(|text| text.trim_end() == self.object_id)
    }
}

/// Environment variables `gitsign` and the configuration check keep from the
/// parent; `GIT_CONFIG_NOSYSTEM` and `GIT_CONFIG_GLOBAL` are set, not kept.
const KEPT_ENVIRONMENT: [&str; 2] = ["PATH", "HOME"];

/// The absolute entries of `path`, which are the only directories a verifier
/// program is looked up in: a relative entry would resolve inside the
/// repository under verification, the working directory of the run.
fn absolute_search_path(path: &OsStr) -> OsString {
    let kept: Vec<PathBuf> = std::env::split_paths(path)
        .filter(|entry| entry.is_absolute())
        .collect();
    std::env::join_paths(kept).unwrap_or_default()
}

/// The executable `name` in the first directory of `search` that holds one.
fn find_executable(name: &str, search: &OsStr) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    std::env::split_paths(search)
        .map(|directory| directory.join(name))
        .find(|candidate| {
            std::fs::metadata(candidate)
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
}

/// Finds `program` for a run: its absolute path and the `PATH` to pass on. Both
/// come from the absolute entries of the inherited `PATH` alone, so a program
/// in the verified repository (the working directory) is never found, whether
/// we start it or a program we start does.
pub(crate) fn resolve_program(program: &str) -> Result<(PathBuf, OsString), ProcessError> {
    let unavailable = || ProcessError::Unavailable {
        detail: format!("{program} is not in an absolute PATH entry"),
    };
    let search = std::env::var_os("PATH")
        .map(|path| absolute_search_path(&path))
        .ok_or_else(unavailable)?;
    let executable = find_executable(program, &search).ok_or_else(unavailable)?;
    Ok((executable, search))
}

/// Runs `program` as `gitsign` and the configuration check need it run: found
/// by absolute path in the absolute entries of the inherited `PATH`, in the
/// repository, with exactly `PATH` (those entries), `HOME`,
/// `GIT_CONFIG_NOSYSTEM=1` and `GIT_CONFIG_GLOBAL=/dev/null`.
fn run_tool(
    program: &str,
    arguments: &[&OsStr],
    repository: &Path,
) -> Result<CompletedProcess, ProcessError> {
    let (executable, search) = resolve_program(program)?;
    let set: Vec<(OsString, OsString)> = vec![
        ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
        ("GIT_CONFIG_GLOBAL".into(), "/dev/null".into()),
        ("PATH".into(), search),
    ];
    let removed: Vec<OsString> = std::env::vars_os()
        .map(|(name, _)| name)
        .filter(|name| !KEPT_ENVIRONMENT.iter().any(|kept| OsStr::new(kept) == name))
        .filter(|name| !set.iter().any(|(set_name, _)| set_name == name))
        .collect();
    let set: Vec<(&OsStr, &OsStr)> = set
        .iter()
        .map(|(name, value)| (name.as_os_str(), value.as_os_str()))
        .collect();
    let removed: Vec<&OsStr> = removed.iter().map(OsString::as_os_str).collect();
    process_host::run_configured(
        executable.as_os_str(),
        arguments,
        Some(repository),
        &set,
        &removed,
        limits(),
    )
}

/// What the process host observed of a delegated run, as the classifier reads it.
fn observed(result: &Result<CompletedProcess, ProcessError>) -> Run {
    match result {
        Ok(output) => output.status.code().map_or(Run::Signaled, Run::Exit),
        Err(ProcessError::Unavailable { .. }) => Run::NotStarted,
        Err(ProcessError::TimedOut { .. }) => Run::TimedOut,
        Err(ProcessError::OutputTooLarge { .. }) => Run::CutShort,
        Err(
            ProcessError::PipeUnavailable { .. }
            | ProcessError::Observation { .. }
            | ProcessError::OutputUnreadable { .. },
        ) => Run::Unobserved,
    }
}

impl Git<'_> {
    /// The commit the identity registry was read from, when one was named.
    fn registry_commit(&self) -> Option<String> {
        self.gitsign.map(|gitsign| gitsign.commit.clone())
    }

    fn commit_checked(&self, revision: &str) -> CheckedMessage {
        let checked = self.commit_checked_unlabelled(revision);
        CheckedMessage {
            verdict: checked
                .verdict
                .with_identity_registry_commit(self.registry_commit()),
            trailers: checked.trailers,
        }
    }

    fn commit_checked_unlabelled(&self, revision: &str) -> CheckedMessage {
        let kind = TargetKind::Commit;
        if let Err(reason) = self.require_repository() {
            return CheckedMessage {
                verdict: Verdict::unverifiable(kind, reason),
                trailers: None,
            };
        }
        let id = match self.resolve(&format!("{revision}^{{commit}}")) {
            Ok(id) => id,
            Err(reason) => {
                return CheckedMessage {
                    verdict: Verdict::unverifiable(kind, reason),
                    trailers: None,
                };
            }
        };
        let object = match self.successful(&["cat-file", "commit", &id]) {
            Ok(object) => object,
            Err(reason) => {
                return CheckedMessage {
                    verdict: Verdict::unverifiable(kind, reason).with_verified_commit_id(Some(id)),
                    trailers: None,
                };
            }
        };
        let trailers = self.parsed_trailers(object_message(&object));
        let meaning = trailers.as_deref().map(signed_off_meaning);
        let format = commit_signature_format(&object);
        let verdict = self
            .commit_verdict(&id, format)
            .unwrap_or_else(|reason| {
                Verdict::unverifiable(kind, reason)
                    .with_signature_kind(format.and_then(SignatureFormat::signature_kind))
            })
            .with_meaning(meaning)
            .with_verified_commit_id(Some(id));
        CheckedMessage { verdict, trailers }
    }

    fn commit_verdict(&self, id: &str, format: Option<SignatureFormat>) -> Result<Verdict, Reason> {
        let git = match format {
            Some(SignatureFormat::Cms) => {
                return self.gitsign_verdict(TargetKind::Commit, id);
            }
            Some(SignatureFormat::Other) => None,
            Some(SignatureFormat::Ssh) | None => Some(self.grade(id)?),
        };
        Ok(classify_commit(&CommitObservation { format, git }))
    }

    /// Delegates a CMS-signed object to `gitsign`; `target` is the commit id or
    /// immutable annotated-tag object id. Without a supplied identity nothing
    /// is started.
    fn gitsign_verdict(&self, kind: TargetKind, target: &str) -> Result<Verdict, Reason> {
        let Some(loaded) = self.gitsign else {
            // The classifier asks for no probe without an identity.
            return Ok(classify_gitsign(kind, None, false, |_| Run::Unobserved));
        };
        let identity = loaded.trust.identity();
        let sets_key = self.repository_sets_gitsign_key()?;
        if sets_key {
            return Ok(classify_gitsign(kind, Some(&loaded.trust), true, |_| {
                Run::Unobserved
            }));
        }
        let tag_snapshot = if kind == TargetKind::Tag {
            Some(self.snapshot_tag_repository(target)?)
        } else {
            None
        };
        let run_repository = tag_snapshot
            .as_ref()
            .map_or(self.repository, |snapshot| snapshot.directory.path());
        let run_target = if tag_snapshot.is_some() {
            SNAPSHOT_TAG
        } else {
            target
        };
        Ok(classify_gitsign(
            kind,
            Some(&loaded.trust),
            false,
            |probe| {
                // The ref must name the resolved object before and after the run;
                // anything else is a run that cannot be trusted.
                if tag_snapshot
                    .as_ref()
                    .is_some_and(|snapshot| !snapshot.binds_object())
                {
                    return Run::SnapshotChanged;
                }
                let run = self.gitsign_run(identity, kind, run_target, run_repository, probe);
                if tag_snapshot
                    .as_ref()
                    .is_some_and(|snapshot| !snapshot.binds_object())
                {
                    Run::SnapshotChanged
                } else {
                    run
                }
            },
        ))
    }

    /// Whether the configuration `gitsign` will see sets any `gitsign.*` key,
    /// asked as gitsign asks it, `git config --get-regexp .*` with no setting
    /// added, in `gitsign`'s own environment, and read as gitsign reads it.
    fn repository_sets_gitsign_key(&self) -> Result<bool, Reason> {
        let arguments = ["config", "--get-regexp", ".*"].map(OsStr::new);
        let output = run_tool("git", &arguments, self.repository)
            .map_err(|error| mechanics_reason(&error, Reason::GitNotStarted))?;
        match output.status.code() {
            Some(0) => Ok(config_sets_gitsign_key(&output.stdout)),
            Some(1) => Ok(false),
            _ => Err(Reason::UnrecognizedOutput),
        }
    }

    /// One `gitsign` invocation of the procedure, as separate arguments and
    /// never through a shell.
    fn gitsign_run(
        &self,
        identity: &GitsignIdentity,
        kind: TargetKind,
        target: &str,
        repository: &Path,
        probe: Probe,
    ) -> Run {
        let (identity_flag, identity_value) = match probe {
            Probe::Exact | Probe::ExactIdentityAnyIssuer => {
                ("--certificate-identity", identity.certificate_identity())
            }
            Probe::AnyIdentityAnyIssuer | Probe::AnyIdentityExactIssuer => {
                ("--certificate-identity-regexp", ".*")
            }
        };
        let (issuer_flag, issuer_value) = match probe {
            Probe::Exact | Probe::AnyIdentityExactIssuer => (
                "--certificate-oidc-issuer",
                identity.certificate_oidc_issuer(),
            ),
            Probe::AnyIdentityAnyIssuer | Probe::ExactIdentityAnyIssuer => {
                ("--certificate-oidc-issuer-regexp", ".*")
            }
        };
        let subcommand = match kind {
            TargetKind::Commit => "verify",
            TargetKind::Tag => "verify-tag",
        };
        let arguments = [
            subcommand,
            identity_flag,
            identity_value,
            issuer_flag,
            issuer_value,
            "--",
            target,
        ]
        .map(OsStr::new);
        observed(&run_tool(GITSIGN_PROGRAM, &arguments, repository))
    }

    /// Makes an ordinary, non-bare repository whose sole tag ref,
    /// `refs/tags/verified`, names the already-resolved tag object, stored in
    /// the snapshot as a loose object. gitsign needs a tag ref and a normal
    /// `.git` layout, and its git library cannot read packed objects through an
    /// alternate, so the snapshot holds the object itself. The source repository
    /// is neither read nor reachable during verification, and a moved source ref
    /// or changed source configuration cannot affect it.
    fn snapshot_tag_repository(&self, id: &str) -> Result<TagSnapshot, Reason> {
        let object = self.successful(&["cat-file", "tag", id])?;
        let directory = tempfile::tempdir().map_err(|_| Reason::SnapshotUnavailable)?;
        let git_directory = directory.path().join(".git");
        let unavailable = |_| Reason::SnapshotUnavailable;
        std::fs::create_dir_all(git_directory.join("objects")).map_err(unavailable)?;
        std::fs::create_dir_all(git_directory.join("refs/tags")).map_err(unavailable)?;
        std::fs::write(
            git_directory.join("config"),
            "[core]\n\trepositoryformatversion = 0\n\tbare = false\n",
        )
        .map_err(unavailable)?;
        std::fs::write(
            git_directory.join("HEAD"),
            format!("ref: refs/tags/{SNAPSHOT_TAG}\n"),
        )
        .map_err(unavailable)?;
        std::fs::write(
            git_directory.join("refs/tags").join(SNAPSHOT_TAG),
            format!("{id}\n"),
        )
        .map_err(unavailable)?;
        let object_path = git_directory.join("tag-object");
        std::fs::write(&object_path, &object).map_err(unavailable)?;
        let stdin = File::open(&object_path).map_err(unavailable)?;
        let stored = self.run_with_stdin(
            &["hash-object", "-t", "tag", "-w", "--stdin"],
            Some(stdin),
            Some(directory.path()),
        )?;
        std::fs::remove_file(&object_path).map_err(unavailable)?;
        // The stored bytes must hash to the resolved id, or nothing is verified.
        if !stored.status.success() {
            return Err(Reason::SnapshotUnavailable);
        }
        if String::from_utf8_lossy(&stored.stdout).trim_end() != id {
            return Err(Reason::SnapshotObjectMismatch);
        }
        let snapshot = TagSnapshot {
            directory,
            object_id: id.to_owned(),
        };
        if !snapshot.binds_object() {
            return Err(Reason::SnapshotUnavailable);
        }
        Ok(snapshot)
    }

    /// Reads a trust source `<rev>:<path>` from the committed git object, once,
    /// as FR-037 defines: the revision is resolved to a full commit id (an
    /// annotated tag peeled, replace refs and grafts ignored) and the content is
    /// the blob at the path in that commit's tree, never the working tree or the
    /// index. Returns the commit id and the content.
    fn read_trust_source(
        &self,
        address: &TrustSourceAddress,
    ) -> Result<(String, String), RegistryFailure> {
        let git_failed = RegistryFailure::Git;
        self.require_repository().map_err(git_failed)?;
        let commit = match self.resolve(&format!("{}^{{commit}}", address.revision())) {
            Ok(commit) => commit,
            Err(Reason::RevisionUnresolved) => return Err(RegistryFailure::Invalid),
            Err(reason) => return Err(git_failed(reason)),
        };
        let listing = self
            .successful(&[
                "--literal-pathspecs",
                "ls-tree",
                "-z",
                "--full-tree",
                &commit,
                "--",
                address.path(),
            ])
            .map_err(git_failed)?;
        let object =
            regular_file_object(&listing, address.path()).ok_or(RegistryFailure::Invalid)?;
        let size = self
            .successful(&["cat-file", "-s", &object])
            .map_err(git_failed)?;
        let size: u64 = std::str::from_utf8(&size)
            .ok()
            .and_then(|text| text.trim_end().parse().ok())
            .ok_or(RegistryFailure::Git(Reason::UnrecognizedOutput))?;
        if size > MAX_TRUST_SOURCE_BYTES {
            return Err(RegistryFailure::Invalid);
        }
        let bytes = self
            .successful(&["cat-file", "blob", &object])
            .map_err(git_failed)?;
        let content = String::from_utf8(bytes).map_err(|_| RegistryFailure::Invalid)?;
        Ok((commit, content))
    }

    fn grade(&self, id: &str) -> Result<GitGrade, Reason> {
        let verify_accepted = self.run(&["verify-commit", id])?.status.success();
        let log = self.successful(&[
            "log",
            "-1",
            "--no-show-signature",
            "--format=%G?%x00%GF%x00%GS",
            id,
        ])?;
        let text = String::from_utf8(log).map_err(|_| Reason::UnrecognizedOutput)?;
        let mut fields = text.trim_end_matches('\n').split('\0');
        let (Some(grade), Some(fingerprint), Some(principal), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err(Reason::UnrecognizedOutput);
        };
        let mut codes = grade.chars();
        let (Some(grade), None) = (codes.next(), codes.next()) else {
            return Err(Reason::UnrecognizedOutput);
        };
        Ok(GitGrade {
            verify_accepted,
            grade,
            fingerprint: (!fingerprint.is_empty()).then(|| fingerprint.to_owned()),
            principal: (!principal.is_empty()).then(|| principal.to_owned()),
        })
    }

    fn tag_checked(&self, revision: &str) -> CheckedTag {
        let kind = TargetKind::Tag;
        let without_message = |verdict: Verdict| CheckedTag {
            verdict: verdict.with_identity_registry_commit(self.registry_commit()),
            trailers: None,
            tagger_signed_at: None,
            tag_object_id: None,
            tag_object: None,
        };
        match self.load(revision) {
            Ok(Some(loaded)) => self.verify_loaded(loaded),
            Ok(None) => without_message(classify_tag(&TagObservation::Lightweight)),
            Err(reason) => without_message(Verdict::unverifiable(kind, reason)),
        }
    }

    /// Resolves the tag `revision` and reads its object and trailer block;
    /// `None` for a lightweight tag.
    fn load(&self, revision: &str) -> Result<Option<LoadedTag>, Reason> {
        let Some((id, object)) = self.load_tag(revision, false)? else {
            return Ok(None);
        };
        Ok(Some(self.loaded(id, object)))
    }

    fn load_listed(&self, name: &str) -> Result<Option<LoadedTag>, Reason> {
        let revision = format!("refs/tags/{name}");
        let Some((id, object)) = self.load_tag(&revision, true)? else {
            return Ok(None);
        };
        Ok(Some(self.loaded(id, object)))
    }

    fn loaded(&self, id: String, object: Vec<u8>) -> LoadedTag {
        let split = split_tag_object(&object);
        let payload = split.as_ref().map_or(object.as_slice(), |s| s.payload);
        let trailers = self.parsed_trailers(object_message(payload));
        LoadedTag {
            id,
            object,
            trailers,
        }
    }

    /// Verifies a loaded tag exactly as `tag_checked` does.
    fn verify_loaded(&self, loaded: LoadedTag) -> CheckedTag {
        let kind = TargetKind::Tag;
        let LoadedTag {
            id,
            object,
            trailers,
        } = loaded;
        let split = split_tag_object(&object);
        let payload = split.as_ref().map_or(object.as_slice(), |s| s.payload);
        let meaning = trailers.as_deref().map(signed_off_meaning);
        let format = split.as_ref().map(|s| s.format);
        let verdict = self
            .tag_verdict(&id, split.as_ref())
            .unwrap_or_else(|reason| {
                Verdict::unverifiable(kind, reason)
                    .with_signature_kind(format.and_then(SignatureFormat::signature_kind))
            })
            .with_meaning(meaning)
            .with_identity_registry_commit(self.registry_commit());
        CheckedTag {
            verdict,
            trailers,
            tagger_signed_at: tagger_signed_at(payload),
            tag_object_id: Some(id),
            tag_object: Some(object),
        }
    }

    /// The id and object of an annotated tag; `None` for a lightweight tag.
    fn load_tag(
        &self,
        revision: &str,
        listed_ref: bool,
    ) -> Result<Option<(String, Vec<u8>)>, Reason> {
        self.require_repository()?;
        if !listed_ref && !self.names_tag_ref(revision)? {
            return Err(Reason::RevisionUnresolved);
        }
        let id = self.resolve(revision)?;
        let kind = self.successful(&["cat-file", "-t", &id])?;
        match kind.as_slice() {
            b"commit\n" => Ok(None),
            b"tag\n" => {
                let object = self.successful(&["cat-file", "tag", &id])?;
                Ok(Some((id, object)))
            }
            _ => Err(Reason::RevisionUnresolved),
        }
    }

    fn tag_verdict(&self, id: &str, split: Option<&SplitTag<'_>>) -> Result<Verdict, Reason> {
        if split.is_some_and(|split| split.format == SignatureFormat::Cms) {
            return self.gitsign_verdict(TargetKind::Tag, id);
        }
        let observation = match split {
            Some(split) => self.annotated_tag(id, split)?,
            None => TagObservation::NoSignature,
        };
        let verdict = classify_tag(&observation);
        // FR-084: only an accepted SSH tag signature can evidence a gesture.
        let principal = verdict.signer.as_ref().and_then(|s| s.principal.as_deref());
        Ok(match (verdict.status, split, principal) {
            (Status::SignedValid, Some(split), Some(principal)) => {
                let evidenced = self.evidenced_gesture(split, principal);
                verdict.with_gesture(evidenced)
            }
            _ => verdict,
        })
    }

    /// The gesture one `ssh-keygen -Y verify -vv` run on the tag's own signature
    /// block shows on standard error (FR-084). The block is passed as extracted
    /// and never parsed; a run that fails or cannot start evidences nothing.
    fn evidenced_gesture(&self, split: &SplitTag<'_>, principal: &str) -> EvidencedGesture {
        let run = || -> Option<EvidencedGesture> {
            let directory = tempfile::tempdir().ok()?;
            let signature_path = directory.path().join("signature");
            let payload_path = directory.path().join("payload");
            std::fs::write(&signature_path, split.signature).ok()?;
            std::fs::write(&payload_path, split.payload).ok()?;
            let stdin = File::open(&payload_path).ok()?;
            let arguments = [
                OsStr::new("-Y"),
                OsStr::new("verify"),
                OsStr::new("-vv"),
                OsStr::new("-f"),
                self.allowed_signers.as_os_str(),
                OsStr::new("-I"),
                OsStr::new(principal),
                OsStr::new("-n"),
                OsStr::new("git"),
                OsStr::new("-s"),
                signature_path.as_os_str(),
            ];
            let (executable, path) = resolve_program("ssh-keygen").ok()?;
            let output = process_host::run_configured_stdin(
                executable.as_os_str(),
                &arguments,
                Some(directory.path()),
                &[(OsStr::new("PATH"), path.as_os_str())],
                &[],
                stdin,
                limits(),
            )
            .ok()?;
            output
                .status
                .success()
                .then(|| EvidencedGesture::from_verify_stderr(&output.stderr))
        };
        run().unwrap_or(EvidencedGesture::None)
    }

    /// Whether `revision` names a ref under `refs/tags/`.
    fn names_tag_ref(&self, revision: &str) -> Result<bool, Reason> {
        // A full refs/tags name can contain an option-like short name even
        // though the revision itself does not begin with `-`.
        if revision
            .strip_prefix("refs/tags/")
            .unwrap_or(revision)
            .starts_with('-')
        {
            return Ok(false);
        }
        let output = self.run(&["rev-parse", "--symbolic-full-name", revision])?;
        Ok(output.status.success() && output.stdout.starts_with(b"refs/tags/"))
    }

    fn annotated_tag(&self, id: &str, split: &SplitTag<'_>) -> Result<TagObservation, Reason> {
        if split.format == SignatureFormat::Other {
            return Ok(TagObservation::Signature {
                format: SignatureFormat::Other,
                verify_accepted: false,
                check: None,
                principal: PrincipalLookup::NotRun,
            });
        }
        let verify_accepted = self.run(&["verify-tag", id])?.status.success();
        let (check, principal) = if verify_accepted {
            (None, self.find_principal(split.signature)?)
        } else {
            (
                Some(ssh_check(split.signature, split.payload)?),
                PrincipalLookup::NotRun,
            )
        };
        Ok(TagObservation::Signature {
            format: SignatureFormat::Ssh,
            verify_accepted,
            check,
            principal,
        })
    }

    /// The first principal `ssh-keygen -Y find-principals` prints for `signature`.
    fn find_principal(&self, signature: &[u8]) -> Result<PrincipalLookup, Reason> {
        let directory = tempfile::tempdir().map_err(|_| Reason::UnrecognizedOutput)?;
        let signature_path = directory.path().join("signature");
        std::fs::write(&signature_path, signature).map_err(|_| Reason::UnrecognizedOutput)?;
        let arguments = [
            OsStr::new("-Y"),
            OsStr::new("find-principals"),
            OsStr::new("-s"),
            signature_path.as_os_str(),
            OsStr::new("-f"),
            self.allowed_signers.as_os_str(),
        ];
        let result = resolve_program("ssh-keygen").and_then(|(executable, path)| {
            process_host::run_configured(
                executable.as_os_str(),
                &arguments,
                Some(directory.path()),
                &[(OsStr::new("PATH"), path.as_os_str())],
                &[],
                limits(),
            )
        });
        match result {
            Ok(output) if output.status.success() => {
                let first = std::str::from_utf8(&output.stdout)
                    .ok()
                    .and_then(|text| text.lines().next())
                    .filter(|line| !line.is_empty());
                Ok(first.map_or(PrincipalLookup::Rejected, |line| {
                    PrincipalLookup::Found(line.to_owned())
                }))
            }
            Ok(_) => Ok(PrincipalLookup::Rejected),
            Err(ProcessError::Unavailable { .. }) => Ok(PrincipalLookup::NotStarted),
            Err(error) => Err(mechanics_reason(&error, Reason::UnrecognizedOutput)),
        }
    }

    /// The output of `git interpret-trailers --parse --no-divider` for
    /// `message`, read in the isolated directory `meaning` describes; `None` when
    /// git could not read it.
    fn parsed_trailers(&self, message: &[u8]) -> Option<String> {
        let directory = tempfile::tempdir().ok()?;
        let path = directory.path().join("message");
        if message.contains(&0) {
            return None;
        }
        std::fs::write(&path, message).ok()?;
        let output = self
            .run_with_stdin(
                &["interpret-trailers", "--parse", "--no-divider"],
                Some(File::open(&path).ok()?),
                Some(directory.path()),
            )
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout).ok()
    }

    fn require_repository(&self) -> Result<(), Reason> {
        if !self.repository.is_dir() {
            return Err(Reason::RepositoryUnavailable);
        }
        if self.run(&["rev-parse", "--git-dir"])?.status.success() {
            Ok(())
        } else {
            Err(Reason::RepositoryUnavailable)
        }
    }

    /// Resolves `expression` to a full object id.
    fn resolve(&self, expression: &str) -> Result<String, Reason> {
        let output = self.run(&[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            expression,
        ])?;
        let id = output
            .status
            .success()
            .then(|| String::from_utf8(output.stdout).ok())
            .flatten()
            .map(|text| text.trim_end_matches('\n').to_owned())
            .filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_hexdigit()));
        id.ok_or(Reason::RevisionUnresolved)
    }

    /// Standard output of a git command that must exit with status 0.
    fn successful(&self, arguments: &[&str]) -> Result<Vec<u8>, Reason> {
        let output = self.run(arguments)?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(Reason::UnrecognizedOutput)
        }
    }

    /// Runs git with only the allowed-signers setting from the request.
    fn run(&self, arguments: &[&str]) -> Result<CompletedProcess, Reason> {
        self.run_with_stdin(arguments, None, None)
    }

    /// Runs git with `stdin` as standard input when given. With `isolated`, git
    /// runs in that directory and, with the ceiling on its parent, cannot
    /// discover a repository there or above it (git ignores a ceiling equal to
    /// the working directory itself).
    fn run_with_stdin(
        &self,
        arguments: &[&str],
        stdin: Option<File>,
        isolated: Option<&Path>,
    ) -> Result<CompletedProcess, Reason> {
        let mut setting = OsString::from("gpg.ssh.allowedSignersFile=");
        setting.push(self.allowed_signers.as_os_str());
        let mut all: Vec<&OsStr> = vec![
            OsStr::new("-c"),
            &setting,
            OsStr::new("-c"),
            OsStr::new("gpg.ssh.program=ssh-keygen"),
            OsStr::new("-c"),
            OsStr::new("gpg.program=false"),
            OsStr::new("-c"),
            OsStr::new("gpg.openpgp.program=false"),
            OsStr::new("-c"),
            OsStr::new("gpg.x509.program=false"),
            OsStr::new("-c"),
            OsStr::new("trailer.separators=:"),
        ];
        all.extend(arguments.iter().map(OsStr::new));
        let removed: Vec<&OsStr> = REMOVED_ENVIRONMENT.iter().map(OsStr::new).collect();
        let mut environment: Vec<(&OsStr, &OsStr)> = vec![
            (OsStr::new("GIT_CONFIG_NOSYSTEM"), OsStr::new("1")),
            (OsStr::new("GIT_CONFIG_GLOBAL"), OsStr::new("/dev/null")),
            (OsStr::new("GIT_OPTIONAL_LOCKS"), OsStr::new("0")),
            // Neither a replace ref nor a graft may change what an object id names.
            (OsStr::new("GIT_NO_REPLACE_OBJECTS"), OsStr::new("1")),
            (
                OsStr::new("GIT_GRAFT_FILE"),
                OsStr::new("/nonexistent/grafts"),
            ),
        ];
        if let Some(parent) = isolated.and_then(Path::parent) {
            environment.push((OsStr::new("GIT_CEILING_DIRECTORIES"), parent.as_os_str()));
        }
        let directory = isolated.unwrap_or(self.repository);
        let (git, path) = resolve_program("git")
            .map_err(|error| mechanics_reason(&error, Reason::GitNotStarted))?;
        environment.push((OsStr::new("PATH"), path.as_os_str()));
        match stdin {
            Some(stdin) => process_host::run_configured_stdin(
                git.as_os_str(),
                &all,
                Some(directory),
                &environment,
                &removed,
                stdin,
                limits(),
            ),
            None => process_host::run_configured(
                git.as_os_str(),
                &all,
                Some(directory),
                &environment,
                &removed,
                limits(),
            ),
        }
        .map_err(|error| mechanics_reason(&error, Reason::GitNotStarted))
    }
}

/// The object id of the one regular-file entry (mode 100644 or 100755) that
/// `git ls-tree -z` listed for `path`, or `None` when the path is absent or is a
/// symbolic link, submodule or tree.
fn regular_file_object(listing: &[u8], path: &str) -> Option<String> {
    let mut entries = listing
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty());
    let entry = std::str::from_utf8(entries.next()?).ok()?;
    if entries.next().is_some() {
        return None;
    }
    let (header, name) = entry.split_once('\t')?;
    let mut fields = header.split(' ');
    let (mode, kind, object) = (fields.next()?, fields.next()?, fields.next()?);
    (name == path && kind == "blob" && matches!(mode, "100644" | "100755"))
        .then(|| object.to_owned())
}

const fn limits() -> ProcessLimits {
    ProcessLimits {
        timeout: PROCESS_TIMEOUT,
        max_output_bytes: MAX_OUTPUT_BYTES,
    }
}

/// Checks `signature` over `payload` with `ssh-keygen -Y check-novalidate`,
/// passing both parts unchanged.
fn ssh_check(signature: &[u8], payload: &[u8]) -> Result<SshCheck, Reason> {
    let directory = tempfile::tempdir().map_err(|_| Reason::UnrecognizedOutput)?;
    let signature_path = directory.path().join("signature");
    let payload_path = directory.path().join("payload");
    std::fs::write(&signature_path, signature).map_err(|_| Reason::UnrecognizedOutput)?;
    std::fs::write(&payload_path, payload).map_err(|_| Reason::UnrecognizedOutput)?;
    let stdin = File::open(&payload_path).map_err(|_| Reason::UnrecognizedOutput)?;
    let arguments = [
        OsStr::new("-Y"),
        OsStr::new("check-novalidate"),
        OsStr::new("-n"),
        OsStr::new("git"),
        OsStr::new("-s"),
        signature_path.as_os_str(),
    ];
    let result = resolve_program("ssh-keygen").and_then(|(executable, path)| {
        process_host::run_configured_stdin(
            executable.as_os_str(),
            &arguments,
            Some(directory.path()),
            &[(OsStr::new("PATH"), path.as_os_str())],
            &[],
            stdin,
            limits(),
        )
    });
    match result {
        Ok(output) if output.status.success() => Ok(SshCheck::Accepted),
        Ok(_) => Ok(SshCheck::Rejected),
        Err(ProcessError::Unavailable { .. }) => Ok(SshCheck::NotStarted),
        Err(error) => Err(mechanics_reason(&error, Reason::UnrecognizedOutput)),
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    #[test]
    #[trace("TC-1035", "FR-040-AC-23")]
    fn the_snapshot_binds_only_while_its_ref_names_the_resolved_object() {
        let id = "a".repeat(40);
        let directory = tempfile::tempdir().expect("tempdir");
        let tags = directory.path().join(".git/refs/tags");
        std::fs::create_dir_all(&tags).expect("tags");
        let snapshot = TagSnapshot {
            directory,
            object_id: id.clone(),
        };
        // No ref at all, as when it is deleted.
        assert!(!snapshot.binds_object());
        std::fs::write(tags.join(SNAPSHOT_TAG), format!("{id}\n")).expect("ref");
        assert!(snapshot.binds_object());
        std::fs::write(tags.join(SNAPSHOT_TAG), format!("{}\n", "b".repeat(40))).expect("moved");
        assert!(!snapshot.binds_object());
        std::fs::write(tags.join(SNAPSHOT_TAG), format!("{id}{id}\n")).expect("longer");
        assert!(!snapshot.binds_object());
    }

    #[test]
    #[trace("TC-295", "FR-040-AC-7")]
    fn gitsign_process_failures_are_observed_as_runs() {
        use engineering_assurance::signature_verify::Run;
        let failure = |error| observed(&Err(error));
        assert_eq!(
            failure(ProcessError::TimedOut {
                timeout: PROCESS_TIMEOUT
            }),
            Run::TimedOut
        );
        assert_eq!(
            failure(ProcessError::OutputTooLarge {
                stream: "stdout",
                limit: MAX_OUTPUT_BYTES
            }),
            Run::CutShort
        );
        assert_eq!(
            failure(ProcessError::Unavailable {
                detail: "missing".to_owned()
            }),
            Run::NotStarted
        );
        {
            use std::os::unix::process::ExitStatusExt;
            let killed = CompletedProcess {
                status: std::process::ExitStatus::from_raw(9),
                stdout: Vec::new(),
                stderr: Vec::new(),
            };
            assert_eq!(observed(&Ok(killed)), Run::Signaled);
        }
        assert_eq!(
            failure(ProcessError::PipeUnavailable { stream: "stdout" }),
            Run::Unobserved
        );
    }

    #[test]
    #[trace("TC-255", "FR-036-AC-6")]
    fn timeout_and_oversized_output_are_unverifiable_reasons() {
        let timed_out = mechanics_reason(
            &ProcessError::TimedOut {
                timeout: PROCESS_TIMEOUT,
            },
            Reason::GitNotStarted,
        );
        assert_eq!(timed_out, Reason::Timeout);
        let oversized = mechanics_reason(
            &ProcessError::OutputTooLarge {
                stream: "stdout",
                limit: MAX_OUTPUT_BYTES,
            },
            Reason::GitNotStarted,
        );
        assert_eq!(oversized, Reason::OutputExceeded);
        let not_started = mechanics_reason(
            &ProcessError::Unavailable {
                detail: "missing".to_owned(),
            },
            Reason::GitNotStarted,
        );
        assert_eq!(not_started, Reason::GitNotStarted);
    }
}
