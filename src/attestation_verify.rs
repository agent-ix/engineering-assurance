// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Verification of the fixed release-attestation set (FR-066, FR-068).
//!
//! Signature and certificate work is delegated to the shared EA-106 verifier.
//! This module checks the authenticated statements' exact subjects and the
//! standard verification-summary predicate; it implements no cryptography.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::attestation_signing::{
    BundleVerifier, ExpectedSubject, SourceWorkflowPolicy, StatementV1, SubjectKind,
    VerificationFailure, VerificationStatus, WorkflowIdentityPolicy, verify_statement_with_source,
};

/// SLSA Provenance v1 predicate type.
pub const PROVENANCE_PREDICATE: &str = concat!("https:", "//slsa.dev/provenance/v1");
/// `CycloneDX` BOM predicate type.
pub const CYCLONEDX_PREDICATE: &str = concat!("https:", "//cyclonedx.org/bom");
/// in-toto Test Result v0.1 predicate type.
pub const TEST_RESULT_PREDICATE: &str =
    concat!("https:", "//in-toto.io/attestation/test-result/v0.1");
/// SLSA Verification Summary v1 predicate type.
pub const VERIFICATION_SUMMARY_PREDICATE: &str =
    concat!("https:", "//slsa.dev/verification_summary/v1");

/// A signed in-toto statement and its Sigstore bundle.
#[derive(Clone, Copy, Debug)]
pub struct SignedStatement<'a> {
    /// The statement payload bytes authenticated by the bundle.
    pub payload: &'a [u8],
    /// The Sigstore bundle JSON.
    pub bundle_json: &'a str,
}

/// One artifact name and its computed SHA-256 subject digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactSubject {
    /// The in-toto subject name.
    pub name: String,
    /// The lowercase SHA-256 digest computed by the caller.
    pub sha256: String,
}

/// The three fixed input attestations, in provenance, SBOM, Test Result order.
#[derive(Clone, Copy, Debug)]
pub struct ReleaseInputs<'a> {
    /// The SLSA Provenance v1 statement.
    pub provenance: SignedStatement<'a>,
    /// The `CycloneDX` statement.
    pub sbom: SignedStatement<'a>,
    /// The Test Result v0.1 statement.
    pub test_result: SignedStatement<'a>,
}

/// Expected release identity and content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedRelease {
    /// The source repository in `owner/name` form.
    pub repository: String,
    /// The signed tag name, without `refs/tags/`.
    pub tag: String,
    /// The release commit named by the tag.
    pub commit: String,
    /// The protected role-policy URI named by the summary.
    pub policy_uri: String,
    /// The expected identity URI of the reusable verifier workflow.
    pub verifier_id: String,
    /// Each artifact file and its computed digest.
    pub artifacts: Vec<ArtifactSubject>,
    /// RFC 8785 digests of provenance, SBOM and Test Result statement bytes.
    pub input_digests: [String; 3],
}

/// Result of evaluating the three required conditions for `PASSED`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerificationConditions {
    /// `release-verify` reported `released`.
    pub release_verified: bool,
    /// The role evaluation was satisfied.
    pub roles_satisfied: bool,
    /// All three input attestations verified.
    pub inputs_verified: bool,
}

/// The standard SLSA verification result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationResult {
    /// Every verification condition held.
    Passed,
    /// At least one verification condition failed.
    Failed,
}

