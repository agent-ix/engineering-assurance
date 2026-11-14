// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Sigstore-backed in-toto statement signing and verification (EA-106,
//! FR-058, FR-064, FR-068, FR-078, NFR-010, NFR-011).
//!
//! Statements are ordinary serde values. All signature operations are
//! delegated to sigstore-rust; this module only canonicalizes, selects the
//! exact workflow identity from trusted policy, and compares string subjects.
//!
//! Sigstore 0.16 verifies the DSSE signature and in-toto payload bytes; it does
//! not interpret Engineering Assurance's `gitCommit` or `gitTree` subject
//! members. After Sigstore accepts the bundle and certificate chain, this
//! module compares the requested subject member as an exact string. It does
//! not hash a git object or infer one from the statement's repository name.
//! Transparency-log inclusion is required by default policy configuration;
//! timestamp-only verification is selected only for a trusted instance policy
//! whose signing service does not operate a transparency log.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sigstore_sign::{SigningContext, SigstoreInstance};
use sigstore_verify::VerificationPolicy;

use crate::record_sink::leak::PublicPathProof;

/// The in-toto Statement v1 type URI.
pub const STATEMENT_V1_TYPE: &str = concat!("https:", "//in-toto.io/Statement/v1");

/// A plain serde representation of an in-toto Statement v1.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatementV1 {
    /// The in-toto statement type.
    #[serde(rename = "_type")]
    pub type_: String,
    /// The artifacts or git objects described by the statement.
    pub subject: Vec<SubjectV1>,
    /// The predicate type URI.
    #[serde(rename = "predicateType")]
    pub predicate_type: String,
    /// The predicate object.
    pub predicate: BTreeMap<String, Value>,
}

impl StatementV1 {
    /// Serialize this statement using the repository's RFC 8785 canonicalizer.
    ///
    /// # Errors
    ///
    /// Returns an error if the statement cannot be serialized canonically.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, StatementEncodingError> {
        serde_json_canonicalizer::to_vec(self).map_err(|_| StatementEncodingError)
    }
}

/// One in-toto statement subject.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectV1 {
    /// The subject's stable name.
    pub name: String,
    /// Digest algorithm names and their string values.
    pub digest: BTreeMap<String, String>,
}

/// A statement subject selected for comparison with a caller's git object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitSubjectKind {
    /// Compare the `gitCommit` string.
    Commit,
    /// Compare the `gitTree` string.
    Tree,
}

/// The expected signer identity from protected workflow policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkflowIdentityPolicy {
    /// The exact GitHub repository in `owner/name` form.
    pub repository: String,
    /// The workflow file path, such as `.github/workflows/receipt.yml`.
    pub workflow_path: String,
    /// The exact signer ref, such as `refs/heads/main`.
    pub reference: String,
    /// The exact OIDC issuer.
    pub issuer: String,
    /// The log evidence this trusted workflow policy requires.
    pub transparency: TransparencyRequirement,
}

impl WorkflowIdentityPolicy {
    fn identity_uri(&self) -> String {
        format!(
            "{}{}github.com/{}/{}@{}",
            "https:", "//", self.repository, self.workflow_path, self.reference
        )
    }

    fn sigstore_policy(&self) -> VerificationPolicy {
        let policy = VerificationPolicy::any_identity();
        let policy = match self.transparency {
            TransparencyRequirement::PublicLog => policy,
            TransparencyRequirement::TimestampOnly => policy.skip_tlog_unsafe(),
        };
        policy
            .require_identity(self.identity_uri())
            .require_issuer(self.issuer.clone())
    }
}

/// Trusted policy for how a signature's transparency evidence is checked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransparencyRequirement {
    /// Require Sigstore transparency-log inclusion evidence.
    PublicLog,
    /// Accept timestamp-only bundles from a Sigstore instance that operates
    /// no transparency log, such as GitHub's private-repository instance.
    TimestampOnly,
}

/// The caller-provided expected git subject.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExpectedGitSubject<'a> {
    /// Which git digest member to compare.
    pub kind: GitSubjectKind,
    /// The expected digest string.
    pub value: &'a str,
}

