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
    Approval, ApprovalStatus, Bindings, RECORD_DIGEST_TRAILER, RECORD_ID_TRAILER,
    UnverifiableReason, bind, decide, is_record_value,
};
use engineering_assurance::signature_verify::{
    GitsignIdentity, Verdict, is_plain_revision, trailer_values,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::signature_verify_host::{CheckedTag, GitsignRequest, check_allowed_signers, check_tag};

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
    allowed_signers: PathBuf,
    gitsign: Option<GitsignRequest>,
}

/// The trust input a verdict was made against, copied from the request.
#[derive(Serialize)]
pub(crate) struct Trust<'a> {
    pub(crate) allowed_signers: &'a Path,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) gitsign: Option<&'a GitsignRequest>,
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
}

/// Decides the FR-041 result for a tag already verified by `check_tag`.
pub(crate) fn assess(checked: CheckedTag, record_id: &str, record_digest: &str) -> Assessment {
    let bindings = checked.trailers.as_deref().map(|trailers| Bindings {
        record_id: bind(&trailer_values(trailers, RECORD_ID_TRAILER), record_id),
        record_digest: bind(
            &trailer_values(trailers, RECORD_DIGEST_TRAILER),
            record_digest,
        ),
    });
    let decision = decide(&checked.verdict, bindings, checked.tagger_signed_at);
    Assessment {
        status: decision.status,
        reason: decision.reason,
        signature: checked.verdict,
        approval: decision.approval,
        binding: bindings,
    }
}

#[derive(Serialize)]
struct ResultLine<'a> {
    protocol: &'static str,
    tag: &'a str,
    record_id: &'a str,
    record_digest: &'a str,
    trust: Trust<'a>,
    #[serde(flatten)]
    assessment: Assessment,
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
            Self::ResultEncoding => "approval_verify_result_encoding_failed",
        }
    }
}

/// Runs one request document and returns the result line.
pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, ApprovalVerifyError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| ApprovalVerifyError::Unreadable)?;
    let request: Request =
        serde_json::from_value(value).map_err(|_| ApprovalVerifyError::Invalid)?;
    if request.protocol != REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !request.allowed_signers.is_absolute()
        || !is_plain_revision(&request.tag)
        || !is_record_value(&request.record_id)
        || !is_record_value(&request.record_digest)
    {
        return Err(ApprovalVerifyError::Invalid);
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
            .ok_or(ApprovalVerifyError::Invalid)
        })
        .transpose()?;
    check_allowed_signers(&request.allowed_signers)
        .map_err(|_| ApprovalVerifyError::AllowedSigners)?;
    let checked = check_tag(
        &request.repository,
        &request.tag,
        &request.allowed_signers,
        identity.as_ref(),
    );
    let assessment = assess(checked, &request.record_id, &request.record_digest);
    let tag = request
        .tag
        .strip_prefix("refs/tags/")
        .unwrap_or(&request.tag);
    let mut line = serde_json::to_vec(&ResultLine {
        protocol: RESULT_PROTOCOL,
        tag,
        record_id: &request.record_id,
        record_digest: &request.record_digest,
        trust: Trust {
            allowed_signers: &request.allowed_signers,
            gitsign: request.gitsign.as_ref(),
        },
        assessment,
    })
    .map_err(|_| ApprovalVerifyError::ResultEncoding)?;
    line.push(b'\n');
    Ok(line)
}
