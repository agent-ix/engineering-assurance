// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Synthetic protocol coverage for the Rust-owned agent-evaluation provider.

use std::{
    fs,
    io::Write,
    process::{Command, Output, Stdio},
};

use ix_trace_rs::trace;
use serde_json::{Value, json};
use tempfile::TempDir;

fn identity(name: &str) -> Value {
    json!({"name": name, "version": "0.2.4", "digest": "a".repeat(64)})
}

fn root() -> TempDir {
    let root = TempDir::new().expect("provider root must be creatable");
    fs::create_dir_all(root.path().join("evals/fixtures"))
        .expect("fixture directory must be creatable");
    fs::write(
        root.path().join("evals/fixtures/suite.json"),
        r#"{
  "suite_revision": "suite-v1",
  "fixture_revision": "fixtures-v1",
  "scenarios": {
    "existing-profile": {"expected":"reused","input":{"decision_boundary":"one candidate","decision_owner":"owner"}},
    "human-acceptance": {"expected":"accepted","choice":"accept","input":{"decision_boundary":"one decision","decision_owner":"owner","owner_decision":"accept","run_id":"run-1","workflow_state_dir":"workflow-state"}}
  }
}"#,
    ).expect("fixture must be writable");
    let skill = root
        .path()
        .join("engineering_assurance/skills/assurance-onboarding");
    fs::create_dir_all(&skill).expect("skill fixture must be creatable");
    fs::write(skill.join("SKILL.md"), "# fixture\n").expect("skill fixture must be writable");
    fs::create_dir_all(root.path().join("engineering_assurance/skeletons"))
        .expect("skeleton directory must be creatable");
    fs::write(
        root.path()
            .join("engineering_assurance/skeletons/AssuranceProfile.md"),
        "# fixture\n",
    )
    .expect("skeleton must be writable");
    fs::create_dir_all(root.path().join(".agent-evals"))
        .expect("snapshot directory must be creatable");
    let governing = json!({
        "module": identity("engineering-assurance"), "plugin": identity("engineering-assurance-plugin"),
        "skill": identity("assurance-onboarding"), "quire": identity("quire"), "quoin": identity("quoin"),
        "ix_flow": identity("ix-flow"), "schema": identity("evaluation-result-contract"), "producer": identity("cli-agent-evals")
    });
    let snapshot = json!({
        "source_revision": "b".repeat(40), "host": identity("codex"), "governing": governing,
        "workflows": {"assurance-intake": identity("assurance-intake"), "architecture-evaluation": identity("architecture-evaluation")}
    });
    fs::write(
        root.path().join(".agent-evals/governing-codex.json"),
        serde_json::to_vec(&snapshot).expect("snapshot must encode"),
    )
    .expect("snapshot must be writable");
    root
}

fn request(
    workspace: &std::path::Path,
    operation: &str,
    scenario: &Value,
    run: Option<Value>,
) -> Value {
    let mut request = json!({
        "protocol": "cli-agent-evals.external-provider-request/v1",
        "operation": operation,
        "context": {"scenario": scenario, "work_dir": workspace, "cwd": workspace, "session_id": "session-1", "report_dir": workspace.join("reports")}
    });
    if let Some(run) = run {
        request["run"] = run;
    }
    request
}

fn invoke(root: &TempDir, request: &Value) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"));
    command.args([
        "agent-evals-provider",
        "--root",
        root.path().to_str().expect("root must be UTF-8"),
    ]);
    command.env(
        "EA_EVAL_GOVERNING_PATH",
        root.path().join(".agent-evals/governing-codex.json"),
    );
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("provider must start");
    child
        .stdin
        .take()
        .expect("stdin must be piped")
        .write_all(&serde_json::to_vec(request).expect("request must encode"))
        .expect("request must be writable");
    child.wait_with_output().expect("provider must terminate")
}