/// The shared verifier seam for production Sigstore verification and test
/// doubles. Implementations must authenticate the bundle and enforce the
/// supplied workflow policy before returning success.
pub trait BundleVerifier {
    /// Verify these exact payload bytes against a Sigstore bundle and policy.
    ///
    /// # Errors
    ///
    /// Returns a typed verification failure for invalid signatures or an
    /// identity that is outside the trusted policy.
    fn verify(
        &self,
        payload: &[u8],
        bundle_json: &str,
        policy: &WorkflowIdentityPolicy,
    ) -> Result<(), VerificationFailure>;
}

/// The production verifier backed by sigstore-rust and a caller-selected root.
pub struct SigstoreBundleVerifier {
    verifier: sigstore_verify::Verifier,
}

impl SigstoreBundleVerifier {
    /// Prepare a verifier from explicit Sigstore trust material.
    ///
    /// # Errors
    ///
    /// Returns an error when the trusted root is malformed.
    pub fn new(
        trusted_root: &sigstore_verify::trust_root::TrustedRoot,
    ) -> Result<Self, VerificationFailure> {
        sigstore_verify::Verifier::new(trusted_root)
            .map(|verifier| Self { verifier })
            .map_err(|_| VerificationFailure::InvalidSignature)
    }
}

impl BundleVerifier for SigstoreBundleVerifier {
    fn verify(
        &self,
        payload: &[u8],
        bundle_json: &str,
        policy: &WorkflowIdentityPolicy,
    ) -> Result<(), VerificationFailure> {
        let bundle = sigstore_types::Bundle::from_json(bundle_json)
            .map_err(|_| VerificationFailure::InvalidSignature)?;

        // Verify cryptography and Sigstore transparency material before
        // classifying identity. This keeps a bad signature distinct from a
        // valid signature by an unlisted workflow.
        let cryptographic_policy = match policy.transparency {
            TransparencyRequirement::PublicLog => VerificationPolicy::any_identity(),
            TransparencyRequirement::TimestampOnly => {
                VerificationPolicy::any_identity().skip_tlog_unsafe()
            }
        };
        let result = self
            .verifier
            .verify(payload, &bundle, &cryptographic_policy)
            .map_err(|_| VerificationFailure::InvalidSignature)?;
        let expected = policy.sigstore_policy();
        let identity_matches = match (expected.identity(), result.identity()) {
            (Some(matcher), Some(identity)) => matcher.matches(identity),
            _ => false,
        };
        if !identity_matches || result.issuer() != Some(policy.issuer.as_str()) {
            return Err(VerificationFailure::UntrustedSigner);
        }
        Ok(())
    }
}

/// Verify a signed statement and then compare one string-valued git subject.
///
/// # Errors
///
/// Returns `SignatureInvalid` when delegated Sigstore verification rejects the
/// bundle, `UntrustedSigner` when its authenticated workflow identity does not
/// match policy, or `WrongSubject` when the requested string subject differs.
pub fn verify_statement(
    verifier: &dyn BundleVerifier,
    payload: &[u8],
    bundle_json: &str,
    policy: &WorkflowIdentityPolicy,
    subject: ExpectedGitSubject<'_>,
) -> Result<VerificationStatus, VerificationFailure> {
    verifier.verify(payload, bundle_json, policy)?;
    let statement: StatementV1 =
        serde_json::from_slice(payload).map_err(|_| VerificationFailure::InvalidStatement)?;
    if statement.type_ != STATEMENT_V1_TYPE {
        return Err(VerificationFailure::InvalidStatement);
    }
    let digest_name = match subject.kind {
        GitSubjectKind::Commit => "gitCommit",
        GitSubjectKind::Tree => "gitTree",
    };
    let matches = statement.subject.iter().any(|candidate| {
        candidate
            .digest
            .get(digest_name)
            .is_some_and(|actual| actual == subject.value)
    });
    if !matches {
        return Err(VerificationFailure::WrongSubject);
    }
    Ok(VerificationStatus::Verified)
}

