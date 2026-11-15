// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Guard the Engineering Assurance receipt workflow structure.

use std::{fs, path::Path};

use ix_trace_rs::trace;
use serde_json::Value;

mod common;

use common::is_structural_external_action_reference;

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn workflow(name: &str) -> Value {
    let path = repository_root().join(".github/workflows").join(name);
    let bytes = fs::read(path).expect("receipt workflow exists");
    yaml_serde::from_slice(&bytes).expect("receipt workflow parses as YAML")
}

fn assert_permission(job: &Value, expected: &Value, name: &str) {
    assert_eq!(&job["permissions"], expected, "{name} permissions");
}

fn assert_actions_are_structural(steps: &[Value]) {
    for step in steps {
        if let Some(reference) = step["uses"].as_str()
            && !reference.starts_with("./")
        {
            assert!(
                is_structural_external_action_reference(reference),
                "external action reference has a valid owner/repository/path@ref shape: {reference}"
            );
        }
        if let Some(run) = step["run"].as_str() {
            assert!(
                !run.contains("${{"),
                "run scripts receive context through env"
            );
        }
        if step["uses"]
            .as_str()
            .is_some_and(|value| value.starts_with("actions/checkout@"))
        {
            assert_eq!(
                step["with"]["persist-credentials"], false,
                "checkout does not retain credentials"
            );
        }
    }
}

#[trace(
    "TC-559",
    "FR-070-AC-1",
    "FR-070-AC-3",
    "FR-070-AC-4",
    "FR-070-AC-5",
    "FR-075-AC-6"
)]
#[test]
fn receipt_workflow_jobs_have_exact_permissions_and_keep_code_out_of_record_job() {
    let workflow = workflow("change-receipt.yml");
    assert_eq!(
        workflow["on"].as_object().map(serde_json::Map::len),
        Some(1)
    );
    assert!(workflow["on"].get("workflow_call").is_some());
    assert_eq!(
        workflow["on"]["workflow_call"]["inputs"]["commit"]["type"],
        "string"
    );

    let jobs = &workflow["jobs"];
    assert_permission(
        &jobs["gates"],
        &serde_json::json!({"contents":"read"}),
        "gates",
    );
    assert_permission(
        &jobs["record"],
        &serde_json::json!({"contents":"read", "id-token":"write"}),
        "record",
    );
    assert_permission(
        &jobs["notes"],
        &serde_json::json!({"contents":"write"}),
        "notes",
    );

    for name in ["gates", "record", "notes"] {
        let steps = jobs[name]["steps"].as_array().expect("job steps");
        assert_actions_are_structural(steps);
    }
    let record_steps = jobs["record"]["steps"].as_array().expect("record steps");
    assert!(
        record_steps.iter().all(|step| {
            !step["uses"].as_str().is_some_and(|reference| {
                reference.starts_with("actions/checkout@") || reference.starts_with("./")
            })
        }),
        "the identity-bearing record job neither checks out nor loads repository code"
    );
    assert!(record_steps.iter().any(|step| {
        step["run"]
            .as_str()
            .is_some_and(|run| run.contains("statements_not_retained"))
    }));
}

#[trace("TC-559", "FR-070-AC-3")]
#[test]
fn action_reference_structure_rejects_missing_or_malformed_components() {
    for valid in [
        "actions/checkout@v7",
        "owner/repository/path/to/action@v1.2.3",
        "owner/repository@refs/tags/v2",
    ] {
        assert!(
            is_structural_external_action_reference(valid),
            "valid action reference rejected: {valid}"
        );
    }

    for invalid in [
        "@v4",
        "owner/@v4",
        "/repository@v4",
        "owner/repository@",
        "owner//repository@v4",
        "owner/repository@v4@extra",
        "owner/repository@v4 extra",
        "owner/repository@refs//heads/main",
    ] {
        assert!(
            !is_structural_external_action_reference(invalid),
            "malformed action reference accepted: {invalid}"
        );
    }
}

#[trace("FR-070-AC-1", "FR-070-AC-3", "FR-075-AC-1", "FR-076-AC-1")]
#[test]
fn push_caller_and_missing_check_have_the_declared_triggers_and_permissions() {
    let caller = workflow("receipt.yml");
    assert_eq!(caller["on"].as_object().map(serde_json::Map::len), Some(1));
    assert_eq!(caller["on"]["push"]["branches"][0], "main");
    let receipt = &caller["jobs"]["receipt"];
    assert_eq!(receipt["uses"], "./.github/workflows/change-receipt.yml");
    assert_eq!(receipt["with"]["commit"], "${{ github.sha }}");
    assert_permission(
        receipt,
        &serde_json::json!({"contents":"write", "id-token":"write"}),
        "receipt caller",
    );

    let check = workflow("missing-receipt-check.yml");
    assert_eq!(check["on"].as_object().map(serde_json::Map::len), Some(2));
    assert!(check["on"].get("schedule").is_some());
    assert!(check["on"].get("workflow_dispatch").is_some());
    assert_eq!(
        check["jobs"]["check"]["permissions"],
        serde_json::json!({"contents":"read"})
    );
    let steps = check["jobs"]["check"]["steps"]
        .as_array()
        .expect("check steps");
    assert_actions_are_structural(steps);
}
