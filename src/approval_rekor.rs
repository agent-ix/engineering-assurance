// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Extracts and verifies an embedded Rekor entry from a CMS tag signature
//! (FR-041).
//!
//! This module only decodes the CMS and Rekor containers. Trust decisions,
//! artifact hashing, signature checks, inclusion checks and certificate
//! validity are delegated to `sigstore-verify`.

use base64::{Engine, engine::general_purpose::STANDARD};
use cms::{
    content_info::ContentInfo,
    signed_data::{SignedData, SignerIdentifier},
};
use der::{Decode, Encode, asn1::OctetString};
use prost::Message;
use serde_json::json;
use sigstore_types::Bundle;

const CMS_BEGIN: &str = "-----BEGIN SIGNED MESSAGE-----";
const CMS_END: &str = "-----END SIGNED MESSAGE-----";
const REKOR_ENTRY_OID: &str = "1.3.6.1.4.1.57264.3.1";

/// The result of reading the embedded entry in a CMS signature block.
pub(crate) enum EmbeddedEntry {
    /// The CMS block is readable and carries no Rekor entry attribute.
    Missing,
    /// The entry, certificate or bundle inputs could not be extracted.
    Invalid,
    /// A Sigstore bundle built from the entry's exact fields.
    Bundle { artifact: Vec<u8>, bundle: Bundle },
}

/// Extract the signed-attributes DER, signer certificate, signature, and
/// optional Rekor entry from an armored CMS block.
pub(crate) fn extract(tag_object: &[u8]) -> EmbeddedEntry {
    let Some(split) = engineering_assurance::signature_verify::split_tag_object(tag_object) else {
        return EmbeddedEntry::Invalid;
    };
    if split.format != engineering_assurance::signature_verify::SignatureFormat::Cms {
        return EmbeddedEntry::Invalid;
    }
    let Some(cms_der) = decode_armor(split.signature) else {
        return EmbeddedEntry::Invalid;
    };
    let Ok(content_info) = ContentInfo::from_der(&cms_der) else {
        return EmbeddedEntry::Invalid;
    };
    if content_info.content_type.to_string() != "1.2.840.113549.1.7.2" {
        return EmbeddedEntry::Invalid;
    }
    let Ok(signed_data) = content_info.content.decode_as::<SignedData>() else {
        return EmbeddedEntry::Invalid;
    };
    let [signer] = signed_data.signer_infos.0.as_slice() else {
        return EmbeddedEntry::Invalid;
    };
    let Some(signed_attrs) = signer.signed_attrs.as_ref() else {
        return EmbeddedEntry::Invalid;
    };

    let entries: Vec<_> = signer
        .unsigned_attrs
        .iter()
        .flat_map(|attrs| attrs.iter())
        .filter(|attr| attr.oid.to_string() == REKOR_ENTRY_OID)
        .collect();
    let entry_attr = match entries.as_slice() {
        [] => return EmbeddedEntry::Missing,
        [entry] => *entry,
        _ => return EmbeddedEntry::Invalid,
    };
    let [entry_value] = entry_attr.values.as_slice() else {
        return EmbeddedEntry::Invalid;
    };
    let Ok(entry_bytes) = entry_value.clone().decode_as::<OctetString>() else {
        return EmbeddedEntry::Invalid;
    };
    let Ok(entry) = RekorTransparencyLogEntry::decode(entry_bytes.as_bytes()) else {
        return EmbeddedEntry::Invalid;
    };

    let Some(certificates) = signed_data.certificates.as_ref() else {
        return EmbeddedEntry::Invalid;
    };
    let signer_certificate = certificates.0.iter().find_map(|choice| {
        let cms::cert::CertificateChoices::Certificate(certificate) = choice else {
            return None;
        };
        let SignerIdentifier::IssuerAndSerialNumber(signer_id) = &signer.sid else {
            return None;
        };
        (certificate.tbs_certificate.issuer == signer_id.issuer
            && certificate.tbs_certificate.serial_number == signer_id.serial_number)
            .then(|| certificate.to_der().ok())
            .flatten()
    });
    let Some(signer_certificate) = signer_certificate else {
        return EmbeddedEntry::Invalid;
    };
    let Ok(artifact) = signed_attrs.to_der() else {
        return EmbeddedEntry::Invalid;
    };
    let signature = bundle_signature(&signer.signature);
    let Some(bundle) = bundle(&entry, &signer_certificate, &signature) else {
        return EmbeddedEntry::Invalid;
    };

    EmbeddedEntry::Bundle { artifact, bundle }
}

/// Verify the extracted entry and return only its library-authenticated
/// integrated time.
pub(crate) fn verify(bundle: &Bundle, artifact: &[u8], trusted_root_json: &str) -> Option<i64> {
    let trusted_root =
        sigstore_verify::trust_root::TrustedRoot::from_json(trusted_root_json).ok()?;
    let verifier = sigstore_verify::Verifier::new(&trusted_root).ok()?;
    let result = verifier
        .verify(
            artifact,
            bundle,
            &sigstore_verify::VerificationPolicy::any_identity(),
        )
        .ok()?;
    result.integrated_time().map(|time| time.as_second())
}

