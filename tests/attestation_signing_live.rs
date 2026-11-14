// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Opt-in public-good Sigstore verification run by the manual CI workflow.

use std::{env, fs};

use engineering_assurance::attestation_signing::{
    ExpectedSubject, SigstoreBundleVerifier, SubjectKind, TransparencyRequirement,
    VerificationStatus, WorkflowIdentityPolicy, verify_statement,
};

/// This live test is run only by `.github/workflows/ea-106-live.yml` after a
/// separate OIDC-enabled job signs the fixture through the public-good service.
#[tokio::test]
#[ignore = "manual workflow_dispatch public-good Sigstore integration"]
async fn verifies_public_good_sigstore_bundle_and_artifact_digest() {
    let payload_path = env::var("EA_LIVE_PAYLOAD_PATH").expect("payload path is configured");
    let bundle_path = env::var("EA_LIVE_BUNDLE_PATH").expect("bundle path is configured");
    let expected_digest =
        env::var("EA_LIVE_SHA256").expect("expected artifact digest is configured");
    let payload = fs::read(payload_path).expect("signed statement payload is available");
    let bundle = fs::read_to_string(bundle_path).expect("public-good bundle is available");
    let trusted_root = sigstore_verify::trust_root::TrustedRoot::production()
        .await
        .expect("public-good trusted root is available");
    let verifier = SigstoreBundleVerifier::new(&trusted_root)
        .expect("Sigstore verifier accepts the public-good root");
    let policy = WorkflowIdentityPolicy {
        repository: "agent-ix/engineering-assurance".to_owned(),
        workflow_path: ".github/workflows/ea-106-live.yml".to_owned(),
        references: ["refs/heads/main".to_owned()].into(),
        issuer: concat!("https:", "//token.actions.githubusercontent.com").to_owned(),
        transparency: TransparencyRequirement::PublicLog,
    };

    assert_eq!(
        verify_statement(
            &verifier,
            &payload,
            &bundle,
            &policy,
            ExpectedSubject {
                kind: SubjectKind::Sha256,
                value: &expected_digest,
            },
        ),
        Ok(VerificationStatus::Verified)
    );
}
