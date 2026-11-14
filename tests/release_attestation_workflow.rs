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

#[trace("TC-557", "FR-070-AC-1")]
#[test]
fn engineering_assurance_tag_caller_builds_then_calls_the_matching_release_tag() {
    let workflow = workflow();
    assert_eq!(
        workflow["on"]["push"]["tags"][0],
        "v*",
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
    assert_eq!(attest["with"]["record_path"], "spec/releases/engineering-assurance.md");
    assert_eq!(
        attest["with"]["allowed_signers_path"],
        ".engineering-assurance/allowed_signers"
    );
    assert!(attest.get("steps").is_none(), "attest job only calls the reusable workflow");
}
