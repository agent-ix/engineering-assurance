// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Authenticate and bind the source-signed execution provenance before Quoin
//! consumes the retained evidence store (FR-069).

use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use engineering_assurance::attestation_signing::{
    BundleVerifier, SigstoreBundleVerifier, StatementV1, SubjectV1, TransparencyRequirement,
    VerificationFailure, WorkflowIdentityDocument, WorkflowRole,
};
use ix_trace_rs::implements;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{process_host, process_host::ProcessLimits, receipt_host};

const REQUEST_PROTOCOL: &str = "engineering-assurance.release-evidence-verify-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.release-evidence-verify-result/v1";
const EXECUTION_PROVENANCE_PREDICATE: &str =
    "application/vnd.agent-ix.execution-provenance.v2+json";
const MAX_REQUEST_BYTES: usize = 1024 * 1024;
const MAX_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;
const MAX_BUNDLE_BYTES: usize = 32 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    repository: PathBuf,
    github_repository: String,
    base_branch: String,
    signer_repository: String,
    signer_workflow_path: String,
    signer_ref: String,
    tag: String,
    commit: String,
    bundle_path: PathBuf,
    subject_path: PathBuf,
    verified_bundle_path: PathBuf,
}

#[derive(Serialize)]
struct ResultLine {
    protocol: &'static str,
    status: &'static str,
    commit: String,
    subjects: usize,
}

#[derive(Debug, Error)]
pub(crate) enum ReleaseEvidenceError {
    #[error("release evidence verification request is invalid")]
    Request,
    #[error("protected producer identity policy is unavailable or invalid")]
    Policy,
    #[error("execution provenance is absent, unreadable, or malformed")]
    ProvenanceInvalid,
    #[error("execution provenance is signed by an untrusted workflow")]
    UntrustedSigner,
    #[error("execution provenance does not bind the release commit and subjects")]
    BindingInvalid,
    #[error("verified execution provenance could not be retained unchanged")]
    Output,
    #[error("release evidence verification result cannot be encoded")]
    Encoding,
}

impl ReleaseEvidenceError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Request => "release_evidence_request_invalid",
            Self::Policy => "release_evidence_policy_unavailable",
            Self::ProvenanceInvalid => "execution_provenance_invalid",
            Self::UntrustedSigner => "execution_provenance_untrusted_signer",
            Self::BindingInvalid => "execution_provenance_binding_invalid",
            Self::Output => "execution_provenance_output_failed",
            Self::Encoding => "release_evidence_result_encoding_failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
enum ProvenanceFailure {
    #[error("invalid provenance envelope")]
    Invalid,
    #[error("untrusted provenance signer")]
    UntrustedSigner,
    #[error("provenance subject or source revision mismatch")]
    Binding,
}

/// Authenticate the exact source-signed bundle, then retain its original bytes
/// for the Quoin exporter.
#[implements("FR-069-AC-2", "FR-069-AC-5", "FR-069-AC-6")]
pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, ReleaseEvidenceError> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(ReleaseEvidenceError::Request);
    }
    let request: Request =
        serde_json::from_slice(bytes).map_err(|_| ReleaseEvidenceError::Request)?;
    validate_request(&request)?;
    if !release_tag_points_to_commit(&request.repository, &request.tag, &request.commit) {
        return Err(ReleaseEvidenceError::BindingInvalid);
    }
    let policy = protected_source_signer(&request)?;
    let bundle = read_bounded_file(&request.bundle_path, MAX_BUNDLE_BYTES)
        .map_err(|_| ReleaseEvidenceError::ProvenanceInvalid)?;
    let subjects_bytes = read_bounded_file(&request.subject_path, MAX_DOCUMENT_BYTES)
        .map_err(|_| ReleaseEvidenceError::BindingInvalid)?;
    let subjects: Vec<SubjectV1> = serde_json::from_slice(&subjects_bytes)
        .map_err(|_| ReleaseEvidenceError::BindingInvalid)?;
    if subjects.is_empty() || subjects.len() > 4096 || !subject_set_is_unique(&subjects) {
        return Err(ReleaseEvidenceError::BindingInvalid);
    }
    let root_json = match policy.transparency {
        TransparencyRequirement::PublicLog => {
            sigstore_verify::trust_root::SIGSTORE_PRODUCTION_TRUSTED_ROOT
        }
        TransparencyRequirement::TimestampOnly => {
            sigstore_verify::trust_root::SIGSTORE_GITHUB_TRUSTED_ROOT
        }
    };
    let root = sigstore_verify::trust_root::TrustedRoot::from_json(root_json)
        .map_err(|_| ReleaseEvidenceError::Policy)?;
    let verifier = SigstoreBundleVerifier::new(&root).map_err(|_| ReleaseEvidenceError::Policy)?;
    verify_execution_provenance(&bundle, &subjects, &request.commit, &policy, &verifier)
        .map_err(map_verification_failure)?;

    retain_verified_bundle(&request.verified_bundle_path, &bundle)
        .map_err(|_| ReleaseEvidenceError::Output)?;
    encode_result(request.commit, subjects.len())
}

