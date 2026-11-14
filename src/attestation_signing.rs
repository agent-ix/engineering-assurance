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
//! not interpret Engineering Assurance's `gitCommit`, `gitTree`, or `sha256`
//! subject members. After Sigstore accepts the bundle and certificate chain,
//! this module compares the requested subject member as an exact string. It
//! does not hash a git object or infer one from the statement's repository name.
//! Transparency-log inclusion is required by default policy configuration;
//! timestamp-only verification is selected only for a trusted instance policy
//! whose signing service does not operate a transparency log.

use std::{
    collections::{BTreeMap, btree_map::Entry},
    future::Future,
    sync::Mutex,
    thread,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sigstore_sign::{SigningContext, SigstoreInstance};
use sigstore_verify::VerificationPolicy;

use crate::record_sink::leak::PublicPathProof;
use crate::record_sink::sink::{Signer, SigningFailed as SinkSigningFailed};

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

/// A statement subject digest selected for comparison with caller input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubjectKind {
    /// Compare the `gitCommit` string.
    Commit,
    /// Compare the `gitTree` string.
    Tree,
    /// Compare the `sha256` digest string of an artifact.
    Sha256,
}

/// The expected signer identity from protected workflow policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkflowIdentityPolicy {
    /// The exact GitHub repository in `owner/name` form.
    pub repository: String,
    /// The workflow file path, such as `.github/workflows/receipt.yml`.
    pub workflow_path: String,
    /// Exact signer refs allowed by protected workflow policy.
    pub references: std::collections::BTreeSet<String>,
    /// The exact OIDC issuer.
    pub issuer: String,
    /// The log evidence this trusted workflow policy requires.
    pub transparency: TransparencyRequirement,
}

impl WorkflowIdentityPolicy {
    fn identity_uri(&self, reference: &str) -> String {
        format!(
            "{}{}github.com/{}/{}@{}",
            "https:", "//", self.repository, self.workflow_path, reference
        )
    }

    fn matches_identity(&self, identity: &sigstore_verify::SubjectAltName) -> bool {
        self.references.iter().any(|reference| {
            let expected =
                VerificationPolicy::any_identity().require_identity(self.identity_uri(reference));
            expected
                .identity()
                .is_some_and(|matcher| matcher.matches(identity))
        })
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

/// The caller-provided expected statement subject digest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExpectedSubject<'a> {
    /// Which digest member to compare.
    pub kind: SubjectKind,
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
        let identity_matches = result
            .identity()
            .is_some_and(|identity| policy.matches_identity(identity));
        if !identity_matches || result.issuer() != Some(policy.issuer.as_str()) {
            return Err(VerificationFailure::UntrustedSigner);
        }
        Ok(())
    }
}

