// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Adapter for the `approval-trail` command (FR-043).
//!
//! The annotated tags are listed by the `signature-verify` adapter's isolated
//! git. Each is verified by `check_tag` and decided by the `approval-verify`
//! adapter's `assess`, in process, so no rule of FR-036, FR-037, FR-040 or
//! FR-041 is restated here. This adapter keeps the tags that name the record,
//! orders them with the pure ordering, and renders the result. It starts no
//! process of its own and writes nothing.

use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use engineering_assurance::approval_trail::{
    MAX_TAGS_CONSIDERED, OrderKey, TagNameNotUtf8, UnreadableReason, compare, escape_name, hex,
    is_unprintable, names_record,
};
use engineering_assurance::approval_verify::{
    RECORD_ID_TRAILER, TrailerUnreadable, UnverifiableReason, is_record_value,
};
use engineering_assurance::attestation_signing::WorkflowIdentityDocument;
use engineering_assurance::signature_verify::{SignedAt, trailer_values};
use engineering_assurance::tag_receipt::{
    Candidate, FindingStatus, InvalidReason, Judged, MissingTagReason, TagFinding,
    TagReceiptPolicy, compare as compare_tags, is_receipted,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::approval_verify_host::{Assessment, GitsignEcho, Trust, TrustReading, assess};
use crate::producer_identity::ProducerIdentity;
use crate::signature_verify_host::{
    AllowedSignersSnapshot, AllowedSignersSource, CaptureFailure, GitsignRequest, ListedRef,
    LoadedGitsign, RegistryFailure, capture_allowed_signers, git_version, list_tags, load_gitsign,
    load_listed_tag, validate_gitsign, verify_loaded_tag,
};
use crate::tag_receipt_host::{
    ProductionVerifiers, Trust as TagTrust, VerifierSource, current_tags, read_candidates,
};

const REQUEST_PROTOCOL: &str = "engineering-assurance.approval-trail-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.approval-trail-result/v1";
/// Total time after which no new tag verification is started.
const TOTAL_TIME: Duration = Duration::from_secs(300);

/// How the result is written.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Format {
    /// One line of JSON.
    Json,
    /// A table for people.
    Table,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    repository: PathBuf,
    record_id: String,
    record_digest: String,
    allowed_signers: String,
    tag_receipts: Option<TagReceiptsRequest>,
    gitsign: Option<GitsignRequest>,
    identity_registry: Option<String>,
}

/// The policy files the tag receipts are verified and compared against, both
/// read by the caller from the protected base branch (FR-043).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TagReceiptsRequest {
    workflow_identities: PathBuf,
    receipt_policy: PathBuf,
}

/// Receipt inputs requested by a caller. A missing policy member disables
/// receipt reads and comparisons rather than treating existing notes as a
/// repository tracking record.
struct TagContext {
    identities: Option<WorkflowIdentityDocument>,
    policy: Option<TagReceiptPolicy>,
}

/// Most bytes of either policy file.
const MAX_POLICY_BYTES: u64 = 1024 * 1024;

fn read_policy_file(path: &Path) -> Result<Vec<u8>, ApprovalTrailError> {
    use std::io::Read as _;
    let file = std::fs::File::open(path).map_err(|_| ApprovalTrailError::Invalid)?;
    let mut bytes = Vec::new();
    file.take(MAX_POLICY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ApprovalTrailError::Invalid)?;
    if bytes.len() as u64 > MAX_POLICY_BYTES {
        return Err(ApprovalTrailError::Invalid);
    }
    Ok(bytes)
}

fn load_tag_context(request: &TagReceiptsRequest) -> Result<TagContext, ApprovalTrailError> {
    if !request.workflow_identities.is_absolute() || !request.receipt_policy.is_absolute() {
        return Err(ApprovalTrailError::Invalid);
    }
    let policy = TagReceiptPolicy::from_receipt_policy(&read_policy_file(&request.receipt_policy)?)
        .map_err(|_| ApprovalTrailError::Invalid)?;
    let identities = policy
        .as_ref()
        .map(|_| {
            WorkflowIdentityDocument::parse(&read_policy_file(&request.workflow_identities)?)
                .map_err(|_| ApprovalTrailError::Invalid)
        })
        .transpose()?;
    Ok(TagContext { identities, policy })
}

