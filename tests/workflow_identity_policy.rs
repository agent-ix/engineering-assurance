// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Public workflow-identity policy parsing contracts.

use engineering_assurance::attestation_signing::WorkflowIdentityDocument;
use ix_trace_rs::trace;
use serde_json::{Value, json};

const TEST_OIDC_ISSUER: &str = concat!("https:", "//", "token.actions.githubusercontent.com");

fn release_identity_document() -> String {
    json!({
        "protocol": "engineering-assurance.workflow-identities/v1",
        "identities": [{
            "role": "release_attester",
            "issuer": TEST_OIDC_ISSUER,
            "repository": "agent-ix/engineering-assurance",
            "workflow_path": ".github/workflows/release-attestation.yml",
            "ref": "refs/tags/v1.2.3",
            "source_repository": "fictional/product",
            "source_workflow_path": ".github/workflows/release.yml",
            "event": "push",
            "source_ref_prefix": "refs/tags/v"
        }]
    })
    .to_string()
}

#[trace("TC-677", "FR-064-AC-9", "FR-064-AC-27")]
#[test]
fn workflow_identity_document_rejects_open_or_malformed_entries() {
    let valid = release_identity_document();
    for invalid in [
        valid.replace(
            "\"source_ref_prefix\":\"refs/tags/v\"",
            "\"source_ref_prefix\":\"refs/pull/\"",
        ),
        valid.replace(
            "\"source_ref_prefix\":\"refs/tags/v\"",
            "\"source_ref_prefix\":\"refs/tags/v*\"",
        ),
        valid.replace(
            "\"ref\":\"refs/tags/v1.2.3\"",
            "\"ref\":\"refs/heads/main\"",
        ),
        valid.replace(
            "\"source_ref_prefix\":\"refs/tags/v\"",
            "\"source_ref_prefix\":null",
        ),
        valid.replace(
            "\"source_ref_prefix\":\"refs/tags/v\"",
            "\"source_ref_prefix\":\"refs/tags/v\",\"unexpected\":true",
        ),
        valid.replace(
            "\"source_ref_prefix\":\"refs/tags/v\"",
            "\"source_ref_prefix\":\"refs/tags/v\",\"source_ref\":\"refs/heads/main\"",
        ),
    ] {
        assert!(
            WorkflowIdentityDocument::parse(invalid.as_bytes()).is_err(),
            "invalid policy unexpectedly accepted: {invalid}"
        );
    }
    let mut missing_source: Value = serde_json::from_str(&valid).expect("valid fixture JSON");
    missing_source["identities"][0]
        .as_object_mut()
        .expect("identity object")
        .remove("source_repository");
    assert!(
        WorkflowIdentityDocument::parse(&serde_json::to_vec(&missing_source).expect("JSON bytes"))
            .is_err(),
        "identity without source_repository must be refused"
    );
}

fn receipt_identity_document() -> String {
    json!({
        "protocol": "engineering-assurance.workflow-identities/v1",
        "identities": [{
            "role": "receipt_writer",
            "issuer": TEST_OIDC_ISSUER,
            "repository": "agent-ix/engineering-assurance",
            "workflow_path": ".github/workflows/change-receipt.yml",
            "ref": "refs/heads/main",
            "source_repository": "fictional/project",
            "source_workflow_path": ".github/workflows/change-receipt.yml",
            "event": "push",
            "source_ref": "refs/heads/main"
        }]
    })
    .to_string()
}

#[trace("TC-1003", "FR-064-AC-28")]
#[test]
fn receipt_writer_policy_requires_source_repository_and_one_source_ref_constraint() {
    let valid = receipt_identity_document();
    let missing_source = valid.replace("\"source_repository\":\"fictional/project\",", "");
    assert!(
        WorkflowIdentityDocument::parse(missing_source.as_bytes()).is_err(),
        "receipt_writer without source_repository must be refused"
    );
    let both_source_refs = valid.replace(
        "\"source_ref\":\"refs/heads/main\"",
        "\"source_ref\":\"refs/heads/main\",\"source_ref_prefix\":\"refs/heads/\"",
    );
    assert!(
        WorkflowIdentityDocument::parse(both_source_refs.as_bytes()).is_err(),
        "receipt_writer with source_ref and source_ref_prefix must be refused"
    );
}