/// Verify a signed statement and then compare one string-valued subject digest.
///
/// # Errors
///
/// Returns `signature_invalid` when delegated Sigstore verification rejects the
/// bundle, `UntrustedSigner` when its authenticated workflow identity does not
/// match policy, or `wrong_subject` when the requested string subject differs.
pub fn verify_statement(
    verifier: &dyn BundleVerifier,
    payload: &[u8],
    bundle_json: &str,
    policy: &WorkflowIdentityPolicy,
    subject: ExpectedSubject<'_>,
) -> Result<VerificationStatus, VerificationFailure> {
    verifier.verify(payload, bundle_json, policy)?;
    let statement: StatementV1 =
        serde_json::from_slice(payload).map_err(|_| VerificationFailure::InvalidStatement)?;
    if statement.type_ != STATEMENT_V1_TYPE {
        return Err(VerificationFailure::InvalidStatement);
    }
    let digest_name = match subject.kind {
        SubjectKind::Commit => "gitCommit",
        SubjectKind::Tree => "gitTree",
        SubjectKind::Sha256 => "sha256",
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

async fn sign_after_ambient_oidc<T, Detect, Start, Sign>(
    detect: Detect,
    start_signing: Start,
) -> Result<T, SigningFailure>
where
    Detect: Future<Output = Result<Option<sigstore_sign::oidc::IdentityToken>, SigningFailure>>,
    Start: FnOnce(sigstore_sign::oidc::IdentityToken) -> Sign,
    Sign: Future<Output = Result<T, SigningFailure>>,
{
    let token = detect
        .await?
        .ok_or(SigningFailure::AmbientOidcUnavailable)?;
    start_signing(token).await
}

async fn detect_ambient_oidc() -> Result<Option<sigstore_sign::oidc::IdentityToken>, SigningFailure>
{
    sigstore_sign::oidc::IdentityToken::detect_ambient()
        .await
        .map_err(|_| SigningFailure::AmbientOidcUnavailable)
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
    sign_statement_without_public_log_with(canonical_statement, detect_ambient_oidc()).await
}

async fn sign_statement_without_public_log_with<Detect>(
    canonical_statement: &[u8],
    detect: Detect,
) -> Result<String, SigningFailure>
where
    Detect: Future<Output = Result<Option<sigstore_sign::oidc::IdentityToken>, SigningFailure>>,
{
    sign_after_ambient_oidc(detect, |identity_token| async move {
        let context = SigningContext::for_instance(SigstoreInstance::GitHub)
            .await
            .map_err(|_| SigningFailure::SigningFailed)?;
        let bundle = context
            .signer(identity_token)
            .sign_raw_statement(canonical_statement)
            .await
            .map_err(|_| SigningFailure::SigningFailed)?;
        bundle.to_json().map_err(|_| SigningFailure::SigningFailed)
    })
    .await
}

/// Sign the bytes protected by the public-path proof.
///
/// The proof can only be created by `record_sink::leak_check`, after the
/// fail-closed public visibility check.
///
/// # Errors
///
/// Returns `AmbientOidcUnavailable` if no ambient token is available, or
/// `SigningFailed` when the delegated Sigstore signer fails.
async fn sign_public_payload(payload: &[u8]) -> Result<String, SigningFailure> {
    sign_public_payload_with(payload, detect_ambient_oidc()).await
}

async fn sign_public_payload_with<Detect>(
    payload: &[u8],
    detect: Detect,
) -> Result<String, SigningFailure>
where
    Detect: Future<Output = Result<Option<sigstore_sign::oidc::IdentityToken>, SigningFailure>>,
{
    sign_after_ambient_oidc(detect, |identity_token| async move {
        let context = SigningContext::for_instance(SigstoreInstance::PublicGood)
            .await
            .map_err(|_| SigningFailure::SigningFailed)?;
        let bundle = context
            .signer(identity_token)
            .sign(payload)
            .await
            .map_err(|_| SigningFailure::SigningFailed)?;
        bundle.to_json().map_err(|_| SigningFailure::SigningFailed)
    })
    .await
}

async fn sign_private_payload(payload: &[u8]) -> Result<String, SigningFailure> {
    sign_private_payload_with(payload, detect_ambient_oidc()).await
}

async fn sign_private_payload_with<Detect>(
    payload: &[u8],
    detect: Detect,
) -> Result<String, SigningFailure>
where
    Detect: Future<Output = Result<Option<sigstore_sign::oidc::IdentityToken>, SigningFailure>>,
{
    sign_after_ambient_oidc(detect, |identity_token| async move {
        let context = SigningContext::for_instance(SigstoreInstance::GitHub)
            .await
            .map_err(|_| SigningFailure::SigningFailed)?;
        let bundle = context
            .signer(identity_token)
            .sign(payload)
            .await
            .map_err(|_| SigningFailure::SigningFailed)?;
        bundle.to_json().map_err(|_| SigningFailure::SigningFailed)
    })
    .await
}

/// The EA-105 Sigstore signer used by `PublicLogSink`.
///
/// `signed_payload` returns only payloads signed by this instance and keeps a
/// bounded, one-shot cache so blob bundles need not duplicate artifact bytes.
pub struct SigstoreSigner {
    pending_payloads: Mutex<BTreeMap<String, Vec<u8>>>,
}

#[derive(Clone, Copy)]
enum SigningPath {
    PublicLog,
    NoPublicLog,
}

impl SigstoreSigner {
    /// Construct a signer whose operations use ambient keyless OIDC.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pending_payloads: Mutex::new(BTreeMap::new()),
        }
    }

    fn sign_and_remember(
        &self,
        payload: &[u8],
        signing_path: SigningPath,
    ) -> Result<String, SinkSigningFailed> {
        const MAX_PENDING_BUNDLES: usize = 16;
        let mut pending = self
            .pending_payloads
            .lock()
            .map_err(|_| SinkSigningFailed)?;
        if pending.len() >= MAX_PENDING_BUNDLES {
            return Err(SinkSigningFailed);
        }
        let payload = payload.to_vec();
        let payload_for_signing = payload.clone();
        let bundle = run_signing_thread(move || async move {
            match signing_path {
                SigningPath::PublicLog => sign_public_payload(&payload_for_signing).await,
                SigningPath::NoPublicLog => sign_private_payload(&payload_for_signing).await,
            }
        })
        .map_err(|_| SinkSigningFailed)?;
        match pending.entry(bundle.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(payload);
                Ok(bundle)
            }
            Entry::Occupied(_) => Err(SinkSigningFailed),
        }
    }
}