#[test]
#[trace("TC-031", "FR-006-AC-1")]
fn tc_031_external_provider_config_declares_the_native_rust_provider() {
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let config = repository.join("evals/cli-agent-evals.config.mjs");
    let script = r"
        import { pathToFileURL } from 'node:url';
        const module = await import(pathToFileURL(process.argv[1]).href);
        process.stdout.write(JSON.stringify(module.default.provider));
    ";
    let output = Command::new("node")
        .args([
            "--input-type=module",
            "-e",
            script,
            config.to_str().expect("configuration path must be UTF-8"),
        ])
        .output()
        .expect("node must load the external-provider configuration");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let provider: Value =
        serde_json::from_slice(&output.stdout).expect("provider configuration must emit JSON");
    assert_eq!(
        provider,
        json!({
            "command": "engineering-assurance",
            "args": [
                "agent-evals-provider",
                "--root",
                repository.to_str().expect("repository path must be UTF-8"),
            ],
        })
    );
}

#[test]
#[trace("TC-109", "FR-017-AC-1", "FR-017-CON-1")]
fn tc_109_provider_prepares_and_checks_a_synthetic_native_evaluation() {
    let root = root();
    let workspace = TempDir::new().expect("workspace must be creatable");
    let scenario = json!({"id":"EA-001", "use_case":"existing-profile", "canary":true});
    let prepared = invoke(
        &root,
        &request(workspace.path(), "prepare", &scenario, None),
    );
    assert!(
        prepared.status.success(),
        "{}",
        String::from_utf8_lossy(&prepared.stderr)
    );
    let response: Value =
        serde_json::from_slice(&prepared.stdout).expect("prepare response must decode");
    assert_eq!(response["operation"], "prepare");
    assert!(
        workspace
            .path()
            .join(".agents/skills/assurance-onboarding/SKILL.md")
            .is_file()
    );
    let input: Value = serde_json::from_slice(
        &fs::read(workspace.path().join("EVALUATION_INPUT.json")).expect("input must exist"),
    )
    .expect("input must decode");
    assert_eq!(
        input["result_contract"]["required_top_level_fields"]
            .as_array()
            .map(Vec::len),
        Some(14)
    );
    let result = json!({
        "host": input["result_contract"]["host"], "host_version": input["result_contract"]["host_version"],
        "source_revision": input["result_contract"]["source_revision"], "suite_revision": input["result_contract"]["suite_revision"],
        "fixture_revision": input["result_contract"]["fixture_revision"], "governing": input["result_contract"]["governing"],
        "command_count": 0, "elapsed_ms": 0, "human_prompt_count": 0, "manual_translation_count": 0, "repeated_prompt_count": 0,
        "observed_outcome": "reused", "terminal_event": null, "unsupported_additions": []
    });
    fs::write(
        workspace.path().join("EVALUATION_RESULT.json"),
        serde_json::to_vec(&result).expect("result must encode"),
    )
    .expect("result must be writable");
    let asserted = invoke(
        &root,
        &request(
            workspace.path(),
            "assert",
            &scenario,
            Some(json!({"ok":true,"exitReason":"complete","wallMs":1})),
        ),
    );
    assert!(
        asserted.status.success(),
        "{}",
        String::from_utf8_lossy(&asserted.stderr)
    );
    let response: Value =
        serde_json::from_slice(&asserted.stdout).expect("assert response must decode");
    assert_eq!(response["assertion"]["ok"], true);
}

#[test]
#[trace("TC-109", "FR-017-AC-1", "FR-017-CON-1")]
fn tc_109_provider_refuses_a_snapshot_outside_the_native_root() {
    let root = root();
    let workspace = TempDir::new().expect("workspace must be creatable");
    let request = request(
        workspace.path(),
        "prepare",
        &json!({"id":"EA-001", "use_case":"existing-profile", "canary":true}),
        None,
    );
    let mut command = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"));
    command.args([
        "agent-evals-provider",
        "--root",
        root.path().to_str().expect("root must be UTF-8"),
    ]);
    command.env("EA_EVAL_GOVERNING_PATH", "/tmp/not-an-ea-snapshot.json");
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("provider must start");
    child
        .stdin
        .take()
        .expect("stdin must be piped")
        .write_all(&serde_json::to_vec(&request).expect("request must encode"))
        .expect("request must be writable");
    let output = child.wait_with_output().expect("provider must terminate");
    assert_eq!(output.status.code(), Some(2));
    let error: Value = serde_json::from_slice(&output.stdout).expect("error must decode");
    assert_eq!(error["code"], "agent_evals_provider_snapshot_invalid");
}