fn validate_request(request: &Request) -> Result<(), ReleaseEvidenceError> {
    if request.protocol != REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !request.repository.is_dir()
        || !receipt_host::valid_github_repository(&request.github_repository)
        || !receipt_host::valid_branch(&request.base_branch)
        || request.signer_repository != "agent-ix/engineering-assurance"
        || request.signer_workflow_path != ".github/workflows/release-attestation.yml"
        || !receipt_host::valid_branch(&request.tag)
        || !receipt_host::is_full_object_id(&request.commit)
        || !request.bundle_path.is_absolute()
        || !request.subject_path.is_absolute()
        || !request.verified_bundle_path.is_absolute()
        || request.bundle_path == request.verified_bundle_path
        || request.bundle_path == request.subject_path
        || request.subject_path == request.verified_bundle_path
    {
        return Err(ReleaseEvidenceError::Request);
    }
    Ok(())
}

fn release_tag_points_to_commit(repository: &Path, tag: &str, expected_commit: &str) -> bool {
    let reference = format!("refs/tags/{tag}");
    let check = run_git(
        repository,
        &[OsStr::new("check-ref-format"), OsStr::new(&reference)],
    );
    if !check.is_ok_and(|output| output.status.success()) {
        return false;
    }
    let peeled = format!("{reference}^{{commit}}");
    run_git(
        repository,
        &[
            OsStr::new("rev-parse"),
            OsStr::new("--verify"),
            OsStr::new("--end-of-options"),
            OsStr::new(&peeled),
        ],
    )
    .is_ok_and(|output| {
        output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == expected_commit
    })
}

fn run_git(
    repository: &Path,
    arguments: &[&OsStr],
) -> Result<process_host::CompletedProcess, process_host::ProcessError> {
    let mut git_arguments = Vec::with_capacity(arguments.len() + 2);
    git_arguments.extend([OsStr::new("-C"), repository.as_os_str()]);
    git_arguments.extend_from_slice(arguments);
    process_host::run(
        OsStr::new("git"),
        &git_arguments,
        ProcessLimits {
            timeout: Duration::from_secs(10),
            max_output_bytes: 8 * 1024,
        },
    )
}

fn protected_source_signer(
    request: &Request,
) -> Result<engineering_assurance::attestation_signing::WorkflowIdentityPolicy, ReleaseEvidenceError>
{
    let repository_info =
        receipt_host::gh_api_json(&format!("repos/{}", request.github_repository))
            .map_err(|_| ReleaseEvidenceError::Policy)?;
    if repository_info
        .get("default_branch")
        .and_then(Value::as_str)
        != Some(request.base_branch.as_str())
    {
        return Err(ReleaseEvidenceError::Policy);
    }
    let private = repository_info
        .get("private")
        .and_then(Value::as_bool)
        .ok_or(ReleaseEvidenceError::Policy)?;
    let branch = receipt_host::gh_api_json(&format!(
        "repos/{}/branches/{}",
        request.github_repository, request.base_branch
    ))
    .map_err(|_| ReleaseEvidenceError::Policy)?;
    if branch.get("protected").and_then(Value::as_bool) != Some(true) {
        return Err(ReleaseEvidenceError::Policy);
    }
    let base_commit = branch
        .pointer("/commit/sha")
        .and_then(Value::as_str)
        .filter(|sha| receipt_host::is_full_object_id(sha))
        .ok_or(ReleaseEvidenceError::Policy)?;
    let identity_bytes = receipt_host::git_show_base_file(
        &request.repository,
        base_commit,
        ".engineering-assurance/workflow-identities.json",
    )
    .map_err(|_| ReleaseEvidenceError::Policy)?;
    let identities = WorkflowIdentityDocument::parse(&identity_bytes)
        .map_err(|_| ReleaseEvidenceError::Policy)?;
    let transparency = if private {
        TransparencyRequirement::TimestampOnly
    } else {
        TransparencyRequirement::PublicLog
    };
    identities
        .source_signer_policy(
            WorkflowRole::ReleaseAttester,
            &request.signer_repository,
            &request.signer_workflow_path,
            &request.signer_ref,
            &format!("refs/tags/{}", request.tag),
            transparency,
        )
        .ok_or(ReleaseEvidenceError::Policy)
}

