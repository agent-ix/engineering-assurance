// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Host adapter for `proof-checkpoint-verify` (FR-080).
//!
//! The command reads and parses one immutable annotated tag object, compares
//! its id with the caller's pin, and delegates signature classification to the
//! FR-036/FR-040 implementation in `signature_verify_host`.

use std::path::PathBuf;

use engineering_assurance::signature_verify::{
    Mapping, Reason, Status as SignatureStatus, TargetKind, Verdict, classify_tag, trailer_values,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use crate::signature_verify_host::{
    AllowedSignersSource, CaptureFailure, GitsignInput, GitsignRequest, LoadedGitsign, LoadedTag,
    RegistryFailure, capture_allowed_signers, load_gitsign, load_tag_ref, validate_gitsign,
    verify_loaded_tag,
};

const REQUEST_PROTOCOL: &str = "engineering-assurance.proof-checkpoint-verify-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.proof-checkpoint-verify-result/v1";
const PURPOSE: &str = "proof-integrity-statement/v1";
const TRAILER_KEYS: [&str; 7] = [
    "Proof-Checkpoint-Purpose",
    "Proof-Statement-Digest",
    "Proof-Record-Id",
    "Proof-Record-Revision",
    "Proof-Candidate-Revision",
    "Proof-Id",
    "Proof-Attestation-Id",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    repository: PathBuf,
    tag: String,
    expected: Expected,
    allowed_signers: String,
    gitsign: Option<GitsignRequest>,
    identity_registry: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    purpose: String,
    statement_digest: String,
    record_id: String,
    record_revision: u64,
    candidate_revision: String,
    proof_id: String,
    attestation_id: String,
    tag_object_id: String,
}

#[derive(Serialize)]
struct ResultLine {
    protocol: &'static str,
    status: &'static str,
    tag: String,
    tag_object_id: Option<String>,
    checkpoint: Value,
    signer: Option<Value>,
    trust: Value,
    signature: Value,
}

#[derive(Debug, Error)]
pub(crate) enum ProofCheckpointError {
    #[error("proof-checkpoint-verify request is not readable JSON")]
    Unreadable,
    #[error("proof-checkpoint-verify request is not an exact valid request")]
    Invalid,
    #[error("proof-checkpoint-verify gitsign member must not carry a principal")]
    GitsignPrincipal,
    #[error("proof-checkpoint-verify identity registry is not a usable committed registry")]
    IdentityRegistry,
    #[error("proof-checkpoint-verify result cannot be encoded")]
    Encoding,
}

impl ProofCheckpointError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Unreadable => "proof_checkpoint_verify_request_unreadable",
            Self::Invalid => "proof_checkpoint_verify_request_invalid",
            Self::GitsignPrincipal => "proof_checkpoint_verify_gitsign_principal_refused",
            Self::IdentityRegistry => "proof_checkpoint_verify_identity_registry_invalid",
            Self::Encoding => "proof_checkpoint_verify_result_encoding_failed",
        }
    }
}

/// Executes one closed request and returns its single JSON result line.
pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, ProofCheckpointError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| ProofCheckpointError::Unreadable)?;
    let request: Request =
        serde_json::from_value(value).map_err(|_| ProofCheckpointError::Invalid)?;
    validate_request(&request)?;
    let gitsign_input = validate_gitsign(
        request.gitsign.as_ref(),
        request.identity_registry.as_deref(),
    )
    .map_err(|error| {
        error.refusal(
            ProofCheckpointError::Invalid,
            ProofCheckpointError::GitsignPrincipal,
        )
    })?;
    let allowed_source = AllowedSignersSource::parse(&request.allowed_signers)
        .ok_or(ProofCheckpointError::Invalid)?;
    let tag_name = normalized_tag_name(&request.tag).ok_or(ProofCheckpointError::Invalid)?;

    let loaded = match load_tag_ref(
        &request.repository,
        tag_name,
        std::path::Path::new("/dev/null"),
    ) {
        Ok(loaded) => loaded,
        Err(reason) => {
            return encode_result(
                &request,
                None,
                empty_checkpoint("missing"),
                "unverifiable",
                None,
                trust_value(&request, None),
                unverifiable_signature(reason),
            );
        }
    };
    let Some(loaded) = loaded else {
        let verdict =
            classify_tag(&engineering_assurance::signature_verify::TagObservation::Lightweight);
        return encode_result(
            &request,
            None,
            empty_checkpoint("missing"),
            "unsigned",
            None,
            trust_value(&request, None),
            serde_json::to_value(verdict).map_err(|_| ProofCheckpointError::Encoding)?,
        );
    };

    let object_id = loaded.id().to_owned();
    let checkpoint = classify_checkpoint(loaded.trailers(), &request.expected, "match");
    if object_id != request.expected.tag_object_id {
        return encode_result(
            &request,
            Some(object_id),
            checkpoint_with_tag_status(checkpoint, "mismatch"),
            "wrong_checkpoint",
            None,
            trust_value(&request, None),
            json!({"status":"unverifiable","reason":"expected_tag_object_id_mismatch","delegated":false}),
        );
    }

    execute_checked_tag(
        &request,
        loaded,
        object_id,
        checkpoint,
        gitsign_input,
        &allowed_source,
    )
}

