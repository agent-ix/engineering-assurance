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
use engineering_assurance::signature_verify::{Verdict, is_plain_revision, trailer_values};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::signature_verify_host::{
    AllowedSignersSnapshot, CheckedTag, GitsignInput, GitsignInputError, GitsignRequest,
    LoadedGitsign, RegistryFailure, capture_allowed_signers, check_tag, load_gitsign,
    validate_gitsign,
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
    allowed_signers: PathBuf,
    gitsign: Option<GitsignRequest>,
    identity_registry: Option<String>,
}

/// The trust input a verdict was made against, copied from the request, with
/// the commit the identity registry revision resolved to.
#[derive(Serialize)]
pub(crate) struct Trust<'a> {
    pub(crate) allowed_signers: &'a Path,
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

impl<'a> Trust<'a> {
    /// The trust of a run that read `loaded`, or none when the revision did not
    /// resolve or no registry was named.
    pub(crate) fn new(
        allowed_signers: &'a Path,
        echo: GitsignEcho<'a>,
        loaded: Option<&'a LoadedGitsign>,
    ) -> Self {
        Self {
            allowed_signers,
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

/// The FR-041 result for one tag.
#[derive(Serialize)]
pub(crate) struct ResultLine<'a> {
    protocol: &'static str,
    tag: &'a str,
    record_id: &'a str,
    record_digest: &'a str,
    trust: Trust<'a>,
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
        request_allowed_signers: &'a Path,
        verification_allowed_signers: &AllowedSignersSnapshot,
        echo: GitsignEcho<'a>,
        loaded: Option<&'a LoadedGitsign>,
    ) -> Self {
        let checked = check_tag(repository, tag, verification_allowed_signers, loaded);
        Self::from_checked(
            checked,
            tag,
            record,
            Trust::new(request_allowed_signers, echo, loaded),
        )
    }

    /// The result for a tag that was checked, or never read.
    pub(crate) fn from_checked(
        checked: CheckedTag,
        tag: &'a str,
        record: (&'a str, &'a str),
        trust: Trust<'a>,
    ) -> Self {
        let (record_id, record_digest) = record;
        Self {
            protocol: RESULT_PROTOCOL,
            tag: tag.strip_prefix("refs/tags/").unwrap_or(tag),
            record_id,
            record_digest,
            trust,
            tag_object_id: checked.tag_object_id.clone(),
            assessment: assess(checked, record_id, record_digest),
        }
    }

    /// The decision part of the result.
    pub(crate) const fn assessment(&self) -> &Assessment {
        &self.assessment
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

/// The refusal for a `gitsign` and `identity_registry` request error.
pub(crate) const fn gitsign_error(error: GitsignInputError) -> ApprovalVerifyError {
    match error {
        GitsignInputError::Invalid => ApprovalVerifyError::Invalid,
        GitsignInputError::PrincipalRefused => ApprovalVerifyError::GitsignPrincipal,
    }
}

/// What reading the identity registry of a request yielded.
enum Registry {
    /// The registry was read, or the request names none.
    Loaded(Option<LoadedGitsign>),
    /// Git could not answer while the registry was read.
    GitFailed(engineering_assurance::signature_verify::Reason),
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
    let input = validate_gitsign(
        request.gitsign.as_ref(),
        request.identity_registry.as_deref(),
    )
    .map_err(gitsign_error)?;
    let allowed_signers = capture_allowed_signers(&request.allowed_signers)
        .map_err(|_| ApprovalVerifyError::AllowedSigners)?;
    let registry = load_registry(&request.repository, input)?;
    let echo = GitsignEcho::new(
        request.gitsign.as_ref(),
        request.identity_registry.as_deref(),
    );
    let record = (request.record_id.as_str(), request.record_digest.as_str());
    let result = match &registry {
        Registry::Loaded(loaded) => ResultLine::verify(
            &request.repository,
            &request.tag,
            record,
            &request.allowed_signers,
            &allowed_signers,
            echo,
            loaded.as_ref(),
        ),
        // Git could not answer while the registry was read: no verifier starts.
        Registry::GitFailed(reason) => ResultLine::from_checked(
            CheckedTag::unread(*reason),
            &request.tag,
            record,
            Trust::new(&request.allowed_signers, echo, None),
        ),
    };
    let mut line = serde_json::to_vec(&result).map_err(|_| ApprovalVerifyError::ResultEncoding)?;
    line.push(b'\n');
    Ok(line)
}