fn verify_execution_provenance(
    bundle_json: &[u8],
    expected_subjects: &[SubjectV1],
    commit: &str,
    signer: &engineering_assurance::attestation_signing::WorkflowIdentityPolicy,
    verifier: &dyn BundleVerifier,
) -> Result<(), ProvenanceFailure> {
    let bundle_text = std::str::from_utf8(bundle_json).map_err(|_| ProvenanceFailure::Invalid)?;
    let bundle =
        sigstore_types::Bundle::from_json(bundle_text).map_err(|_| ProvenanceFailure::Invalid)?;
    let sigstore_types::SignatureContent::DsseEnvelope(envelope) = bundle.content else {
        return Err(ProvenanceFailure::Invalid);
    };
    if envelope.payload_type != "application/vnd.in-toto+json" {
        return Err(ProvenanceFailure::Invalid);
    }
    let payload = envelope.payload.as_ref();
    let statement: StatementV1 =
        serde_json::from_slice(payload).map_err(|_| ProvenanceFailure::Invalid)?;
    if statement.type_ != engineering_assurance::attestation_signing::STATEMENT_V1_TYPE
        || statement.predicate_type != EXECUTION_PROVENANCE_PREDICATE
        || statement.predicate.get("version").and_then(Value::as_u64) != Some(2)
        || statement
            .predicate
            .get("sourceRevision")
            .and_then(Value::as_str)
            != Some(commit)
        || !same_subject_set(&statement.subject, expected_subjects)
    {
        return Err(ProvenanceFailure::Binding);
    }
    verifier
        .verify(payload, bundle_text, signer)
        .map_err(|error| match error {
            VerificationFailure::UntrustedSigner => ProvenanceFailure::UntrustedSigner,
            VerificationFailure::InvalidSignature => ProvenanceFailure::Invalid,
            VerificationFailure::WrongSubject | VerificationFailure::InvalidStatement => {
                ProvenanceFailure::Binding
            }
        })
}

fn same_subject_set(actual: &[SubjectV1], expected: &[SubjectV1]) -> bool {
    actual.len() == expected.len()
        && subject_set_is_unique(actual)
        && subject_set_is_unique(expected)
        && subject_keys(actual) == subject_keys(expected)
}