fn execute_checked_tag(
    request: &Request,
    loaded: LoadedTag,
    object_id: String,
    checkpoint: Value,
    gitsign_input: Option<GitsignInput>,
    allowed_source: &AllowedSignersSource,
) -> Result<Vec<u8>, ProofCheckpointError> {
    let allowed_signers = match capture_allowed_signers(&request.repository, allowed_source) {
        Ok(snapshot) => snapshot,
        Err(CaptureFailure::Invalid) => return Err(ProofCheckpointError::Invalid),
        Err(CaptureFailure::Git(reason)) => {
            return encode_result(
                request,
                Some(object_id),
                checkpoint,
                "unverifiable",
                None,
                trust_value(request, None),
                unverifiable_signature(reason),
            );
        }
    };
    let gitsign = match gitsign_input
        .map(|input| load_gitsign(&request.repository, input))
        .transpose()
    {
        Ok(gitsign) => gitsign,
        Err(RegistryFailure::Invalid) => return Err(ProofCheckpointError::IdentityRegistry),
        Err(RegistryFailure::Git(reason)) => {
            return encode_result(
                request,
                Some(object_id),
                checkpoint,
                "unverifiable",
                None,
                trust_value(request, None),
                unverifiable_signature(reason),
            );
        }
    };
    let checked = verify_loaded_tag(
        &request.repository,
        &allowed_signers,
        gitsign.as_ref(),
        loaded,
    );
    let signature =
        serde_json::to_value(&checked.verdict).map_err(|_| ProofCheckpointError::Encoding)?;
    let matches = checkpoint_matches(&checkpoint);
    let (status, signer) = match checked.verdict.status {
        SignatureStatus::SignedValid => {
            let mapped = checked
                .verdict
                .signer
                .as_ref()
                .is_some_and(|signer| signer.mapping == Mapping::Mapped);
            if !mapped {
                ("untrusted_signer", None)
            } else if matches {
                (
                    "verified",
                    checked
                        .verdict
                        .signer
                        .as_ref()
                        .map(|_| json!(checked.verdict.signer.clone())),
                )
            } else {
                ("wrong_checkpoint", None)
            }
        }
        SignatureStatus::SignedUntrusted => ("untrusted_signer", None),
        SignatureStatus::SignedBad => ("signed_bad", None),
        SignatureStatus::Unsigned => ("unsigned", None),
        SignatureStatus::Unverifiable => ("unverifiable", None),
    };
    encode_result(
        request,
        Some(object_id),
        checkpoint,
        status,
        signer,
        trust_value(request, gitsign.as_ref()),
        signature,
    )
}

fn validate_request(request: &Request) -> Result<(), ProofCheckpointError> {
    let expected = &request.expected;
    if request.protocol != REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || normalized_tag_name(&request.tag).is_none()
        || expected.purpose != PURPOSE
        || !is_digest(&expected.statement_digest)
        || !is_identifier(&expected.record_id)
        || expected.record_revision == 0
        || !is_identifier(&expected.candidate_revision)
        || !is_identifier(&expected.proof_id)
        || !is_identifier(&expected.attestation_id)
        || !is_object_id(&expected.tag_object_id)
    {
        return Err(ProofCheckpointError::Invalid);
    }
    Ok(())
}

fn normalized_tag_name(tag: &str) -> Option<&str> {
    let name = tag.strip_prefix("refs/tags/").unwrap_or(tag);
    if name.is_empty()
        || name.starts_with('-')
        || name.starts_with('/')
        || name.ends_with('/')
        || name.ends_with('.')
        || name.contains("..")
        || name.contains("//")
        || name.contains("@{")
        || name.bytes().any(|byte| {
            byte <= b' '
                || byte == b'~'
                || byte == b'^'
                || byte == b':'
                || byte == b'?'
                || byte == b'*'
                || byte == b'['
                || byte == b'\\'
        })
    {
        return None;
    }
    Some(name)
}

