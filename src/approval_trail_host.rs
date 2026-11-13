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
    path::PathBuf,
    time::{Duration, Instant},
};

use engineering_assurance::approval_trail::{
    MAX_TAGS_CONSIDERED, OrderKey, TagNameNotUtf8, UnreadableReason, compare, escape_name, hex,
    is_unprintable, names_record,
};
use engineering_assurance::approval_verify::{
    RECORD_ID_TRAILER, TrailerUnreadable, UnverifiableReason, is_record_value,
};
use engineering_assurance::signature_verify::{GitsignIdentity, SignedAt, trailer_values};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::approval_verify_host::{Assessment, Trust, assess};
use crate::signature_verify_host::{
    GitsignRequest, ListedRef, check_allowed_signers, list_tags, load_tag, verify_loaded_tag,
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
    allowed_signers: PathBuf,
    gitsign: Option<GitsignRequest>,
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
    /// The tagger date the tag claims; the signer's own claim on any status.
    #[serde(skip_serializing_if = "Option::is_none")]
    claimed_at: Option<SignedAt>,
    #[serde(flatten)]
    assessment: Assessment,
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
    record_id: &'a str,
    record_digest: &'a str,
    trust: Trust<'a>,
    ordering: &'static str,
    entries: Vec<Entry>,
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
            Self::ResultEncoding => "approval_trail_result_encoding_failed",
        }
    }
}

/// Runs one request document and returns the rendered result.
pub(crate) fn execute(bytes: &[u8], format: Format) -> Result<Vec<u8>, ApprovalTrailError> {
    execute_within(bytes, format, TOTAL_TIME)
}

/// A parsed request and its `gitsign` identity.
struct Parsed {
    request: Request,
    identity: Option<GitsignIdentity>,
}

fn parse(bytes: &[u8]) -> Result<Parsed, ApprovalTrailError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| ApprovalTrailError::Unreadable)?;
    let request: Request =
        serde_json::from_value(value).map_err(|_| ApprovalTrailError::Invalid)?;
    if request.protocol != REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !request.allowed_signers.is_absolute()
        || !is_record_value(&request.record_id)
        || !is_record_value(&request.record_digest)
    {
        return Err(ApprovalTrailError::Invalid);
    }
    let identity = request
        .gitsign
        .clone()
        .map(|g| {
            GitsignIdentity::new(
                g.certificate_identity,
                g.certificate_oidc_issuer,
                g.principal,
            )
            .ok_or(ApprovalTrailError::Invalid)
        })
        .transpose()?;
    check_allowed_signers(&request.allowed_signers)
        .map_err(|_| ApprovalTrailError::AllowedSigners)?;
    Ok(Parsed { request, identity })
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
    let revision = format!("refs/tags/{name}");
    let loaded = match load_tag(&request.repository, &revision, &request.allowed_signers) {
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
        &revision,
        &request.allowed_signers,
        parsed.identity.as_ref(),
        loaded,
    );
    Examined::Entry(Box::new(Entry {
        tag: name.to_owned(),
        object,
        claimed_at: checked.tagger_signed_at.clone(),
        assessment: assess(checked, &request.record_id, &request.record_digest),
    }))
}

/// As [`execute`], starting no tag load after `total_time`.
fn execute_within(
    bytes: &[u8],
    format: Format,
    total_time: Duration,
) -> Result<Vec<u8>, ApprovalTrailError> {
    let parsed = parse(bytes)?;
    let request = &parsed.request;
    let listing = list_tags(
        &request.repository,
        &request.allowed_signers,
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
    entries.sort_by(|left, right| compare(&key(left), &key(right)));
    let result = ResultLine {
        protocol: RESULT_PROTOCOL,
        record_id: &request.record_id,
        record_digest: &request.record_digest,
        trust: Trust {
            allowed_signers: &request.allowed_signers,
            gitsign: request.gitsign.as_ref(),
        },
        ordering: "claimed_at_ascending",
        complete: truncation.is_none() && unreadable.is_empty(),
        entries,
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

fn key(entry: &Entry) -> OrderKey<'_> {
    OrderKey {
        claimed_at: entry.claimed_at.as_ref().map(SignedAt::as_str),
        tag: &entry.tag,
    }
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
    for entry in &result.entries {
        let assessment = &entry.assessment;
        let reason = assessment.reason().map(|r| text_of(&r)).transpose()?;
        let meaning = assessment
            .approval()
            .map(|approval| approval.meaning.join("; "));
        rows.push([
            cell(entry.claimed_at.as_ref().map(SignedAt::as_str)),
            cell(Some(&text_of(&assessment.status())?)),
            cell(reason.as_deref()),
            cell(assessment.approval().map(|a| a.approver.as_str())),
            cell(meaning.as_deref()),
            cell(Some(&entry.tag)),
        ]);
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