fn decode_armor(signature: &[u8]) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(signature).ok()?;
    let mut lines = text.lines();
    if lines.next()? != CMS_BEGIN {
        return None;
    }
    let mut encoded = String::new();
    let mut found_end = false;
    while let Some(line) = lines.next() {
        if line == CMS_END {
            found_end = true;
            break;
        }
        if line.is_empty()
            || !line
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
        {
            return None;
        }
        encoded.push_str(line);
    }
    if !found_end || lines.any(|line| !line.is_empty()) {
        return None;
    }
    STANDARD.decode(encoded).ok()
}

fn bundle(
    entry: &RekorTransparencyLogEntry,
    certificate: &[u8],
    signature: &str,
) -> Option<Bundle> {
    let log_id = entry.log_id.as_ref()?;
    let kind_version = entry.kind_version.as_ref()?;
    let promise = entry.inclusion_promise.as_ref()?;
    let proof = entry.inclusion_proof.as_ref()?;
    let canonicalized_body = entry.canonicalized_body.as_ref()?;
    if entry.integrated_time <= 0
        || entry.log_index < 0
        || proof.log_index < 0
        || proof.tree_size <= 0
    {
        return None;
    }
    let checkpoint = std::str::from_utf8(&proof.checkpoint).ok()?;
    let json_bundle = json!({
        "mediaType": "application/vnd.dev.sigstore.bundle.v0.3+json",
        "verificationMaterial": {
            "certificate": { "rawBytes": STANDARD.encode(certificate) },
            "tlogEntries": [{
                "logIndex": entry.log_index.to_string(),
                "logId": { "keyId": STANDARD.encode(&log_id.key_id) },
                "kindVersion": { "kind": kind_version.kind, "version": kind_version.version },
                "integratedTime": entry.integrated_time.to_string(),
                "inclusionPromise": { "signedEntryTimestamp": STANDARD.encode(&promise.signed_entry_timestamp) },
                "inclusionProof": {
                    "logIndex": proof.log_index.to_string(),
                    "rootHash": STANDARD.encode(&proof.root_hash),
                    "treeSize": proof.tree_size.to_string(),
                    "hashes": proof.hashes.iter().map(|hash| STANDARD.encode(hash)).collect::<Vec<_>>(),
                    "checkpoint": { "envelope": checkpoint },
                },
                "canonicalizedBody": STANDARD.encode(canonicalized_body),
            }]
        },
        "messageSignature": { "signature": signature }
    });
    Bundle::from_json(&json_bundle.to_string()).ok()
}

fn bundle_signature(signature: &OctetString) -> String {
    STANDARD.encode(signature.as_bytes())
}

#[derive(Clone, PartialEq, Message)]
struct RekorTransparencyLogEntry {
    #[prost(int64, tag = "1")]
    log_index: i64,
    #[prost(message, optional, tag = "2")]
    log_id: Option<RekorLogId>,
    #[prost(message, optional, tag = "3")]
    kind_version: Option<RekorKindVersion>,
    #[prost(int64, tag = "4")]
    integrated_time: i64,
    #[prost(message, optional, tag = "5")]
    inclusion_promise: Option<RekorInclusionPromise>,
    #[prost(message, optional, tag = "6")]
    inclusion_proof: Option<RekorInclusionProof>,
    #[prost(bytes = "vec", optional, tag = "7")]
    canonicalized_body: Option<Vec<u8>>,
}

#[derive(Clone, PartialEq, Message)]
struct RekorLogId {
    #[prost(bytes = "vec", tag = "1")]
    key_id: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
struct RekorKindVersion {
    #[prost(string, tag = "1")]
    kind: String,
    #[prost(string, tag = "2")]
    version: String,
}

#[derive(Clone, PartialEq, Message)]
struct RekorInclusionPromise {
    #[prost(bytes = "vec", tag = "1")]
    signed_entry_timestamp: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
struct RekorInclusionProof {
    #[prost(int64, tag = "1")]
    log_index: i64,
    #[prost(bytes = "vec", tag = "2")]
    root_hash: Vec<u8>,
    #[prost(int64, tag = "3")]
    tree_size: i64,
    #[prost(bytes = "vec", repeated, tag = "4")]
    hashes: Vec<Vec<u8>>,
    #[prost(bytes = "vec", tag = "5")]
    checkpoint: Vec<u8>,
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::bundle_signature;
    use base64::{Engine, engine::general_purpose::STANDARD};
    use der::asn1::OctetString;

    #[test]
    #[ix_trace_rs::trace("FR-041-AC-28")]
    fn cms_octet_string_contents_are_not_der_wrapped_in_the_bundle() {
        // These opaque bytes deliberately resemble a DER sequence so wrapping
        // the CMS OCTET STRING produces a distinguishable value.
        let raw_octets = [0x30, 0x00];
        let cms_signature = OctetString::new(raw_octets.to_vec())
            .expect("opaque octets are valid CMS OCTET STRING contents");
        let encoded = bundle_signature(&cms_signature);

        assert_eq!(encoded, STANDARD.encode(raw_octets));
        assert_ne!(encoded, STANDARD.encode([0x04, 0x02, 0x30, 0x00]));
    }
}
