// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Guard the Engineering Assurance receipt workflow structure.

use std::{fs, path::Path, process::Command};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use ix_trace_rs::trace;
use serde_json::{Value, json};

mod common;

use common::is_structural_action_reference;

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
        if let Some(reference) = step["uses"].as_str() {
            assert!(
                is_structural_action_reference(reference),
                "action reference has a valid local or external structure: {reference}"
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

fn assert_record_job_is_isolated(jobs: &Value) {
    let record_steps = jobs["record"]["steps"].as_array().expect("record steps");
    assert!(
        record_steps.iter().all(|step| {
            !step["uses"].as_str().is_some_and(|reference| {
                reference.starts_with("actions/checkout@") || reference.starts_with("./")
            })
        }),
        "the identity-bearing record job neither checks out nor loads repository code"
    );
    assert!(
        record_steps.iter().any(|step| {
            step["run"]
                .as_str()
                .is_some_and(|run| run.contains("receipt-record --request"))
        }),
        "record job invokes the published receipt-record command"
    );
    assert!(
        record_steps.iter().any(|step| {
            step["uses"]
                .as_str()
                .is_some_and(|reference| reference.starts_with("actions/download-artifact@"))
        }),
        "record job receives gate outputs and Test Result as data"
    );
    assert!(
        record_steps.iter().all(|step| {
            !step["run"]
                .as_str()
                .is_some_and(|run| run.contains("cargo test") || run.contains("python3 -"))
        }),
        "record job does not execute repository gates or inline repository scripts"
    );
}

fn assert_gate_job_boundary(jobs: &Value) -> String {
    let gate_steps = jobs["gates"]["steps"].as_array().expect("gate steps");
    let gate_text = gate_steps
        .iter()
        .filter_map(|step| step["run"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(gate_text.contains("producer-execute"));
    assert!(gate_text.contains("evidence record --suite SUITE-001"));
    assert!(gate_text.contains("evidence export-test-result"));
    assert!(gate_text.contains("receipt-gate-results/v1"));
    assert!(
        gate_text.contains("TC-560"),
        "the executed gate carries its declared TC-560 trace"
    );
    assert!(
        gate_text.contains("exitCode"),
        "gate outcomes use the published receipt camelCase contract"
    );
    assert!(gate_text.contains(" archive \"$INPUT_COMMIT\""));
    assert!(gate_text.contains("sha256sum"));
    assert!(jobs["gates"]["permissions"].get("id-token").is_none());
    assert!(jobs["gates"].get("secrets").is_none());
    assert!(jobs["gates"].get("environment").is_none());
    let runner = jobs["gates"]["runs-on"].as_str().expect("declared runner");
    assert!(
        ["ubuntu-", "macos-", "windows-"]
            .iter()
            .any(|prefix| runner.starts_with(prefix)),
        "gate job uses a GitHub-hosted runner"
    );
    gate_text
}

fn assert_gate_failure_exports_data_before_failing(gate_text: &str, jobs: &Value) {
    let export_at = gate_text
        .find("evidence export-test-result")
        .expect("the gates job exports Quoin Test Result data");
    let fail_after_export_at = gate_text
        .find("if [[ -e \"$RUNNER_TEMP/receipt-gate-failed\" ]]")
        .expect("the gate failure is reported after the export");
    assert!(export_at < fail_after_export_at);
    assert!(
        jobs["record"]["if"]
            .as_str()
            .is_some_and(|value| value == "always()")
    );
    assert!(
        jobs["gates"]["steps"]
            .as_array()
            .is_some_and(|steps| steps.iter().any(|step| {
                step["uses"]
                    .as_str()
                    .is_some_and(|value| value.starts_with("actions/upload-artifact@"))
                    && step["if"] == "always()"
            }))
    );
}

fn assert_receipt_policy_and_suite() {
    let policy_path = repository_root().join(".engineering-assurance/receipt-policy.json");
    let policy: Value =
        serde_json::from_slice(&fs::read(policy_path).expect("receipt policy exists"))
            .expect("receipt policy parses as JSON");
    assert_eq!(
        policy["protocol"],
        "engineering-assurance.receipt-policy/v1"
    );
    assert_eq!(policy["gates"].as_array().map(Vec::len), Some(1));
    assert_eq!(policy["gates"][0]["name"], "receipt-workflow-contract");
    assert_eq!(policy["gates"][0]["argv"][0], "cargo");
    let gate_command = policy["gates"][0]["argv"]
        .as_array()
        .expect("receipt gate argv")
        .iter()
        .map(|argument| argument.as_str().expect("receipt gate argument"))
        .collect::<Vec<_>>()
        .join(" ");
    let workflow = workflow("change-receipt.yml");
    let record_steps = workflow["jobs"]["record"]["steps"]
        .as_array()
        .expect("record steps");
    assert!(
        record_steps.iter().all(|step| {
            !step["run"]
                .as_str()
                .is_some_and(|run| run.contains(&gate_command))
        }),
        "record job does not execute a configured receipt-policy command"
    );
    let suites = fs::read_to_string(repository_root().join("spec/evidence/suites.md"))
        .expect("suite registry exists");
    assert!(suites.contains("| SUITE-001 | Receipt workflow contract gate |"));
    assert!(suites.contains("| Integration |"));
}

#[cfg(unix)]
fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write fixture executable");
    let mut permissions = fs::metadata(path).expect("fixture metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make fixture executable");
}

#[cfg(unix)]
fn producer_request(root: &Path, path: &Path, marker: &Path, policy: &Value) -> Value {
    let argv = policy["gates"][0]["argv"]
        .as_array()
        .expect("fixture policy argv")
        .iter()
        .map(|argument| argument.as_str().expect("fixture argv string"))
        .collect::<Vec<_>>();
    let executable = argv.first().expect("fixture policy executable");
    json!({
        "protocol": "engineering-assurance.producer-execution-request/v1",
        "producer": {"name": "receipt-adversarial-gate", "version": "1", "executable": executable},
        "caller": {"kind": "fixture.receipt-gate", "version": "1"},
        "procedure": "direct",
        "capabilityRoot": fs::canonicalize(root).expect("canonical fixture root").display().to_string(),
        "arguments": argv.iter().skip(1).map(|value| json!({"kind":"literal", "value":value})).collect::<Vec<_>>(),
        "environment": {
            "PATH": format!("{}:/usr/bin:/bin", path.display()),
            "FIXTURE_MARKER_PATH": marker.display().to_string()
        },
        "inputs": [],
        "stdin": {"kind": "null"},
        "outputs": [],
        "containment": {"profile": "process-group-v1", "contract": {"kind": "fixture.receipt-gate", "version": "1"}},
        "cancellation": {"kind": "disabled"},
        "budget": {
            "timeoutMillis": 5000,
            "maxStdoutBytes": 1024,
            "maxStderrBytes": 1024,
            "maxInputBytes": 1024,
            "maxOutputArtifacts": 0,
            "maxOutputBytes": 0,
            "maxDescendants": 64,
            "maxConcurrency": 1
        },
        "response": {
            "protocol": {"kind": "engineering-assurance.process-evidence", "version": "1"},
            "adapter": {"kind": "engineering-assurance.cli-process-evidence-adapter", "version": "1"},
            "exitCodes": {"kind": "exact", "codes": [0]}
        }
    })
}

#[cfg(unix)]
#[trace("TC-595", "FR-075-AC-6")]
#[test]
fn adversarial_receipt_gate_cannot_read_signing_tokens_or_use_signer() {
    let root = tempfile::tempdir().expect("temporary fixture root");
    let bin = root.path().join("bin");
    fs::create_dir(&bin).expect("fixture bin directory");
    let marker = root.path().join("adversarial-gate-marker");
    let shell = fs::canonicalize("/bin/sh").expect("resolve shell to a regular executable");
    let adversarial_command = "if [ -n \"${GH_TOKEN:-}\" ] || [ -n \"${ACTIONS_ID_TOKEN_REQUEST_TOKEN:-}\" ]; then exit 41; fi; if engineering-assurance receipt-record --request /dev/null; then exit 42; else code=$?; [ \"$code\" -eq 77 ] || exit 43; fi; printf 'token-absent-and-signer-refused\\n' >> \"$FIXTURE_MARKER_PATH\"";
    let fixture_policy = json!({
        "protocol": "engineering-assurance.receipt-policy/v1",
        "gates": [{"name":"adversarial-token-gate", "argv":[shell.display().to_string(), "-c", adversarial_command]}]
    });
    write_executable(
        &bin.join("engineering-assurance"),
        "#!/bin/sh\nprintf 'signer-invoked\\n' >> \"$FIXTURE_MARKER_PATH\"\nif [ -n \"${GH_TOKEN:-}\" ] || [ -n \"${ACTIONS_ID_TOKEN_REQUEST_TOKEN:-}\" ]; then exit 76; fi\nexit 77\n",
    );

    let workflow = workflow("change-receipt.yml");
    let jobs = &workflow["jobs"];
    let gate_text = assert_gate_job_boundary(jobs);
    assert_record_job_is_isolated(jobs);
    assert!(gate_text.contains("producer-execute"));

    let request = producer_request(root.path(), &bin, &marker, &fixture_policy);
    let output = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["producer-execute", "--request"])
        .arg(request.to_string())
        .env("GH_TOKEN", "ambient-fixture-token")
        .env("ACTIONS_ID_TOKEN_REQUEST_TOKEN", "ambient-fixture-token")
        .output()
        .expect("producer-execute starts");
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    let result: Value = serde_json::from_slice(&output.stdout).expect("producer result JSON");
    assert_eq!(result["state"]["kind"], "completed", "result: {result}");
    assert_eq!(result["state"]["observation"]["exitCode"], 0);
    let marker_text = fs::read_to_string(marker).expect("fixture gate wrote its outcome");
    assert_eq!(
        marker_text,
        "signer-invoked\ntoken-absent-and-signer-refused\n"
    );
}

#[trace("TC-560", "FR-070-AC-4")]
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
    assert_record_job_is_isolated(jobs);
    assert!(!format!("{workflow}").contains("statements_not_retained"));

    let gate_text = assert_gate_job_boundary(jobs);
    assert_gate_failure_exports_data_before_failing(&gate_text, jobs);
    assert!(
        jobs["record"]["if"]
            .as_str()
            .is_some_and(|value| value == "always()")
    );
    assert_receipt_policy_and_suite();
}

#[trace("TC-559", "FR-070-AC-3")]
#[test]
fn action_reference_structure_rejects_missing_or_malformed_components() {
    for valid in [
        "./.github/workflows/change-receipt.yml",
        "./actions/local/action.yml",
        "actions/checkout@v7",
        "owner/repository/path/to/action@v1.2.3",
        "owner/repository@refs/tags/v2",
    ] {
        assert!(
            is_structural_action_reference(valid),
            "valid action reference rejected: {valid}"
        );
    }

    for invalid in [
        "./",
        ".//action.yml",
        "./../action.yml",
        "./actions//action.yml",
        "./actions/action@v1",
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
            !is_structural_action_reference(invalid),
            "malformed action reference accepted: {invalid}"
        );
    }
}

#[trace("TC-991", "FR-070-AC-1")]
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
    let receipt_missing = steps
        .iter()
        .find(|step| {
            step["run"]
                .as_str()
                .is_some_and(|run| run.contains("receipt-missing"))
        })
        .expect("receipt-missing invocation");
    let run = receipt_missing["run"].as_str().expect("shell script");
    assert!(run.contains("record_publisher_repository"));
    assert!(run.contains("record_publisher_workflow_path"));
    assert!(run.contains("record_publisher_ref"));
}
