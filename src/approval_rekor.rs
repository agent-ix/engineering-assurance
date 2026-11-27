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
use ix_trace_rs::implements;
use prost::Message;
use serde_json::json;
use sigstore_types::Bundle;

const CMS_BEGIN: &str = "-----BEGIN SIGNED MESSAGE-----";
const CMS_END: &str = "-----END SIGNED MESSAGE-----";
const REKOR_ENTRY_OID: &str = "1.3.6.1.4.1.57264.3.1";
const DER_SET_OF_TAG: u8 = 0x31;

/// The result of reading the embedded entry in a CMS signature block.
pub(crate) enum EmbeddedEntry {
    /// The CMS block is readable and carries no Rekor entry attribute.
    Missing,
    /// The entry, certificate or bundle inputs could not be extracted.
    Invalid,
    /// A Sigstore bundle built from the entry's exact fields.
    Bundle {
        artifact: Vec<u8>,
        bundle: Box<Bundle>,
    },
}

/// Extract the signed-attributes DER, signer certificate, signature, and
/// optional Rekor entry from an armored CMS block.
#[implements("FR-041-AC-28")]
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
    // SignedAttributes is a SET OF. Its standalone DER is the artifact
    // sigstore-verify hashes; the CMS field's implicit context tag is not.
    let Ok(artifact) = signed_attrs.to_der() else {
        return EmbeddedEntry::Invalid;
    };
    if artifact.first() != Some(&DER_SET_OF_TAG) {
        return EmbeddedEntry::Invalid;
    }
    let signature = bundle_signature(&signer.signature);
    let Some(bundle) = bundle(&entry, &signer_certificate, &signature) else {
        return EmbeddedEntry::Invalid;
    };

    EmbeddedEntry::Bundle {
        artifact,
        bundle: Box::new(bundle),
    }
}

/// Verify the extracted entry and return only its library-authenticated
/// integrated time.
#[implements("FR-041-AC-29")]
pub(crate) fn verify(bundle: &Bundle, artifact: &[u8], trusted_root_json: &str) -> Option<i64> {
    verify_with_policy(
        bundle,
        artifact,
        trusted_root_json,
        &sigstore_verify::VerificationPolicy::any_identity(),
    )
}

fn verify_with_policy(
    bundle: &Bundle,
    artifact: &[u8],
    trusted_root_json: &str,
    policy: &sigstore_verify::VerificationPolicy,
) -> Option<i64> {
    let trusted_root =
        sigstore_verify::trust_root::TrustedRoot::from_json(trusted_root_json).ok()?;
    let verifier = sigstore_verify::Verifier::new(&trusted_root).ok()?;
    let result = verifier.verify(artifact, bundle, policy).ok()?;
    result.integrated_time().map(jiff::Timestamp::as_second)
}