/// Why a trail is a partial list.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum Truncation {
    TagLimit,
    TimeLimit,
}

#[derive(Serialize)]
struct Entry {
    tag: String,
    /// The id of the tag object verified, so copies of one object under several
    /// ref names can be told apart from independent approvals.
    object: String,
    #[serde(flatten)]
    producer: ProducerIdentity,
    /// The tagger date the tag claims; the signer's own claim on any status.
    #[serde(skip_serializing_if = "Option::is_none")]
    claimed_at: Option<SignedAt>,
    #[serde(flatten)]
    assessment: Assessment,
    /// Whether a verified tag receipt names this ref and object (FR-081);
    /// absent when the request carried no `tag_receipts`.
    #[serde(flatten)]
    receipt: Option<ReceiptState>,
}

/// `verified` when a verified receipt names the tag's ref and object, else `none`.
#[derive(Serialize)]
struct ReceiptState {
    tag_receipt: ReceiptStateKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    accepted_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    accepted_reason: Option<String>,
    /// Whether a `none` still counts against `complete`: the tag has a policy
    /// prefix and no `accepted_tags` entry accepts it. Not part of the result.
    #[serde(skip)]
    open: bool,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum ReceiptStateKind {
    Verified,
    None,
}

/// An entry of a receipt whose tag is absent or changed, or that does not verify.
#[derive(Serialize)]
struct FindingEntry {
    #[serde(flatten)]
    producer: ProducerIdentity,
    /// `missing_tag`, `tag_no_receipt`, `invalid_receipt` or `accepted`.
    status: &'static str,
    /// The status an acceptance replaced.
    #[serde(skip_serializing_if = "Option::is_none")]
    finding: Option<FindingStatus>,
    /// Absent for an invalid receipt: its content is not trusted.
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
    /// The receipted object id.
    #[serde(skip_serializing_if = "Option::is_none")]
    object: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<FindingEntryReason>,
    /// `none` for a current prefix-matching tag without a receipt.
    #[serde(skip_serializing_if = "Option::is_none")]
    tag_receipt: Option<ReceiptStateKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_object: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pushed_at: Option<String>,
    /// False only for a receipt whose trailers were not read, and always for an
    /// invalid receipt.
    attributed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    accepted_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    accepted_reason: Option<String>,
}

#[derive(Clone, Copy, Serialize)]
#[serde(untagged)]
enum FindingEntryReason {
    Missing(MissingTagReason),
    Invalid(InvalidReason),
}

/// One line of the trail: the entry of a present tag, or a tag receipt finding.
#[derive(Serialize)]
#[serde(untagged)]
enum TrailEntry {
    Tag(Box<Entry>),
    Finding(Box<FindingEntry>),
}

impl TrailEntry {
    fn key(&self) -> OrderKey<'_> {
        match self {
            Self::Tag(entry) => OrderKey {
                claimed_at: entry.claimed_at.as_ref().map(SignedAt::as_str),
                tag: Some(&entry.tag),
                object: &entry.object,
                commit: "",
            },
            Self::Finding(entry) => OrderKey {
                claimed_at: None,
                tag: entry.tag.as_deref(),
                object: entry.object.as_deref().unwrap_or(""),
                commit: entry.commit.as_deref().unwrap_or(""),
            },
        }
    }

    fn status_text(&self) -> Result<String, ApprovalTrailError> {
        match self {
            Self::Tag(entry) => text_of(&entry.assessment.status()),
            Self::Finding(entry) => Ok(entry.status.to_owned()),
        }
    }
}