/// Error returned when a signed release-attestation set is invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum AttestationVerificationError {
    /// A signed statement is not valid under the delegated signer policy.
    #[error("statement signature or signer verification failed")]
    Signature(#[from] VerificationFailure),
    /// A statement does not use the required standard predicate.
    #[error("attestation predicate is invalid")]
    PredicateInvalid,
    /// The statement subjects differ from the expected release artifacts.
    #[error("attestation subjects do not match the release")]
    WrongSubject,
    /// The summary predicate does not match its expected release facts.
    #[error("verification summary does not match the release")]
    SummaryInvalid,
    /// The release did not satisfy all conditions required for PASSED.
    #[error("release verification conditions are not all satisfied")]
    NotPassed,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationSummaryStatement {
    #[serde(rename = "_type")]
    type_: String,
    subject: Vec<SummarySubject>,
    #[serde(rename = "predicateType")]
    predicate_type: String,
    predicate: VerificationSummaryPredicate,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SummarySubject {
    name: String,
    digest: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationSummaryPredicate {
    verifier: VerifierIdentity,
    #[serde(rename = "timeVerified")]
    time_verified: String,
    #[serde(rename = "resourceUri")]
    resource_uri: String,
    policy: PolicyIdentity,
    #[serde(rename = "inputAttestations")]
    input_attestations: Vec<InputAttestation>,
    #[serde(rename = "verificationResult")]
    verification_result: String,
    #[serde(rename = "verifiedLevels")]
    verified_levels: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifierIdentity {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyIdentity {
    uri: String,
}

#[derive(Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
struct InputAttestation {
    uri: String,
    digest: InputDigest,
}

#[derive(Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
struct InputDigest {
    sha256: String,
}

/// Return `PASSED` only when all three required checks succeeded.
#[must_use]
pub const fn result_for(conditions: VerificationConditions) -> VerificationResult {
    if conditions.release_verified && conditions.roles_satisfied && conditions.inputs_verified {
        VerificationResult::Passed
    } else {
        VerificationResult::Failed
    }
}

/// Verify the three input statements, their artifact subjects, and a signed
/// SLSA Verification Summary against the release and signer policies.
///
/// The canonical statement digests in `expected` are calculated by the caller
/// using its approved SHA-256 provider. Every signature and workflow claim is
/// authenticated by `verifier`, which must be the shared EA-106 verifier.
///
/// # Errors
///
/// Returns a typed failure for invalid predicates or subjects, an untrusted
/// source workflow, an invalid summary, or an unmet release condition.
pub fn verify_release_attestation_set(
    verifier: &dyn BundleVerifier,
    signer_policy: &WorkflowIdentityPolicy,
    source_policy: &SourceWorkflowPolicy,
    expected: &ExpectedRelease,
    conditions: VerificationConditions,
    inputs: ReleaseInputs<'_>,
    summary: SignedStatement<'_>,
) -> Result<VerificationStatus, AttestationVerificationError> {
    if result_for(conditions) != VerificationResult::Passed {
        return Err(AttestationVerificationError::NotPassed);
    }

    let required = [
        (inputs.provenance, PROVENANCE_PREDICATE),
        (inputs.sbom, CYCLONEDX_PREDICATE),
        (inputs.test_result, TEST_RESULT_PREDICATE),
    ];
    for (document, predicate_type) in required {
        let statement = parse_statement(document.payload)?;
        if statement.predicate_type != predicate_type {
            return Err(AttestationVerificationError::PredicateInvalid);
        }
        check_artifact_subjects(&statement, &expected.artifacts)?;
        for artifact in &expected.artifacts {
            verify_statement_with_source(
                verifier,
                document.payload,
                document.bundle_json,
                signer_policy,
                Some(source_policy),
                ExpectedSubject {
                    kind: SubjectKind::Sha256,
                    value: &artifact.sha256,
                },
            )?;
        }
    }

    let summary_statement: VerificationSummaryStatement =
        serde_json::from_slice(summary.payload)
            .map_err(|_| AttestationVerificationError::PredicateInvalid)?;
    if summary_statement.type_ != crate::attestation_signing::STATEMENT_V1_TYPE
        || summary_statement.predicate_type != VERIFICATION_SUMMARY_PREDICATE
    {
        return Err(AttestationVerificationError::PredicateInvalid);
    }
    check_summary_subjects(&summary_statement.subject, expected)?;
    verify_statement_with_source(
        verifier,
        summary.payload,
        summary.bundle_json,
        signer_policy,
        Some(source_policy),
        ExpectedSubject {
            kind: SubjectKind::Commit,
            value: &expected.commit,
        },
    )?;
    validate_summary(&summary_statement.predicate, expected)?;
    Ok(VerificationStatus::Verified)
}

fn parse_statement(payload: &[u8]) -> Result<StatementV1, AttestationVerificationError> {
    let statement: StatementV1 = serde_json::from_slice(payload)
        .map_err(|_| AttestationVerificationError::PredicateInvalid)?;
    if statement.type_ != crate::attestation_signing::STATEMENT_V1_TYPE {
        return Err(AttestationVerificationError::PredicateInvalid);
    }
    Ok(statement)
}

fn check_artifact_subjects(
    statement: &StatementV1,
    expected: &[ArtifactSubject],
) -> Result<(), AttestationVerificationError> {
    let actual: BTreeSet<(&str, &str)> = statement
        .subject
        .iter()
        .filter_map(|subject| {
            subject
                .digest
                .get("sha256")
                .map(|digest| (subject.name.as_str(), digest.as_str()))
        })
        .collect();
    let expected: BTreeSet<(&str, &str)> = expected
        .iter()
        .map(|subject| (subject.name.as_str(), subject.sha256.as_str()))
        .collect();
    if actual != expected || statement.subject.len() != expected.len() {
        return Err(AttestationVerificationError::WrongSubject);
    }
    Ok(())
}

fn check_summary_subjects(
    actual: &[SummarySubject],
    expected: &ExpectedRelease,
) -> Result<(), AttestationVerificationError> {
    let wanted: BTreeSet<(String, BTreeMap<String, String>)> = expected
        .artifacts
        .iter()
        .map(|artifact| {
            (
                artifact.name.clone(),
                [("sha256".to_owned(), artifact.sha256.clone())].into(),
            )
        })
        .collect();
    let commit_subjects: Vec<&SummarySubject> = actual
        .iter()
        .filter(|subject| {
            subject.digest.len() == 1 && subject.digest.get("gitCommit") == Some(&expected.commit)
        })
        .collect();
    if commit_subjects.len() != 1 {
        return Err(AttestationVerificationError::WrongSubject);
    }
    let artifact_subjects: Vec<&SummarySubject> = actual
        .iter()
        .filter(|subject| subject.digest.contains_key("sha256"))
        .collect();
    let observed: BTreeSet<(String, BTreeMap<String, String>)> = artifact_subjects
        .iter()
        .map(|subject| (subject.name.clone(), subject.digest.clone()))
        .collect();
    if artifact_subjects.len() != expected.artifacts.len()
        || observed != wanted
        || actual.len() != artifact_subjects.len() + 1
    {
        return Err(AttestationVerificationError::WrongSubject);
    }
    Ok(())
}

fn validate_summary(
    predicate: &VerificationSummaryPredicate,
    expected: &ExpectedRelease,
) -> Result<(), AttestationVerificationError> {
    let expected_resource = format!(
        "{}{}github.com/{}/tree/{}",
        "https:", "//", expected.repository, expected.tag
    );
    if predicate.verifier.id != expected.verifier_id
        || predicate.time_verified.is_empty()
        || predicate.resource_uri != expected_resource
        || predicate.policy.uri != expected.policy_uri
        || predicate.verification_result != "PASSED"
        || !predicate.verified_levels.is_empty()
    {
        return Err(AttestationVerificationError::SummaryInvalid);
    }
    let mut wanted: Vec<InputAttestation> = [
        (PROVENANCE_PREDICATE, &expected.input_digests[0]),
        (CYCLONEDX_PREDICATE, &expected.input_digests[1]),
        (TEST_RESULT_PREDICATE, &expected.input_digests[2]),
    ]
    .into_iter()
    .map(|(name, digest)| InputAttestation {
        uri: name.to_owned(),
        digest: InputDigest {
            sha256: digest.clone(),
        },
    })
    .collect();
    let mut observed = predicate.input_attestations.iter().collect::<Vec<_>>();
    wanted.sort();
    observed.sort();
    if observed.len() != wanted.len()
        || observed
            .iter()
            .zip(wanted.iter())
            .any(|(actual, expected)| **actual != *expected)
    {
        return Err(AttestationVerificationError::SummaryInvalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use serde_json::json;

    fn expected() -> ExpectedRelease {
        ExpectedRelease {
            repository: "fictional/release".to_owned(),
            tag: "v1.2.3".to_owned(),
            commit: "0123456789012345678901234567890123456789".to_owned(),
            policy_uri: "fictional-policy".to_owned(),
            verifier_id: "fictional-workflow".to_owned(),
            artifacts: vec![ArtifactSubject {
                name: "release.tar.gz".to_owned(),
                sha256: "a".repeat(64),
            }],
            input_digests: ["b".repeat(64), "c".repeat(64), "d".repeat(64)],
        }
    }

    fn summary_json() -> serde_json::Value {
        json!({
            "_type": crate::attestation_signing::STATEMENT_V1_TYPE,
            "subject": [
                {"name":"release.tar.gz", "digest":{"sha256":"a".repeat(64)}},
                {
                    "name":"release-tag",
                    "digest":{"gitCommit":"0123456789012345678901234567890123456789"}
                }
            ],
            "predicateType": VERIFICATION_SUMMARY_PREDICATE,
            "predicate": {
                "verifier": {"id":"fictional-workflow"},
                "timeVerified":"2026-11-13T10:00:00Z",
                "resourceUri":concat!("https:", "//github.com/fictional/release/tree/v1.2.3"),
                "policy":{"uri":"fictional-policy"},
                "inputAttestations":[
                    {"uri":PROVENANCE_PREDICATE, "digest":{"sha256":"b".repeat(64)}},
                    {"uri":CYCLONEDX_PREDICATE, "digest":{"sha256":"c".repeat(64)}},
                    {"uri":TEST_RESULT_PREDICATE, "digest":{"sha256":"d".repeat(64)}}
                ],
                "verificationResult":"PASSED",
                "verifiedLevels":[]
            }
        })
    }

    #[test]
    #[trace("TC-673", "FR-068-AC-2")]
    fn tc_673_passed_requires_all_three_conditions() {
        for release_verified in [false, true] {
            for roles_satisfied in [false, true] {
                for inputs_verified in [false, true] {
                    let all = release_verified && roles_satisfied && inputs_verified;
                    assert_eq!(
                        result_for(VerificationConditions {
                            release_verified,
                            roles_satisfied,
                            inputs_verified,
                        }) == VerificationResult::Passed,
                        all
                    );
                }
            }
        }
    }

    #[test]
    #[trace("TC-674", "FR-068-AC-4")]
    fn tc_674_summary_binds_three_input_statements_by_canonical_digest() {
        let expected = expected();
        let statement: VerificationSummaryStatement =
            serde_json::from_value(summary_json()).expect("closed summary fixture");
        assert_eq!(validate_summary(&statement.predicate, &expected), Ok(()));

        let mut changed = summary_json();
        changed["predicate"]["inputAttestations"][0]["digest"]["sha256"] = json!("e".repeat(64));
        let statement: VerificationSummaryStatement =
            serde_json::from_value(changed).expect("schema remains valid");
        assert_eq!(
            validate_summary(&statement.predicate, &expected),
            Err(AttestationVerificationError::SummaryInvalid)
        );
    }

    #[test]
    #[trace("TC-675", "FR-068-AC-5", "FR-068-AC-6")]
    fn tc_675_summary_binds_tag_commit_and_closed_empty_levels() {
        let expected = expected();
        let statement: VerificationSummaryStatement =
            serde_json::from_value(summary_json()).expect("closed summary fixture");
        check_summary_subjects(&statement.subject, &expected).expect("release commit subject");
        assert!(statement.predicate.verified_levels.is_empty());

        let mut changed_commit = summary_json();
        changed_commit["subject"][1]["digest"]["gitCommit"] = json!("other-commit");
        let statement: VerificationSummaryStatement =
            serde_json::from_value(changed_commit).expect("schema remains valid");
        assert_eq!(
            check_summary_subjects(&statement.subject, &expected),
            Err(AttestationVerificationError::WrongSubject)
        );

        let mut extra = summary_json();
        extra["predicate"]["unexpected"] = json!(true);
        assert!(serde_json::from_value::<VerificationSummaryStatement>(extra).is_err());
    }
}