#[test]
#[trace("TC-109", "FR-017-AC-1", "FR-017-CON-1")]
fn tc_109_provider_emits_the_complete_terminal_contract() {
    let root = root();
    let workspace = TempDir::new().expect("workspace must be creatable");
    let response = invoke(
        &root,
        &request(
            workspace.path(),
            "prepare",
            &json!({"id":"EA-006", "use_case":"human-acceptance"}),
            None,
        ),
    );
    assert!(
        response.status.success(),
        "{}",
        String::from_utf8_lossy(&response.stderr)
    );
    let input: Value = serde_json::from_slice(
        &fs::read(workspace.path().join("EVALUATION_INPUT.json")).expect("input must exist"),
    )
    .expect("input must decode");
    let terminal = &input["result_contract"]["terminal_event_contract"];
    assert_eq!(terminal["required"], true);
    assert_eq!(terminal["choice"], "accept");
    assert_eq!(terminal["run_id"], "run-1");
    assert_eq!(terminal["workflow_state_dir"], "workflow-state");
    assert!(workspace.path().join("workflow-state").is_dir());
}

#[test]
#[trace("TC-109", "FR-017-AC-1", "FR-017-CON-3")]
fn tc_109_provider_refuses_a_fixture_state_path_that_escapes_the_workspace() {
    let root = root();
    let fixture = root.path().join("evals/fixtures/suite.json");
    let original = fs::read_to_string(&fixture).expect("fixture must be readable");
    fs::write(&fixture, original.replace("workflow-state", "../escape"))
        .expect("fixture must be writable");
    let workspace = TempDir::new().expect("workspace must be creatable");
    let response = invoke(
        &root,
        &request(
            workspace.path(),
            "prepare",
            &json!({"id":"EA-006", "use_case":"human-acceptance"}),
            None,
        ),
    );
    assert_eq!(response.status.code(), Some(2));
    assert!(
        !workspace
            .path()
            .parent()
            .expect("workspace parent must exist")
            .join("escape")
            .exists()
    );
}

#[test]
#[trace("TC-136", "FR-017-AC-9")]
fn tc_136_describe_omits_optional_scenario_metadata_it_does_not_hold() {
    let root = root();
    let output = invoke(
        &root,
        &json!({
            "protocol": "cli-agent-evals.external-provider-request/v1",
            "operation": "describe",
            "context": {
                "scenario": {"id": ""},
                "work_dir": "",
                "cwd": "",
                "session_id": "",
                "report_dir": ""
            }
        }),
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: Value =
        serde_json::from_slice(&output.stdout).expect("describe response must be JSON");
    let scenarios = response["scenarios"]
        .as_array()
        .expect("describe response must carry a scenario catalogue");
    assert!(!scenarios.is_empty());
    let mut canaries = 0_usize;
    for scenario in scenarios {
        let object = scenario
            .as_object()
            .expect("each described scenario must be an object");
        for (key, value) in object {
            assert!(
                !value.is_null(),
                "described scenario {} serialized a null {key}; the external-provider contract \
                 requires an absent optional field to be omitted",
                object["id"]
            );
        }
        assert!(object.contains_key("id"));
        assert!(object.contains_key("use_case"));
        assert!(
            !object.contains_key("title"),
            "no described scenario carries a title, so the key must be omitted"
        );
        if object.contains_key("canary") {
            assert_eq!(object["canary"], json!(true));
            canaries += 1;
        }
    }
    assert_eq!(
        canaries, 1,
        "exactly one described scenario is the canary and only that scenario carries the key"
    );
}