impl FindingEntry {
    fn new(judged: &Judged, producer: &ProducerIdentity) -> Self {
        let (status, finding) = match &judged.accepted {
            Some(_) => ("accepted", Some(judged.finding.status())),
            None => (
                match judged.finding.status() {
                    FindingStatus::MissingTag => "missing_tag",
                    FindingStatus::TagNoReceipt => "tag_no_receipt",
                    FindingStatus::InvalidReceipt => "invalid_receipt",
                },
                None,
            ),
        };
        let mut entry = match &judged.finding {
            TagFinding::MissingTag {
                receipt,
                reason,
                current_object,
            } => Self {
                producer: producer.clone(),
                status,
                finding,
                tag: Some(
                    receipt
                        .reference
                        .strip_prefix("refs/tags/")
                        .unwrap_or(&receipt.reference)
                        .to_owned(),
                ),
                object: Some(receipt.tag_object.clone()),
                reason: Some(FindingEntryReason::Missing(*reason)),
                tag_receipt: None,
                current_object: current_object.clone(),
                commit: Some(receipt.commit.clone()),
                pushed_at: Some(receipt.pushed_at.clone()),
                attributed: receipt.trailers_readable,
                accepted_by: None,
                accepted_reason: None,
            },
            TagFinding::InvalidReceipt(invalid) => Self {
                producer: producer.clone(),
                status,
                finding,
                tag: None,
                object: None,
                reason: Some(FindingEntryReason::Invalid(invalid.reason)),
                tag_receipt: None,
                current_object: None,
                commit: Some(invalid.commit.clone()),
                pushed_at: None,
                attributed: false,
                accepted_by: None,
                accepted_reason: None,
            },
            TagFinding::TagNoReceipt {
                reference,
                current_object,
            } => Self {
                producer: producer.clone(),
                status,
                finding,
                tag: Some(
                    reference
                        .strip_prefix("refs/tags/")
                        .unwrap_or(reference)
                        .to_owned(),
                ),
                object: Some(current_object.clone()),
                reason: None,
                tag_receipt: Some(ReceiptStateKind::None),
                current_object: None,
                commit: None,
                pushed_at: None,
                attributed: true,
                accepted_by: None,
                accepted_reason: None,
            },
        };
        if let Some(acceptance) = &judged.accepted {
            entry.accepted_by = Some(acceptance.accepted_by.clone());
            entry.accepted_reason = Some(acceptance.accepted_reason.clone());
        }
        entry
    }
}

/// The `tag_receipts` object of the result.
#[derive(Serialize)]
struct TagReceiptsResult {
    /// Whether the request carried `tag_receipts` and the notes ref was read.
    checked: bool,
    /// The receipt entries read.
    considered: usize,
}

/// An annotated tag that could name the record but cannot be read.
#[derive(Serialize)]
struct Unreadable {
    /// The tag name, with bytes that are not UTF-8 written `\xNN`.
    tag: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    tag_escaped: bool,
    /// The raw name bytes in hexadecimal, present only when the name is escaped.
    #[serde(skip_serializing_if = "Option::is_none")]
    tag_hex: Option<String>,
    reason: UnreadableReason,
}

impl Unreadable {
    fn new(name: &[u8], reason: UnreadableReason) -> Self {
        let (tag, tag_escaped) = escape_name(name);
        Self {
            tag,
            tag_escaped,
            tag_hex: tag_escaped.then(|| hex(name)),
            reason,
        }
    }
}

#[derive(Serialize)]
struct ResultLine<'a> {
    protocol: &'static str,
    #[serde(flatten)]
    producer: ProducerIdentity,
    record_id: &'a str,
    record_digest: &'a str,
    trust: Trust<'a>,
    ordering: &'static str,
    entries: Vec<TrailEntry>,
    tag_receipts: TagReceiptsResult,
    unreadable: Vec<Unreadable>,
    considered: usize,
    truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncation: Option<Truncation>,
    complete: bool,
}

