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

#[trace("TC-1003", "FR-064-AC-29")]
#[test]
fn a_receipt_writer_with_a_source_ref_prefix_is_a_tag_receipt_identity() {
    use engineering_assurance::attestation_signing::TransparencyRequirement;

    let tag_entry = |workflow: &str| {
        json!({
            "role": "receipt_writer",
            "issuer": TEST_OIDC_ISSUER,
            "repository": "agent-ix/engineering-assurance",
            "workflow_path": workflow,
            "ref": "refs/heads/main",
            "source_repository": "fictional/project",
            "source_workflow_path": ".github/workflows/tag-receipt-push.yml",
            "event": "push",
            "source_ref_prefix": "refs/tags/approve-"
        })
    };
    let change: Value = serde_json::from_str(&receipt_identity_document()).expect("fixture");
    let mut both = change.clone();
    both["identities"]
        .as_array_mut()
        .expect("identities")
        .push(tag_entry(".github/workflows/tag-receipt.yml"));
    let document =
        WorkflowIdentityDocument::parse(&serde_json::to_vec(&both).expect("bytes"))
            .expect("one entry of each kind is valid");
    let signers = document.tag_receipt_signers(TransparencyRequirement::PublicLog);
    assert_eq!(signers.len(), 1, "only the prefix entry signs tag receipts");
    assert_eq!(
        signers[0].0.workflow_path,
        ".github/workflows/tag-receipt.yml"
    );
    assert_eq!(
        signers[0].1.source_ref_prefix.as_deref(),
        Some("refs/tags/approve-")
    );
    assert!(signers[0].1.source_ref.is_none());

    // A prefix that is not a tag-ref prefix is refused.
    let mut branch_prefix = tag_entry(".github/workflows/tag-receipt.yml");
    branch_prefix["source_ref_prefix"] = json!("refs/heads/release-");
    let mut refused = change;
    refused["identities"]
        .as_array_mut()
        .expect("identities")
        .push(branch_prefix);
    assert!(
        WorkflowIdentityDocument::parse(&serde_json::to_vec(&refused).expect("bytes")).is_err()
    );
}