async fn ambient_identity_token() -> Result<sigstore_sign::oidc::IdentityToken, SigningFailure> {
    let identity_token = sigstore_sign::oidc::IdentityToken::detect_ambient()
        .await
        .map_err(|_| SigningFailure::AmbientOidcUnavailable)?;
    require_ambient_token(identity_token)
}

fn require_ambient_token(
    token: Option<sigstore_sign::oidc::IdentityToken>,
) -> Result<sigstore_sign::oidc::IdentityToken, SigningFailure> {
    token.ok_or(SigningFailure::AmbientOidcUnavailable)
}

/// Sign a statement in a Sigstore instance without a public transparency log.
///
/// OIDC discovery happens before constructing any network client or contacting
/// Sigstore services. The statement bytes must already be canonical and use
/// the in-toto Statement v1 shape.
///
/// # Errors
///
/// Returns `AmbientOidcUnavailable` if no ambient token is available, or
/// `SigningFailed` when the delegated Sigstore signer fails.
pub async fn sign_statement_without_public_log(
    canonical_statement: &[u8],
) -> Result<String, SigningFailure> {
    let identity_token = ambient_identity_token().await?;
    let context = SigningContext::for_instance(SigstoreInstance::GitHub)
        .await
        .map_err(|_| SigningFailure::SigningFailed)?;
    let bundle = context
        .signer(identity_token)
        .sign_raw_statement(canonical_statement)
        .await
        .map_err(|_| SigningFailure::SigningFailed)?;
    bundle.to_json().map_err(|_| SigningFailure::SigningFailed)
}

/// Sign the bytes protected by the public-path proof.
///
/// The proof can only be created by `record_sink::leak_check`, after the
/// fail-closed public visibility check.
///
/// # Errors
///
/// Returns the same failures as [`sign_statement_with_ambient_oidc`].
pub async fn sign_public_path_with_ambient_oidc(
    proof: &PublicPathProof<'_>,
) -> Result<String, SigningFailure> {
    let identity_token = ambient_identity_token().await?;
    let context = SigningContext::for_instance(SigstoreInstance::PublicGood)
        .await
        .map_err(|_| SigningFailure::SigningFailed)?;
    let bundle = context
        .signer(identity_token)
        .sign(proof.signed_bytes())
        .await
        .map_err(|_| SigningFailure::SigningFailed)?;
    bundle.to_json().map_err(|_| SigningFailure::SigningFailed)
}

/// A statement could not be represented as canonical JSON.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("statement cannot be encoded as canonical JSON")]
pub struct StatementEncodingError;

/// The outcome for a successfully verified statement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationStatus {
    /// The signature, signer identity, and expected subject all matched.
    Verified,
}

/// A verification failure with stable machine-readable distinctions.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum VerificationFailure {
    /// The Sigstore bundle or signature is invalid.
    #[error("signature_invalid")]
    InvalidSignature,
    /// The authenticated workflow identity is not listed by policy.
    #[error("untrusted_signer")]
    UntrustedSigner,
    /// The verified statement does not name the caller's git subject.
    #[error("wrong_subject")]
    WrongSubject,
    /// The signed bytes are not an in-toto Statement v1.
    #[error("statement_invalid")]
    InvalidStatement,
}

/// Signing failed with a stable reason.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SigningFailure {
    /// No ambient OIDC identity token is available.
    #[error("ambient_oidc_unavailable")]
    AmbientOidcUnavailable,
    /// The Sigstore signing stack failed.
    #[error("signing_failed")]
    SigningFailed,
}

impl VerificationFailure {
    /// The stable machine-readable code for this failure.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidSignature => "signature_invalid",
            Self::UntrustedSigner => "untrusted_signer",
            Self::WrongSubject => "wrong_subject",
            Self::InvalidStatement => "statement_invalid",
        }
    }

    /// Every failure code this module emits.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::InvalidSignature,
            Self::UntrustedSigner,
            Self::WrongSubject,
            Self::InvalidStatement,
        ]
    }

    /// Resolve a stable code to its typed variant.
    #[must_use]
    pub const fn from_code(code: &str) -> Option<Self> {
        match code.as_bytes() {
            b"signature_invalid" => Some(Self::InvalidSignature),
            b"untrusted_signer" => Some(Self::UntrustedSigner),
            b"wrong_subject" => Some(Self::WrongSubject),
            b"statement_invalid" => Some(Self::InvalidStatement),
            _ => None,
        }
    }
}

