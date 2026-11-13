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

use engineering_assurance::approval_trail::{MAX_TAGS_CONSIDERED, OrderKey, compare, names_record};
use engineering_assurance::approval_verify::{RECORD_ID_TRAILER, is_record_value};
use engineering_assurance::signature_verify::{GitsignIdentity, SignedAt, trailer_values};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::approval_verify_host::{Assessment, Trust, assess};
use crate::signature_verify_host::{GitsignRequest, check_allowed_signers, check_tag, list_tags};

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
    /// The tagger date the tag claims; the signer's own claim on any status.
    #[serde(skip_serializing_if = "Option::is_none")]
    claimed_at: Option<SignedAt>,
    #[serde(flatten)]
    assessment: Assessment,
}

#[derive(Serialize)]
struct ResultLine<'a> {
    protocol: &'static str,
    record_id: &'a str,
    record_digest: &'a str,
    trust: Trust<'a>,
    ordering: &'static str,
    entries: Vec<Entry>,
    considered: usize,
    skipped: usize,
    truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncation: Option<Truncation>,
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
    let listing = list_tags(
        &request.repository,
        &request.allowed_signers,
        MAX_TAGS_CONSIDERED,
    )
    .ok_or(ApprovalTrailError::Repository)?;

    let deadline = Instant::now() + TOTAL_TIME;
    let mut truncation = listing.truncated.then_some(Truncation::TagLimit);
    let mut skipped = listing.unnamed;
    let mut entries = Vec::new();
    for (name, annotated) in &listing.refs {
        if !annotated {
            continue;
        }
        if Instant::now() >= deadline {
            truncation = Some(Truncation::TimeLimit);
            break;
        }
        let checked = check_tag(
            &request.repository,
            &format!("refs/tags/{name}"),
            &request.allowed_signers,
            identity.as_ref(),
        );
        let Some(trailers) = checked.trailers.as_deref() else {
            skipped += 1;
            continue;
        };
        if !names_record(
            &trailer_values(trailers, RECORD_ID_TRAILER),
            &request.record_id,
        ) {
            continue;
        }
        let claimed_at = checked.tagger_signed_at.clone();
        entries.push(Entry {
            tag: name.clone(),
            claimed_at,
            assessment: assess(checked, &request.record_id, &request.record_digest),
        });
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
        entries,
        considered: listing.considered,
        skipped,
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

/// A cell for people: control characters are replaced, an absent value is `-`.
fn cell(text: Option<&str>) -> String {
    match text {
        Some(text) if !text.is_empty() => text
            .chars()
            .map(|c| if c.is_control() { '?' } else { c })
            .collect(),
        _ => "-".to_owned(),
    }
}

fn table(result: &ResultLine<'_>) -> Result<String, ApprovalTrailError> {
    let mut rows = vec![["CLAIMED_AT", "STATUS", "APPROVER", "MEANING", "TAG"].map(str::to_owned)];
    for entry in &result.entries {
        let value = serde_json::to_value(entry).map_err(|_| ApprovalTrailError::ResultEncoding)?;
        let text = |member: &str| value.get(member).and_then(Value::as_str);
        let meaning = value
            .get("meaning")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("; ")
            });
        rows.push([
            cell(text("claimed_at")),
            cell(text("status")),
            cell(text("approver")),
            cell(meaning.as_deref()),
            cell(text("tag")),
        ]);
    }
    let mut widths = [0_usize; 5];
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
        "considered {}, skipped {}, truncated {}",
        result.considered,
        result.skipped,
        if result.truncated { "yes" } else { "no" }
    );
    Ok(out)
}