fn decode_armor(signature: &[u8]) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(signature).ok()?;
    let mut lines = text.lines();
    if lines.next()? != CMS_BEGIN {
        return None;
    }
    let mut encoded = String::new();
    let mut found_end = false;
    for line in lines.by_ref() {
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
    use super::{DER_SET_OF_TAG, EmbeddedEntry, bundle_signature, extract, verify_with_policy};
    use base64::{Engine, engine::general_purpose::STANDARD};
    use der::asn1::OctetString;
    use ix_trace_rs::trace;
    use serde_json::json;
    use sigstore_types::Bundle;

    const VERIFIED_TAG: &[u8] = include_bytes!("../tests/fixtures/approval-rekor/tag-object.txt");
    const SECOND_VERIFIED_TAG: &[u8] =
        include_bytes!("../tests/fixtures/approval-rekor/tag-object-second.txt");
    const FIXTURE_ROOT_CERTIFICATE: &str =
        include_str!("../tests/fixtures/approval-rekor/ca-certificate.txt");

    fn fixture_trusted_root() -> String {
        let root_certificate = sigstore_types::DerCertificate::from_pem(FIXTURE_ROOT_CERTIFICATE)
            .expect("fixture root certificate is PEM text");
        json!({
            "mediaType": "application/vnd.dev.sigstore.trustedroot+json;version=0.1",
            "tlogs": [{
                "baseUrl": "urn:ea-306-offline-fixture",
                "hashAlgorithm": "SHA2_256",
                "publicKey": {
                    "rawBytes": "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE0OOrukb0TlXmQ+VGSmn/KIiD7F5VEQ7c6yO9wh1EkRJxSUD14B4SQDrLcqeNflP7M1DMjxK9DDiNOij+M9zX6A==",
                    "keyDetails": "PKIX_ECDSA_P256_SHA_256",
                    "validFor": { "start": "2020-01-01T00:00:00Z", "end": "2099-01-01T00:00:00Z" }
                },
                "logId": { "keyId": "72outTSDeRP7OxwD1u4X5TUijrbl/GrQCMu04tF5vr8=" }
            }, {
                "baseUrl": "urn:ea-306-offline-fixture-two",
                "hashAlgorithm": "SHA2_256",
                "publicKey": {
                    "rawBytes": "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEFEKTsPqQ1yX7t+HtszWIVrGqpEU2kcO4Go5C1evhijExaVQO1jy8BDqXSjuLTTCzuiWNKl9J0FO/IV5tASqbsQ==",
                    "keyDetails": "PKIX_ECDSA_P256_SHA_256",
                    "validFor": { "start": "2020-01-01T00:00:00Z", "end": "2099-01-01T00:00:00Z" }
                },
                "logId": { "keyId": "3pYjQzKKsAXQI0v795v01hYeWIDo6qDA7CBVrC77FPA=" }
            }],
            "certificateAuthorities": [{
                "subject": { "commonName": "EA Offline Fixture Root" },
                "uri": "urn:ea-306-offline-fixture",
                "certChain": { "certificates": [{ "rawBytes": STANDARD.encode(root_certificate.as_bytes()) }] },
                "validFor": { "start": "2020-01-01T00:00:00Z", "end": "2099-01-01T00:00:00Z" }
            }],
            "ctlogs": [],
            "timestampAuthorities": []
        })
        .to_string()
    }

    fn bundle_json(bundle: &Bundle) -> serde_json::Value {
        serde_json::from_str(&bundle.to_json().expect("fixture bundle serializes"))
            .expect("fixture bundle serializes as JSON")
    }

    fn mutate_bundle(bundle: &Bundle, mutate: impl FnOnce(&mut serde_json::Value)) -> Bundle {
        let mut value = bundle_json(bundle);
        mutate(&mut value);
        Bundle::from_json(&value.to_string()).expect("mutated fixture bundle remains parseable")
    }

    fn verify_fixture(bundle: &Bundle, artifact: &[u8]) -> Result<Option<i64>, String> {
        let policy = sigstore_verify::VerificationPolicy::any_identity().skip_sct();
        let trusted_root =
            sigstore_verify::trust_root::TrustedRoot::from_json(&fixture_trusted_root())
                .map_err(|error| error.to_string())?;
        let verifier =
            sigstore_verify::Verifier::new(&trusted_root).map_err(|error| error.to_string())?;
        let result = verifier
            .verify(artifact, bundle, &policy)
            .map_err(|error| error.to_string())?;
        Ok(result.integrated_time().map(jiff::Timestamp::as_second))
    }

    fn assert_rejected(result: &Result<Option<i64>, String>) {
        assert!(
            !matches!(result, Ok(Some(_))),
            "tampered verifier input was accepted: {result:?}"
        );
    }

    #[test]
    #[trace("FR-041-AC-28")]
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

    #[test]
    #[trace("FR-041-AC-28")]
    fn cms_rekor_fixture_extracts_verified_integrated_time_offline() {
        let EmbeddedEntry::Bundle { artifact, bundle } = extract(VERIFIED_TAG) else {
            panic!("fixture must yield a bundle from CMS extraction");
        };
        assert_eq!(
            artifact.first(),
            Some(&DER_SET_OF_TAG),
            "the artifact is the universal DER SET OF signed attributes"
        );

        assert_eq!(
            verify_fixture(&bundle, &artifact).unwrap_or_else(|error| panic!("{error}")),
            Some(1_767_225_600),
            "successful SET, inclusion, checkpoint, artifact and signature verification yields integratedTime"
        );
    }

    #[test]
    #[trace("FR-041-AC-29")]
    fn cms_rekor_fixture_rejects_integrated_time_mutation() {
        let EmbeddedEntry::Bundle { artifact, bundle } = extract(VERIFIED_TAG) else {
            panic!("fixture must yield a bundle from CMS extraction");
        };
        let changed = mutate_bundle(&bundle, |value| {
            value["verificationMaterial"]["tlogEntries"][0]["integratedTime"] = json!("1767225601");
        });

        assert_rejected(&verify_fixture(&changed, &artifact));
    }

    #[test]
    #[trace("FR-041-AC-29")]
    fn cms_rekor_fixture_rejects_a_tampered_set() {
        let EmbeddedEntry::Bundle { artifact, bundle } = extract(VERIFIED_TAG) else {
            panic!("fixture must yield a bundle from CMS extraction");
        };
        let changed = mutate_bundle(&bundle, |value| {
            let encoded = value["verificationMaterial"]["tlogEntries"][0]["inclusionPromise"]
                ["signedEntryTimestamp"]
                .as_str()
                .expect("fixture SET is base64");
            let mut signature = STANDARD
                .decode(encoded)
                .expect("fixture SET is valid base64");
            if let Some(first) = signature.first_mut() {
                *first ^= 1;
            }
            value["verificationMaterial"]["tlogEntries"][0]["inclusionPromise"]["signedEntryTimestamp"] =
                json!(STANDARD.encode(signature));
        });

        assert_rejected(&verify_fixture(&changed, &artifact));
    }

    #[test]
    #[trace("FR-041-AC-29")]
    fn cms_rekor_fixture_rejects_a_tampered_inclusion_proof() {
        let EmbeddedEntry::Bundle { artifact, bundle } = extract(VERIFIED_TAG) else {
            panic!("fixture must yield a bundle from CMS extraction");
        };
        let changed = mutate_bundle(&bundle, |value| {
            value["verificationMaterial"]["tlogEntries"][0]["inclusionProof"]["rootHash"] =
                json!(STANDARD.encode([0_u8; 32]));
        });

        assert_rejected(&verify_fixture(&changed, &artifact));
    }

    #[test]
    #[trace("FR-041-AC-29")]
    fn cms_rekor_fixture_rejects_an_entry_from_a_distinct_valid_tag() {
        let EmbeddedEntry::Bundle {
            artifact: first_artifact,
            bundle: first_bundle,
        } = extract(VERIFIED_TAG)
        else {
            panic!("first fixture must yield a bundle from CMS extraction");
        };
        let EmbeddedEntry::Bundle {
            artifact: second_artifact,
            bundle: second_bundle,
        } = extract(SECOND_VERIFIED_TAG)
        else {
            panic!("second fixture must yield a bundle from CMS extraction");
        };
        assert_eq!(
            verify_fixture(&second_bundle, &second_artifact)
                .unwrap_or_else(|error| panic!("{error}")),
            Some(1_767_225_600)
        );

        let second_entry =
            bundle_json(&second_bundle)["verificationMaterial"]["tlogEntries"][0].clone();
        let substituted = mutate_bundle(&first_bundle, |value| {
            value["verificationMaterial"]["tlogEntries"][0] = second_entry;
        });

        assert_rejected(&verify_fixture(&substituted, &first_artifact));
    }

    #[test]
    #[trace("FR-041-AC-29")]
    fn cms_rekor_fixture_rejects_an_unrelated_trusted_root() {
        let EmbeddedEntry::Bundle { artifact, bundle } = extract(VERIFIED_TAG) else {
            panic!("fixture must yield a bundle from CMS extraction");
        };
        let mut unrelated_root: serde_json::Value =
            serde_json::from_str(&fixture_trusted_root()).expect("fixture trusted root is JSON");
        unrelated_root["tlogs"][0]["logId"]["keyId"] = json!(STANDARD.encode([0_u8; 32]));
        let policy = sigstore_verify::VerificationPolicy::any_identity().skip_sct();

        assert_eq!(
            verify_with_policy(&bundle, &artifact, &unrelated_root.to_string(), &policy),
            None
        );
    }
}