/// Typed refusal that produces no trail.
#[derive(Debug, Error)]
pub(crate) enum ApprovalTrailError {
    /// The request is not readable JSON.
    #[error("approval-trail request is not readable JSON")]
    Unreadable,
    /// The request is JSON that is not an exact approval-trail request.
    #[error("approval-trail request is not an exact approval-trail request")]
    Invalid,
    /// The allowed-signers file is not a readable regular UTF-8 file of at most 1 MiB.
    #[error(
        "approval-trail allowed-signers file is not a readable UTF-8 regular file of at most 1 MiB"
    )]
    AllowedSigners,
    /// The repository cannot be listed.
    #[error("approval-trail repository is not a git repository whose tags can be listed")]
    Repository,
    /// The `gitsign` member names a person, which only the identity registry may do.
    #[error("approval-trail gitsign member must not carry a principal")]
    GitsignPrincipal,
    /// The identity registry source is invalid or its content is unusable.
    #[error("approval-trail identity registry is not a usable committed registry")]
    IdentityRegistry,
    /// The result cannot be encoded.
    #[error("approval-trail result cannot be encoded")]
    ResultEncoding,
}

impl ApprovalTrailError {
    /// Stable machine error code.
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Unreadable => "approval_trail_request_unreadable",
            Self::Invalid => "approval_trail_request_invalid",
            Self::AllowedSigners => "approval_trail_allowed_signers_invalid",
            Self::Repository => "approval_trail_repository_unavailable",
            Self::GitsignPrincipal => "approval_trail_gitsign_principal_refused",
            Self::IdentityRegistry => "approval_trail_identity_registry_invalid",
            Self::ResultEncoding => "approval_trail_result_encoding_failed",
        }
    }
}

/// Runs one request document and returns the rendered result.
pub(crate) fn execute(bytes: &[u8], format: Format) -> Result<Vec<u8>, ApprovalTrailError> {
    execute_within(bytes, format, TOTAL_TIME)
}

/// A parsed request and its `gitsign` trust.
struct Parsed {
    request: Request,
    gitsign: Option<LoadedGitsign>,
    allowed_signers: AllowedSignersSnapshot,
    git_version: Option<String>,
    tags: Option<TagContext>,
    producer: ProducerIdentity,
}

fn parse(bytes: &[u8], producer: ProducerIdentity) -> Result<Parsed, ApprovalTrailError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| ApprovalTrailError::Unreadable)?;
    let request: Request =
        serde_json::from_value(value).map_err(|_| ApprovalTrailError::Invalid)?;
    if request.protocol != REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !is_record_value(&request.record_id)
        || !is_record_value(&request.record_digest)
    {
        return Err(ApprovalTrailError::Invalid);
    }
    let source =
        AllowedSignersSource::parse(&request.allowed_signers).ok_or(ApprovalTrailError::Invalid)?;
    let input = validate_gitsign(
        request.gitsign.as_ref(),
        request.identity_registry.as_deref(),
    )
    .map_err(|error| {
        error.refusal(
            ApprovalTrailError::Invalid,
            ApprovalTrailError::GitsignPrincipal,
        )
    })?;
    // Git failing while the allowed-signers source is read is the repository
    // being unavailable, so there is no result and no trust.
    let allowed_signers =
        capture_allowed_signers(&request.repository, &source).map_err(|failure| match failure {
            CaptureFailure::Invalid => ApprovalTrailError::AllowedSigners,
            CaptureFailure::Git(_) => ApprovalTrailError::Repository,
        })?;
    // Git failing while the registry is read is the repository being unavailable.
    let gitsign = input
        .map(|input| load_gitsign(&request.repository, input))
        .transpose()
        .map_err(|failure| match failure {
            RegistryFailure::Invalid => ApprovalTrailError::IdentityRegistry,
            RegistryFailure::Git(_) => ApprovalTrailError::Repository,
        })?;
    let git_version = git_version(&request.repository);
    let tags = request
        .tag_receipts
        .as_ref()
        .map(load_tag_context)
        .transpose()?;
    Ok(Parsed {
        request,
        gitsign,
        allowed_signers,
        git_version,
        tags,
        producer,
    })
}

/// What one annotated tag contributes to the trail.
enum Examined {
    /// A readable tag that does not name the record.
    Other,
    Entry(Box<Entry>),
    Unreadable(Unreadable),
}