impl SigningFailure {
    /// The stable machine-readable code for this failure.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AmbientOidcUnavailable => "ambient_oidc_unavailable",
            Self::SigningFailed => "signing_failed",
        }
    }

    /// Every signing failure code this module emits.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::AmbientOidcUnavailable, Self::SigningFailed]
    }

    /// Resolve a stable code to its typed variant.
    #[must_use]
    pub const fn from_code(code: &str) -> Option<Self> {
        match code.as_bytes() {
            b"ambient_oidc_unavailable" => Some(Self::AmbientOidcUnavailable),
            b"signing_failed" => Some(Self::SigningFailed),
            _ => None,
        }
    }
}

impl VerificationStatus {
    /// The stable result spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
        }
    }
}

impl StatementEncodingError {
    /// The stable machine-readable code for canonicalization failure.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        "statement_encoding_failed"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAYLOAD: &[u8] = concat!(
        r#"{"_type":"https:"#,
        r#"//in-toto.io/Statement/v1","predicate":{},"predicateType":"urn:example:predicate:v1","subject":[{"digest":{"gitCommit":"abc123","gitTree":"def456"},"name":"source"}]}"#
    )
    .as_bytes();
    const BUNDLE: &str = "fixture-bundle";

    struct FixtureVerifier {
        accepted_payload: Vec<u8>,
        accepted_identity: String,
        accepted_issuer: String,
    }

    impl BundleVerifier for FixtureVerifier {
        fn verify(
            &self,
            payload: &[u8],
            bundle_json: &str,
            policy: &WorkflowIdentityPolicy,
        ) -> Result<(), VerificationFailure> {
            if bundle_json != BUNDLE || payload != self.accepted_payload {
                return Err(VerificationFailure::InvalidSignature);
            }
            let identity = format!(
                "{}{}github.com/{}/{}@{}",
                "https:", "//", policy.repository, policy.workflow_path, policy.reference
            );
            if identity != self.accepted_identity || policy.issuer != self.accepted_issuer {
                return Err(VerificationFailure::UntrustedSigner);
            }
            Ok(())
        }
    }

    fn policy() -> WorkflowIdentityPolicy {
        WorkflowIdentityPolicy {
            repository: "fictional/project".to_owned(),
            workflow_path: ".github/workflows/attest.yml".to_owned(),
            reference: "refs/heads/main".to_owned(),
            issuer: concat!("https:", "//token.actions.githubusercontent.com").to_owned(),
            transparency: TransparencyRequirement::PublicLog,
        }
    }

    fn verifier() -> FixtureVerifier {
        let policy = policy();
        FixtureVerifier {
            accepted_payload: PAYLOAD.to_vec(),
            accepted_identity: policy.identity_uri(),
            accepted_issuer: policy.issuer,
        }
    }

    /// Trace: FR-064-AC-1, FR-064-AC-2, FR-064-AC-3, FR-064-AC-4
    #[test]
    fn verifies_policy_and_git_subjects_and_rejects_mismatches() {
        let verifier = verifier();
        assert_eq!(
            verify_statement(
                &verifier,
                PAYLOAD,
                BUNDLE,
                &policy(),
                ExpectedGitSubject {
                    kind: GitSubjectKind::Commit,
                    value: "abc123",
                },
            ),
            Ok(VerificationStatus::Verified)
        );
        assert_eq!(
            verify_statement(
                &verifier,
                PAYLOAD,
                BUNDLE,
                &policy(),
                ExpectedGitSubject {
                    kind: GitSubjectKind::Tree,
                    value: "def456",
                },
            ),
            Ok(VerificationStatus::Verified)
        );

        for changed in [
            WorkflowIdentityPolicy {
                workflow_path: ".github/workflows/other.yml".to_owned(),
                ..policy()
            },
            WorkflowIdentityPolicy {
                reference: "refs/heads/review".to_owned(),
                ..policy()
            },
            WorkflowIdentityPolicy {
                repository: "fictional/other".to_owned(),
                ..policy()
            },
        ] {
            assert_eq!(
                verify_statement(
                    &verifier,
                    PAYLOAD,
                    BUNDLE,
                    &changed,
                    ExpectedGitSubject {
                        kind: GitSubjectKind::Commit,
                        value: "abc123",
                    },
                ),
                Err(VerificationFailure::UntrustedSigner)
            );
        }

        let mut changed_payload = PAYLOAD.to_vec();
        let changed_index = PAYLOAD
            .windows(b"abc123".len())
            .position(|window| window == b"abc123")
            .expect("fixture contains the git commit subject");
        changed_payload[changed_index] = b'z';
        assert_eq!(
            verify_statement(
                &verifier,
                &changed_payload,
                BUNDLE,
                &policy(),
                ExpectedGitSubject {
                    kind: GitSubjectKind::Commit,
                    value: "abc123",
                },
            ),
            Err(VerificationFailure::InvalidSignature)
        );
        assert_eq!(
            verify_statement(
                &verifier,
                PAYLOAD,
                BUNDLE,
                &policy(),
                ExpectedGitSubject {
                    kind: GitSubjectKind::Commit,
                    value: "different",
                },
            ),
            Err(VerificationFailure::WrongSubject)
        );
    }

    struct RejectingVerifier;

    impl BundleVerifier for RejectingVerifier {
        fn verify(
            &self,
            _payload: &[u8],
            _bundle_json: &str,
            _policy: &WorkflowIdentityPolicy,
        ) -> Result<(), VerificationFailure> {
            Err(VerificationFailure::InvalidSignature)
        }
    }

    /// Trace: NFR-010-M-3
    #[test]
    fn rejecting_sigstore_verifier_rejects_all_statement_cases() {
        let verifier = RejectingVerifier;
        for (bundle, subject, expected) in [
            (BUNDLE, "abc123", VerificationFailure::InvalidSignature),
            (BUNDLE, "abc123", VerificationFailure::InvalidSignature),
            (BUNDLE, "abc123", VerificationFailure::InvalidSignature),
            (BUNDLE, "different", VerificationFailure::InvalidSignature),
        ] {
            assert_eq!(
                verify_statement(
                    &verifier,
                    PAYLOAD,
                    bundle,
                    &policy(),
                    ExpectedGitSubject {
                        kind: GitSubjectKind::Commit,
                        value: subject,
                    },
                ),
                Err(expected)
            );
        }
    }

    /// Trace: FR-058-AC-1
    #[test]
    fn statement_serializes_as_plain_in_toto_v1_and_canonicalizes() {
        let statement: StatementV1 = serde_json::from_slice(PAYLOAD).expect("fixture parses");
        assert_eq!(statement.type_, STATEMENT_V1_TYPE);
        assert_eq!(
            statement.canonical_bytes().expect("canonical bytes"),
            PAYLOAD
        );
    }

    /// Trace: FR-058-AC-7
    #[test]
    fn signing_requires_an_ambient_oidc_token_before_service_setup() {
        assert!(matches!(
            require_ambient_token(None),
            Err(SigningFailure::AmbientOidcUnavailable)
        ));
    }

    /// Trace: FR-064-AC-1, FR-064-AC-2, FR-064-AC-3, FR-064-AC-4
    #[test]
    fn verification_failure_codes_round_trip() {
        for code in VerificationFailure::all() {
            assert_eq!(VerificationFailure::from_code(code.as_str()), Some(*code));
        }
        for code in SigningFailure::all() {
            assert_eq!(SigningFailure::from_code(code.as_str()), Some(*code));
        }
        assert_eq!(VerificationFailure::from_code("unknown"), None);
        assert_eq!(SigningFailure::from_code("unknown"), None);
    }
}
