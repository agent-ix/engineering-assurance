// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Guard the Engineering Assurance release caller's workflow contract.

use std::{fs, path::Path};

use ix_trace_rs::trace;
use serde_json::Value;

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn workflow() -> Value {
    let path = repository_root().join(".github/workflows/release.yml");
    let bytes = fs::read(&path).expect("release caller workflow exists");
    yaml_serde::from_slice(&bytes).expect("release caller workflow parses as YAML")
}

fn reusable_workflow() -> Value {
    let path = repository_root().join(".github/workflows/release-attestation.yml");
    let bytes = fs::read(&path).expect("release-attestation reusable workflow exists");
    yaml_serde::from_slice(&bytes).expect("release-attestation reusable workflow parses as YAML")
}

#[trace("TC-557", "FR-070-AC-1")]
#[test]
fn engineering_assurance_tag_caller_builds_then_calls_the_matching_release_tag() {
    let workflow = workflow();
    assert_eq!(
        workflow["on"]["push"]["tags"][0], "v*",
        "the caller runs only for release tags"
    );

    let jobs = &workflow["jobs"];
    let build = &jobs["build"];
    assert_eq!(build["permissions"]["contents"], "read");
    let steps = build["steps"].as_array().expect("build steps");
    assert!(steps.iter().any(|step| {
        step["uses"]
            .as_str()
            .is_some_and(|reference| reference.starts_with("actions/checkout@"))
            && step["with"]["persist-credentials"] == false
    }));
    assert!(steps.iter().any(|step| {
        step["uses"]
            .as_str()
            .is_some_and(|reference| reference.starts_with("actions/upload-artifact@"))
            && step["with"]["name"] == "release-artifacts"
    }));

    let attest = &jobs["attest"];
    assert_eq!(attest["needs"], "build");
    assert_eq!(
        attest["permissions"],
        serde_json::json!({
            "contents":"read",
            "id-token":"write",
            "attestations":"write"
        })
    );
    assert_eq!(
        attest["uses"],
        "agent-ix/engineering-assurance/.github/workflows/release-attestation.yml@v0.9.0"
    );
    assert_eq!(attest["with"]["tag"], "${{ github.ref_name }}");
    assert_eq!(attest["with"]["ea_tooling_tag"], "v0.9.0");
    assert_eq!(
        attest["with"]["record_path"],
        "spec/releases/engineering-assurance.md"
    );
    assert_eq!(
        attest["with"]["allowed_signers_path"],
        ".engineering-assurance/allowed_signers"
    );
    assert!(
        attest.get("steps").is_none(),
        "attest job only calls the reusable workflow"
    );
}

#[trace("TC-562", "FR-070-AC-6", "FR-067-AC-7")]
#[test]
fn reusable_release_workflow_checks_tag_and_fails_closed_before_signing_without_statement_source() {
    let workflow = reusable_workflow();
    let inputs = &workflow["on"]["workflow_call"]["inputs"];
    assert_eq!(
        inputs,
        &serde_json::json!({
            "tag": {"required": true, "type": "string"},
            "ea_tooling_tag": {"required": true, "type": "string"},
            "record_path": {"required": true, "type": "string"},
            "allowed_signers_path": {"required": true, "type": "string"},
            "artifact_name": {"required": false, "type": "string", "default": "release-artifacts"},
            "evidence_artifact": {"required": false, "type": "string", "default": "release-evidence"}
        })
    );

    let jobs = &workflow["jobs"];
    let verify = &jobs["verify"];
    assert_eq!(
        verify["permissions"],
        serde_json::json!({"contents": "read"})
    );
    assert_eq!(
        verify["outputs"]["attestation_inputs_artifact"],
        "${{ steps.attestation-artifact-name.outputs.name }}"
    );
    let verify_steps = verify["steps"].as_array().expect("verify steps");
    assert!(verify_steps.iter().any(|step| {
        step["id"] == "attestation-artifact-name"
            && step["run"]
                .as_str()
                .is_some_and(|run| run.contains("secrets.token_hex(32)"))
    }));
    let tag_check = verify_steps
        .iter()
        .position(|step| step["name"] == "Check the triggering release tag")
        .expect("trigger tag check exists");
    let release_check = verify_steps
        .iter()
        .position(|step| step["name"] == "Verify the signed release tag against protected policy")
        .expect("release-verify check exists");
    let role_source_check = verify_steps
        .iter()
        .position(|step| step["name"] == "Refuse without a trusted approval-statement source")
        .expect("missing statement source refuses");
    assert!(tag_check < release_check && release_check < role_source_check);
    assert!(verify_steps.iter().any(|step| {
        step["run"]
            .as_str()
            .is_some_and(|run| run.contains("release_input_tag_mismatch"))
    }));
    assert!(verify_steps.iter().any(|step| {
        step["run"]
            .as_str()
            .is_some_and(|run| run.contains("no_retained_evidence_store"))
    }));
    assert!(verify_steps.iter().any(|step| {
        step["run"]
            .as_str()
            .is_some_and(|run| run.contains("release_roles_source_unavailable"))
    }));
    for step in verify_steps {
        if let Some(run) = step["run"].as_str() {
            assert!(!run.contains("${{"), "run values use env for expressions");
        }
    }

    let attest = &jobs["attest"];
    assert_eq!(attest["needs"], "verify");
    assert_eq!(
        attest["permissions"],
        serde_json::json!({
            "contents": "read",
            "id-token": "write",
            "attestations": "write"
        })
    );
    let attest_steps = attest["steps"].as_array().expect("attest steps");
    assert_eq!(attest_steps.len(), 6);
    for step in attest_steps {
        let reference = step["uses"].as_str().expect("pinned action reference");
        let (_, revision) = reference.split_once('@').expect("action has ref");
        assert_eq!(revision.len(), 40, "action is pinned by full commit SHA");
        assert!(revision.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
    assert!(attest_steps.iter().all(|step| step.get("run").is_none()));
    assert!(
        attest_steps
            .iter()
            .filter(|step| step["uses"]
                .as_str()
                .is_some_and(|name| name.starts_with("actions/attest@")))
            .all(|step| step["with"]["create-storage-record"] == false)
    );
    assert!(attest_steps.iter().any(|step| {
        step["with"]["predicate-type"] == concat!("https:", "//slsa.dev/verification_summary/v1")
    }));
    assert_eq!(
        attest_steps[1]["with"]["name"],
        "${{ needs.verify.outputs.attestation_inputs_artifact }}"
    );
}