/// Loads the annotated tag `listed`, and verifies it only when it names the record.
fn examine(parsed: &Parsed, listed: &ListedRef) -> Examined {
    let request = &parsed.request;
    let Ok(name) = std::str::from_utf8(&listed.name) else {
        return Examined::Unreadable(Unreadable::new(
            &listed.name,
            UnreadableReason::Name(TagNameNotUtf8::TagNameNotUtf8),
        ));
    };
    let loaded = match load_listed_tag(&request.repository, name, &parsed.allowed_signers.path) {
        Ok(Some(loaded)) => loaded,
        // The ref was replaced by a lightweight tag after it was listed.
        Ok(None) => return Examined::Other,
        Err(reason) => {
            return Examined::Unreadable(Unreadable::new(
                &listed.name,
                UnreadableReason::read_failure(reason),
            ));
        }
    };
    let Some(trailers) = loaded.trailers() else {
        return Examined::Unreadable(Unreadable::new(
            &listed.name,
            UnreadableReason::Unverifiable(UnverifiableReason::Trailer(
                TrailerUnreadable::TrailerUnreadable,
            )),
        ));
    };
    if !names_record(
        &trailer_values(trailers, RECORD_ID_TRAILER),
        &request.record_id,
    ) {
        return Examined::Other;
    }
    let object = loaded.id().to_owned();
    let checked = verify_loaded_tag(
        &request.repository,
        &parsed.allowed_signers,
        parsed.gitsign.as_ref(),
        loaded,
    );
    Examined::Entry(Box::new(Entry {
        tag: name.to_owned(),
        object,
        producer: parsed.producer.clone(),
        claimed_at: checked.tagger_signed_at.clone(),
        assessment: assess(&checked, &request.record_id, &request.record_digest),
        receipt: None,
    }))
}

/// As [`execute`], starting no tag load after `total_time`.
fn execute_within(
    bytes: &[u8],
    format: Format,
    total_time: Duration,
) -> Result<Vec<u8>, ApprovalTrailError> {
    execute_with(bytes, format, total_time, &ProductionVerifiers::default())
}

/// Whether a receipt candidate belongs in the trail of `record_id`: its
/// `record_ids` hold the id byte for byte, or its trailers were not read (or it
/// does not verify), so it belongs to no record by content and is shown in all.
fn belongs(candidate: &Candidate, record_id: &str) -> bool {
    match candidate {
        Candidate::Invalid(_) => true,
        Candidate::Verified(receipt) => {
            !receipt.trailers_readable || names_record(&receipt.record_ids, record_id)
        }
    }
}

/// The entries of the trail once the tag receipts are applied.
struct TagSection {
    entries: Vec<TrailEntry>,
    considered: usize,
    /// No open `missing_tag`, `invalid_receipt` or unreceipted prefix tag.
    complete: bool,
}