fn subject_set_is_unique(subjects: &[SubjectV1]) -> bool {
    let mut names = BTreeSet::new();
    subjects.iter().all(|subject| {
        !subject.name.is_empty()
            && subject.digest.len() == 1
            && subject.digest.get("sha256").is_some_and(|digest| {
                digest.len() == 64
                    && digest
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
            && names.insert(subject.name.as_str())
    })
}

fn subject_keys(subjects: &[SubjectV1]) -> BTreeSet<(String, String)> {
    subjects
        .iter()
        .filter_map(|subject| {
            subject
                .digest
                .get("sha256")
                .map(|digest| (subject.name.clone(), digest.clone()))
        })
        .collect()
}

fn map_verification_failure(error: ProvenanceFailure) -> ReleaseEvidenceError {
    match error {
        ProvenanceFailure::Invalid => ReleaseEvidenceError::ProvenanceInvalid,
        ProvenanceFailure::UntrustedSigner => ReleaseEvidenceError::UntrustedSigner,
        ProvenanceFailure::Binding => ReleaseEvidenceError::BindingInvalid,
    }
}

fn read_bounded_file(path: &Path, limit: usize) -> Result<Vec<u8>, ()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    if !metadata.file_type().is_file() || metadata.len() > limit as u64 {
        return Err(());
    }
    let file = File::open(path).map_err(|_| ())?;
    let opened = file.metadata().map_err(|_| ())?;
    if !opened.is_file() || opened.len() > limit as u64 {
        return Err(());
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() > limit {
        return Err(());
    }
    Ok(bytes)
}

fn retain_verified_bundle(path: &Path, bundle: &[u8]) -> Result<(), std::io::Error> {
    let mut output = OpenOptions::new().write(true).create_new(true).open(path)?;
    output.write_all(bundle)?;
    output.sync_all()
}

fn encode_result(commit: String, subjects: usize) -> Result<Vec<u8>, ReleaseEvidenceError> {
    let mut bytes = serde_json::to_vec(&ResultLine {
        protocol: RESULT_PROTOCOL,
        status: "verified",
        commit,
        subjects,
    })
    .map_err(|_| ReleaseEvidenceError::Encoding)?;
    bytes.push(b'\n');
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use ix_trace_rs::trace;
    use std::collections::BTreeMap;

    use engineering_assurance::attestation_signing::{
        TransparencyRequirement, VerificationFailure, WorkflowIdentityPolicy,
    };

    const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const ISSUER: &str = concat!("https:", "//", "token.actions.githubusercontent.com");

    struct FixtureVerifier {
        accepted_payload: Vec<u8>,
        accepted_signer: bool,
    }

    impl BundleVerifier for FixtureVerifier {
        fn verify(
            &self,
            payload: &[u8],
            _bundle_json: &str,
            policy: &engineering_assurance::attestation_signing::WorkflowIdentityPolicy,
        ) -> Result<(), VerificationFailure> {
            if !self.accepted_signer
                || policy.repository != "fictional/product"
                || policy.workflow_path != ".github/workflows/release.yml"
                || policy.references != BTreeSet::from(["refs/tags/v1.2.3".to_owned()])
                || policy.issuer != ISSUER
            {
                return Err(VerificationFailure::UntrustedSigner);
            }
            if payload != self.accepted_payload {
                return Err(VerificationFailure::InvalidSignature);
            }
            Ok(())
        }
    }

    fn subject(name: &str, digest: &str) -> SubjectV1 {
        SubjectV1 {
            name: name.to_owned(),
            digest: BTreeMap::from([("sha256".to_owned(), digest.to_owned())]),
        }
    }

    fn fixture_statement(subjects: Vec<SubjectV1>, commit: &str) -> Vec<u8> {
        serde_json::to_vec(&StatementV1 {
            type_: engineering_assurance::attestation_signing::STATEMENT_V1_TYPE.to_owned(),
            subject: subjects,
            predicate_type: EXECUTION_PROVENANCE_PREDICATE.to_owned(),
            predicate: BTreeMap::from([
                ("version".to_owned(), Value::from(2)),
                ("sourceRevision".to_owned(), Value::from(commit)),
            ]),
        })
        .expect("statement fixture serializes")
    }

    fn fixture_bundle(payload: &[u8]) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "mediaType": "application/vnd.dev.sigstore.bundle.v0.3+json",
            "verificationMaterial": {"certificate": {"rawBytes": "dGVzdA=="}},
            "dsseEnvelope": {
                "payloadType": "application/vnd.in-toto+json",
                "payload": STANDARD.encode(payload),
                "signatures": [{"sig": "dGVzdA=="}]
            }
        }))
        .expect("bundle fixture serializes")
    }

    fn run_fixture_git(repository: &Path, arguments: &[&str]) -> process_host::CompletedProcess {
        let arguments = arguments
            .iter()
            .map(|argument| OsStr::new(argument))
            .collect::<Vec<_>>();
        run_git(repository, &arguments).expect("git fixture command completes")
    }

    fn signer() -> WorkflowIdentityPolicy {
        WorkflowIdentityPolicy {
            repository: "fictional/product".to_owned(),
            workflow_path: ".github/workflows/release.yml".to_owned(),
            references: BTreeSet::from(["refs/tags/v1.2.3".to_owned()]),
            issuer: ISSUER.to_owned(),
            transparency: TransparencyRequirement::PublicLog,
        }
    }

    #[trace("TC-1215", "FR-069-AC-2")]
    #[test]
    fn release_provenance_requires_the_exact_tag_to_peel_to_the_requested_commit() {
        let directory = tempfile::tempdir().expect("source repository directory");
        let git = |arguments: &[&str]| {
            let output = run_fixture_git(directory.path(), arguments);
            assert!(
                output.status.success(),
                "git {arguments:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            output
        };
        git(&["init", "-q"]);
        git(&["config", "user.name", "Fixture"]);
        git(&["config", "user.email", "fixture@example.test"]);
        fs::write(directory.path().join("README.md"), "release\n").expect("source fixture");
        git(&["add", "README.md"]);
        git(&["commit", "-q", "-m", "fixture release"]);
        git(&["tag", "-a", "v1.2.3", "-m", "fixture tag"]);
        let commit = String::from_utf8(git(&["rev-parse", "HEAD"]).stdout)
            .expect("commit id is UTF-8")
            .trim()
            .to_owned();
        assert!(release_tag_points_to_commit(
            directory.path(),
            "v1.2.3",
            &commit
        ));
        assert!(!release_tag_points_to_commit(
            directory.path(),
            "v1.2.3",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        ));
        assert!(!release_tag_points_to_commit(
            directory.path(),
            "--upload-pack=malicious",
            &commit
        ));
    }

    #[trace("TC-1210", "FR-069-AC-5", "FR-069-AC-6")]
    #[test]
    fn execution_provenance_is_authenticated_and_transported_byte_for_byte() {
        let expected = vec![subject("release.tar.gz", &"a".repeat(64))];
        let payload = fixture_statement(expected.clone(), COMMIT);
        let bundle = fixture_bundle(&payload);
        let verifier = FixtureVerifier {
            accepted_payload: payload,
            accepted_signer: true,
        };

        verify_execution_provenance(&bundle, &expected, COMMIT, &signer(), &verifier)
            .expect("trusted and bound provenance verifies");
        let directory = tempfile::tempdir().expect("verified output directory");
        let retained = directory.path().join("verified.dsse.json");
        retain_verified_bundle(&retained, &bundle).expect("write original verified bundle");
        assert_eq!(
            fs::read(&retained).expect("retained bundle is readable"),
            bundle,
            "the boundary forwards the authenticated bundle byte for byte"
        );
        assert!(
            retain_verified_bundle(&retained, b"replacement").is_err(),
            "verified output cannot overwrite an existing path"
        );
    }

    #[trace("TC-1211", "FR-069-AC-2", "FR-069-AC-6")]
    #[test]
    fn execution_provenance_rejects_missing_untrusted_and_mismatched_inputs() {
        let expected = vec![subject("release.tar.gz", &"a".repeat(64))];
        assert_eq!(
            verify_execution_provenance(
                b"",
                &expected,
                COMMIT,
                &signer(),
                &FixtureVerifier {
                    accepted_payload: Vec::new(),
                    accepted_signer: true,
                }
            ),
            Err(ProvenanceFailure::Invalid)
        );

        let payload = fixture_statement(expected.clone(), COMMIT);
        let bundle = fixture_bundle(&payload);
        let untrusted = FixtureVerifier {
            accepted_payload: payload.clone(),
            accepted_signer: false,
        };
        assert_eq!(
            verify_execution_provenance(&bundle, &expected, COMMIT, &signer(), &untrusted),
            Err(ProvenanceFailure::UntrustedSigner)
        );

        let wrong_subject = vec![subject("other.tar.gz", &"a".repeat(64))];
        assert_eq!(
            verify_execution_provenance(
                &bundle,
                &wrong_subject,
                COMMIT,
                &signer(),
                &FixtureVerifier {
                    accepted_payload: payload.clone(),
                    accepted_signer: true,
                }
            ),
            Err(ProvenanceFailure::Binding)
        );
        assert_eq!(
            verify_execution_provenance(
                &bundle,
                &expected,
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                &signer(),
                &FixtureVerifier {
                    accepted_payload: payload,
                    accepted_signer: true,
                }
            ),
            Err(ProvenanceFailure::Binding)
        );
    }
}