impl Default for SigstoreSigner {
    fn default() -> Self {
        Self::new()
    }
}

impl Signer for SigstoreSigner {
    fn sign_for_public_log(
        &self,
        proof: &PublicPathProof<'_>,
    ) -> Result<String, SinkSigningFailed> {
        self.sign_and_remember(proof.signed_bytes(), SigningPath::PublicLog)
    }

    fn sign_without_public_log(&self, canonical: &[u8]) -> Result<String, SinkSigningFailed> {
        self.sign_and_remember(canonical, SigningPath::NoPublicLog)
    }

    fn signed_payload(&self, bundle: &str) -> Result<Vec<u8>, SinkSigningFailed> {
        self.pending_payloads
            .lock()
            .map_err(|_| SinkSigningFailed)?
            .remove(bundle)
            .ok_or(SinkSigningFailed)
    }
}

fn run_signing_thread<T, Build, Work>(build: Build) -> Result<T, SigningFailure>
where
    T: Send + 'static,
    Build: FnOnce() -> Work + Send + 'static,
    Work: Future<Output = Result<T, SigningFailure>> + Send + 'static,
{
    thread::Builder::new()
        .name("sigstore-signing".to_owned())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| SigningFailure::SigningFailed)?;
            runtime.block_on(build())
        })
        .map_err(|_| SigningFailure::SigningFailed)?
        .join()
        .map_err(|_| SigningFailure::SigningFailed)?
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
    use ix_trace_rs::trace;

    const PAYLOAD: &[u8] = concat!(
        r#"{"_type":"https:"#,
        r#"//in-toto.io/Statement/v1","predicate":{},"predicateType":"urn:example:predicate:v1","subject":[{"digest":{"gitCommit":"abc123","gitTree":"def456","sha256":"0000000000000000000000000000000000000000000000000000000000000000"},"name":"source"}]}"#
    )
    .as_bytes();
    const NO_SHA256_PAYLOAD: &[u8] = concat!(
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
            let accepted = policy.references.iter().any(|reference| {
                format!(
                    "{}{}github.com/{}/{}@{}",
                    "https:", "//", policy.repository, policy.workflow_path, reference
                ) == self.accepted_identity
            });
            if !accepted || policy.issuer != self.accepted_issuer {
                return Err(VerificationFailure::UntrustedSigner);
            }
            Ok(())
        }
    }

    fn policy() -> WorkflowIdentityPolicy {
        WorkflowIdentityPolicy {
            repository: "fictional/project".to_owned(),
            workflow_path: ".github/workflows/attest.yml".to_owned(),
            references: ["refs/heads/main".to_owned()].into(),
            issuer: concat!("https:", "//token.actions.githubusercontent.com").to_owned(),
            transparency: TransparencyRequirement::PublicLog,
        }
    }

    fn verifier() -> FixtureVerifier {
        let policy = policy();
        FixtureVerifier {
            accepted_payload: PAYLOAD.to_vec(),
            accepted_identity: policy.identity_uri("refs/heads/main"),
            accepted_issuer: policy.issuer,
        }
    }

    fn verifier_for(payload: &[u8], reference: &str) -> FixtureVerifier {
        let policy = policy();
        FixtureVerifier {
            accepted_payload: payload.to_vec(),
            accepted_identity: policy.identity_uri(reference),
            accepted_issuer: policy.issuer,
        }
    }

    #[trace("FR-064-AC-1", "FR-064-AC-2", "FR-064-AC-3", "FR-064-AC-4")]
    #[test]
    fn verifies_policy_and_git_subjects_and_rejects_mismatches() {
        let verifier = verifier();
        assert_eq!(
            verify_statement(
                &verifier,
                PAYLOAD,
                BUNDLE,
                &policy(),
                ExpectedSubject {
                    kind: SubjectKind::Commit,
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
                ExpectedSubject {
                    kind: SubjectKind::Tree,
                    value: "def456",
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
                ExpectedSubject {
                    kind: SubjectKind::Sha256,
                    value: "0000000000000000000000000000000000000000000000000000000000000000",
                },
            ),
            Ok(VerificationStatus::Verified)
        );

        let allowed_refs = WorkflowIdentityPolicy {
            references: [
                "refs/heads/main".to_owned(),
                "refs/heads/release".to_owned(),
            ]
            .into(),
            ..policy()
        };
        let release_verifier = verifier_for(PAYLOAD, "refs/heads/release");
        assert_eq!(
            verify_statement(
                &release_verifier,
                PAYLOAD,
                BUNDLE,
                &allowed_refs,
                ExpectedSubject {
                    kind: SubjectKind::Commit,
                    value: "abc123",
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
                references: ["refs/heads/review".to_owned()].into(),
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
                    ExpectedSubject {
                        kind: SubjectKind::Commit,
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
                ExpectedSubject {
                    kind: SubjectKind::Commit,
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
                ExpectedSubject {
                    kind: SubjectKind::Commit,
                    value: "different",
                },
            ),
            Err(VerificationFailure::WrongSubject)
        );
        assert_eq!(
            verify_statement(
                &verifier,
                PAYLOAD,
                BUNDLE,
                &policy(),
                ExpectedSubject {
                    kind: SubjectKind::Tree,
                    value: "different-tree",
                },
            ),
            Err(VerificationFailure::WrongSubject)
        );
        let no_sha_verifier = verifier_for(NO_SHA256_PAYLOAD, "refs/heads/main");
        assert_eq!(
            verify_statement(
                &no_sha_verifier,
                NO_SHA256_PAYLOAD,
                BUNDLE,
                &policy(),
                ExpectedSubject {
                    kind: SubjectKind::Sha256,
                    value: "0000000000000000000000000000000000000000000000000000000000000000",
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

    #[trace("NFR-010-M-3")]
    #[test]
    fn rejecting_sigstore_verifier_rejects_policy_payload_and_subject_cases() {
        let verifier = RejectingVerifier;
        let mut tampered_payload = PAYLOAD.to_vec();
        let commit_offset = PAYLOAD
            .windows(b"abc123".len())
            .position(|window| window == b"abc123")
            .expect("fixture contains the git commit subject");
        tampered_payload[commit_offset] = b'z';
        let wrong_policy = WorkflowIdentityPolicy {
            repository: "fictional/untrusted".to_owned(),
            ..policy()
        };
        let trusted_policy = policy();
        let cases = [
            (PAYLOAD, &trusted_policy, SubjectKind::Commit, "abc123"),
            (PAYLOAD, &wrong_policy, SubjectKind::Commit, "abc123"),
            (
                tampered_payload.as_slice(),
                &trusted_policy,
                SubjectKind::Commit,
                "abc123",
            ),
            (
                PAYLOAD,
                &trusted_policy,
                SubjectKind::Tree,
                "different-tree",
            ),
        ];
        for (payload, case_policy, kind, subject) in cases {
            assert_eq!(
                verify_statement(
                    &verifier,
                    payload,
                    BUNDLE,
                    case_policy,
                    ExpectedSubject {
                        kind,
                        value: subject,
                    },
                ),
                Err(VerificationFailure::InvalidSignature)
            );
        }
    }

    #[test]
    fn statement_serializes_as_plain_in_toto_v1_and_canonicalizes() {
        let statement: StatementV1 = serde_json::from_slice(PAYLOAD).expect("fixture parses");
        assert_eq!(statement.type_, STATEMENT_V1_TYPE);
        assert_eq!(
            statement.canonical_bytes().expect("canonical bytes"),
            PAYLOAD
        );
    }

    #[tokio::test]
    async fn signing_requires_an_ambient_oidc_token_before_service_setup() {
        let missing_token = || async { Ok(None) };
        for (signer_kind, result) in [
            (
                "public-good payload signer",
                sign_public_payload_with(b"fictional payload", missing_token()).await,
            ),
            (
                "private-instance payload signer",
                sign_private_payload_with(b"fictional payload", missing_token()).await,
            ),
            (
                "raw-statement signer",
                sign_statement_without_public_log_with(b"fictional statement", missing_token())
                    .await,
            ),
        ] {
            assert_eq!(
                result,
                Err(SigningFailure::AmbientOidcUnavailable),
                "{signer_kind} signer must fail with the named missing-token error"
            );
        }
    }

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