/// Verifies the receipts, compares them with the tag set and attaches the
/// result to the entries (FR-043, FR-082).
fn tag_section(
    parsed: &Parsed,
    context: &TagContext,
    entries: Vec<Entry>,
    source: &dyn VerifierSource,
    producer: &ProducerIdentity,
) -> Result<TagSection, ApprovalTrailError> {
    let request = &parsed.request;
    let identities = context
        .identities
        .as_ref()
        .ok_or(ApprovalTrailError::Invalid)?;
    let policy = context.policy.as_ref().ok_or(ApprovalTrailError::Invalid)?;
    let trust = TagTrust::new(identities, source).map_err(|_| ApprovalTrailError::Repository)?;
    let candidates =
        read_candidates(&request.repository, &trust).map_err(|_| ApprovalTrailError::Repository)?;
    let tags = current_tags(&request.repository).map_err(|_| ApprovalTrailError::Repository)?;
    let belonging: Vec<Candidate> = candidates
        .iter()
        .filter(|candidate| belongs(candidate, &request.record_id))
        .cloned()
        .collect();
    let judged = compare_tags(&belonging, &tags, policy);
    let mut complete = true;
    let mut out = Vec::with_capacity(entries.len() + judged.len());
    for mut entry in entries {
        let reference = format!("refs/tags/{}", entry.tag);
        let state = if is_receipted(&candidates, &reference, &entry.object) {
            ReceiptState {
                tag_receipt: ReceiptStateKind::Verified,
                accepted_by: None,
                accepted_reason: None,
                open: false,
            }
        } else {
            let acceptance = policy.accepts_unreceipted(&reference, &entry.object);
            ReceiptState {
                tag_receipt: ReceiptStateKind::None,
                open: policy.covers(&entry.tag) && acceptance.is_none(),
                accepted_by: acceptance.as_ref().map(|a| a.accepted_by.clone()),
                accepted_reason: acceptance.map(|a| a.accepted_reason),
            }
        };
        complete &= !state.open;
        entry.receipt = Some(state);
        out.push(TrailEntry::Tag(Box::new(entry)));
    }
    for finding in &judged {
        if let TagFinding::TagNoReceipt {
            reference,
            current_object,
        } = &finding.finding
        {
            let name = reference.strip_prefix("refs/tags/").unwrap_or(reference);
            let already_in_approval_entries = out.iter().any(|entry| match entry {
                TrailEntry::Tag(entry) => entry.tag == name && entry.object == *current_object,
                TrailEntry::Finding(_) => false,
            });
            if already_in_approval_entries {
                continue;
            }
        }
        complete &= !finding.is_open();
        out.push(TrailEntry::Finding(Box::new(FindingEntry::new(
            finding, producer,
        ))));
    }
    Ok(TagSection {
        entries: out,
        considered: candidates.len(),
        complete,
    })
}

/// As [`execute_within`], with the verifiers of the tag receipts supplied.
fn execute_with(
    bytes: &[u8],
    format: Format,
    total_time: Duration,
    verifiers: &dyn VerifierSource,
) -> Result<Vec<u8>, ApprovalTrailError> {
    let producer = ProducerIdentity::sample().ok_or(ApprovalTrailError::ResultEncoding)?;
    let parsed = parse(bytes, producer)?;
    let request = &parsed.request;
    let listing = list_tags(
        &request.repository,
        &parsed.allowed_signers.path,
        MAX_TAGS_CONSIDERED,
    )
    .ok_or(ApprovalTrailError::Repository)?;

    let deadline = Instant::now() + total_time;
    let mut truncation = listing.truncated.then_some(Truncation::TagLimit);
    let mut considered = listing.refs.len();
    let mut unreadable = Vec::new();
    let mut entries = Vec::new();
    for (index, listed) in listing.refs.iter().enumerate() {
        if !listed.annotated {
            continue;
        }
        if Instant::now() >= deadline {
            truncation = Some(Truncation::TimeLimit);
            considered = index;
            break;
        }
        match examine(&parsed, listed) {
            Examined::Other => {}
            Examined::Entry(entry) => entries.push(*entry),
            Examined::Unreadable(item) => unreadable.push(item),
        }
    }
    let (mut trail, tag_receipts, tags_complete) = match &parsed.tags {
        None => (
            entries
                .into_iter()
                .map(|entry| TrailEntry::Tag(Box::new(entry)))
                .collect(),
            TagReceiptsResult {
                checked: false,
                considered: 0,
            },
            false,
        ),
        Some(context) if context.policy.is_none() => (
            entries
                .into_iter()
                .map(|entry| TrailEntry::Tag(Box::new(entry)))
                .collect(),
            TagReceiptsResult {
                checked: false,
                considered: 0,
            },
            true,
        ),
        Some(context) => {
            let section = tag_section(&parsed, context, entries, verifiers, &parsed.producer)?;
            (
                section.entries,
                TagReceiptsResult {
                    checked: true,
                    considered: section.considered,
                },
                section.complete,
            )
        }
    };
    trail.sort_by(|left, right| compare(&left.key(), &right.key()));
    let result = ResultLine {
        protocol: RESULT_PROTOCOL,
        producer: parsed.producer.clone(),
        record_id: &request.record_id,
        record_digest: &request.record_digest,
        trust: Trust::new(
            TrustReading {
                requested: &request.allowed_signers,
                snapshot: &parsed.allowed_signers,
                git_version: parsed.git_version.as_deref(),
            },
            GitsignEcho::new(
                request.gitsign.as_ref(),
                request.identity_registry.as_deref(),
            ),
            parsed.gitsign.as_ref(),
        ),
        ordering: "claimed_at_ascending",
        complete: truncation.is_none() && unreadable.is_empty() && tags_complete,
        entries: trail,
        tag_receipts,
        unreadable,
        considered,
        truncated: truncation.is_some(),
        truncation,
    };
    let mut out = match format {
        Format::Json => {
            serde_json::to_vec(&result).map_err(|_| ApprovalTrailError::ResultEncoding)?
        }
        Format::Table => table(&result)?.into_bytes(),
    };
    out.push(b'\n');
    Ok(out)
}