fn is_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn is_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_control)
}

fn classify_checkpoint(parsed: Option<&str>, expected: &Expected, tag_status: &str) -> Value {
    let revision = expected.record_revision.to_string();
    let fields = [
        ("purpose", TRAILER_KEYS[0], expected.purpose.as_str()),
        (
            "statement_digest",
            TRAILER_KEYS[1],
            expected.statement_digest.as_str(),
        ),
        ("record_id", TRAILER_KEYS[2], expected.record_id.as_str()),
        ("record_revision", TRAILER_KEYS[3], revision.as_str()),
        (
            "candidate_revision",
            TRAILER_KEYS[4],
            expected.candidate_revision.as_str(),
        ),
        ("proof_id", TRAILER_KEYS[5], expected.proof_id.as_str()),
        (
            "attestation_id",
            TRAILER_KEYS[6],
            expected.attestation_id.as_str(),
        ),
    ];
    let mut result = serde_json::Map::new();
    result.insert("tag_object_id".to_owned(), json!(tag_status));
    for (name, key, expected_value) in fields {
        let observed_values = parsed.map(|text| trailer_values(text, key));
        let classification = match observed_values.as_deref() {
            Some([value]) if value == expected_value => "match",
            Some([_]) => "mismatch",
            Some([]) | None => "missing",
            Some(_) => "repeated",
        };
        let observed = observed_values.map(|values| match values.as_slice() {
            [] => Value::Null,
            [value] => json!(value),
            _ => json!(values),
        });
        result.insert(
            name.to_owned(),
            json!({"value":observed,"status":classification}),
        );
    }
    Value::Object(result)
}

fn checkpoint_matches(checkpoint: &Value) -> bool {
    checkpoint.as_object().is_some_and(|fields| {
        fields
            .iter()
            .all(|(name, value)| name == "tag_object_id" || value["status"] == "match")
    })
}

fn checkpoint_with_tag_status(mut checkpoint: Value, status: &str) -> Value {
    checkpoint["tag_object_id"] = json!(status);
    checkpoint
}

fn empty_checkpoint(tag_status: &str) -> Value {
    let mut result = serde_json::Map::new();
    result.insert("tag_object_id".to_owned(), json!(tag_status));
    for name in [
        "purpose",
        "statement_digest",
        "record_id",
        "record_revision",
        "candidate_revision",
        "proof_id",
        "attestation_id",
    ] {
        result.insert(name.to_owned(), json!({"value":null,"status":"missing"}));
    }
    Value::Object(result)
}

fn trust_value(request: &Request, gitsign: Option<&LoadedGitsign>) -> Value {
    let mut trust = serde_json::Map::new();
    trust.insert("allowed_signers".to_owned(), json!(request.allowed_signers));
    if let Some(gitsign_request) = &request.gitsign {
        trust.insert(
            "gitsign".to_owned(),
            json!({
                "certificate_identity":gitsign_request.certificate_identity,
                "certificate_oidc_issuer":gitsign_request.certificate_oidc_issuer,
            }),
        );
        if let Some(registry) = &request.identity_registry {
            trust.insert("identity_registry".to_owned(), json!(registry));
        }
        if let Some(gitsign) = gitsign {
            trust.insert(
                "identity_registry_commit".to_owned(),
                json!(gitsign.commit()),
            );
        }
    }
    Value::Object(trust)
}

fn unverifiable_signature(reason: Reason) -> Value {
    serde_json::to_value(Verdict::unverifiable(TargetKind::Tag, reason))
        .unwrap_or_else(|_| json!({"status":"unverifiable","kind":"tag"}))
}

fn encode_result(
    request: &Request,
    tag_object_id: Option<String>,
    checkpoint: Value,
    status: &'static str,
    signer: Option<Value>,
    trust: Value,
    signature: Value,
) -> Result<Vec<u8>, ProofCheckpointError> {
    let mut line = serde_json::to_vec(&ResultLine {
        protocol: RESULT_PROTOCOL,
        status,
        tag: request.tag.clone(),
        tag_object_id,
        checkpoint,
        signer,
        trust,
        signature,
    })
    .map_err(|_| ProofCheckpointError::Encoding)?;
    line.push(b'\n');
    Ok(line)
}