/// The text of a value that serializes as one JSON string.
fn text_of<T: Serialize>(value: &T) -> Result<String, ApprovalTrailError> {
    match serde_json::to_value(value) {
        Ok(Value::String(text)) => Ok(text),
        _ => Err(ApprovalTrailError::ResultEncoding),
    }
}

/// A cell for people: unprintable characters are replaced, an absent value is `-`.
fn cell(text: Option<&str>) -> String {
    match text {
        Some(text) if !text.is_empty() => text
            .chars()
            .map(|c| if is_unprintable(c) { '?' } else { c })
            .collect(),
        _ => "-".to_owned(),
    }
}

fn table(result: &ResultLine<'_>) -> Result<String, ApprovalTrailError> {
    let mut rows = vec![
        [
            "CLAIMED_AT",
            "STATUS",
            "REASON",
            "APPROVER",
            "MEANING",
            "TAG",
        ]
        .map(str::to_owned),
    ];
    for trail_entry in &result.entries {
        let status = trail_entry.status_text()?;
        match trail_entry {
            TrailEntry::Tag(entry) => {
                let assessment = &entry.assessment;
                let reason = assessment.reason().map(|r| text_of(&r)).transpose()?;
                let meaning = assessment
                    .approval()
                    .map(|approval| approval.meaning.join("; "));
                rows.push([
                    cell(entry.claimed_at.as_ref().map(SignedAt::as_str)),
                    cell(Some(&status)),
                    cell(reason.as_deref()),
                    cell(assessment.approval().map(|a| a.approver.as_str())),
                    cell(meaning.as_deref()),
                    cell(Some(&entry.tag)),
                ]);
            }
            TrailEntry::Finding(entry) => rows.push([
                cell(None),
                cell(Some(&status)),
                cell(Some(&text_of(&entry.reason)?)),
                cell(None),
                cell(None),
                cell(entry.tag.as_deref()),
            ]),
        }
    }
    for item in &result.unreadable {
        rows.push([
            cell(None),
            cell(Some("unreadable")),
            cell(Some(&text_of(&item.reason)?)),
            cell(None),
            cell(None),
            cell(Some(&item.tag)),
        ]);
    }
    let mut widths = [0_usize; 6];
    for row in &rows {
        for (width, column) in widths.iter_mut().zip(row) {
            *width = (*width).max(column.chars().count());
        }
    }
    let mut out = String::new();
    for row in &rows {
        let last = row.len() - 1;
        for (index, column) in row.iter().enumerate() {
            if index == last {
                out.push_str(column);
            } else {
                let _ = write!(out, "{column:<width$}  ", width = widths[index]);
            }
        }
        out.push('\n');
    }
    let _ = write!(
        out,
        "considered {}, unreadable {}, truncated {}, complete {}",
        result.considered,
        result.unreadable.len(),
        yes_no(result.truncated),
        yes_no(result.complete),
    );
    Ok(out)
}

const fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
// Keep child-process test setup outside `src` so the production capability scan
// does not classify it as a production host capability.
#[path = "../tests/support/approval_trail_host.rs"]
mod tests;
