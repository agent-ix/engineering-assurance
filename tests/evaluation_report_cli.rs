// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Direct command coverage for retained evaluation-report aggregation.

use std::{fmt::Write as _, fs, path::Path, process::Command};

use ix_trace_rs::trace;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const SOURCE_REVISION: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn identity(name: &str) -> Value {
    json!({"name": name, "version": "1.2.3", "digest": "a".repeat(64)})
}

fn valid_partial_report(work_dir: &std::path::Path, digest: &str) -> Value {
    let governing = json!({
        "module": identity("engineering-assurance"),
        "plugin": identity("engineering-assurance-plugin"),
        "skill": identity("assurance-onboarding"),
        "workflow": identity("assurance-intake"),
        "quire": identity("quire"),
        "quoin": identity("quoin"),
        "ix_flow": identity("ix-flow"),
        "schema": identity("evaluation-envelope"),
        "producer": identity("cli-agent-evals")
    });
    let observation = json!({
        "host": "codex",
        "host_version": "1.2.3",
        "source_revision": SOURCE_REVISION,
        "suite_revision": "suite-v1",
        "fixture_revision": "fixtures-v1",
        "governing": governing,
        "command_count": 1,
        "elapsed_ms": 1,
        "human_prompt_count": 0,
        "manual_translation_count": 0,
        "repeated_prompt_count": 0,
        "observed_outcome": "reused",
        "terminal_event": null,
        "unsupported_additions": []
    });
    let sample = json!({
        "ok": true,
        "latencyMs": 1,
        "exitReason": "complete",
        "metricStatus": "available",
        "tokenUsage": {
            "input": 1,
            "output": 1,
            "cacheCreation": 0,
            "cacheRead": 0,
            "contextInput": 1,
            "total": 2
        },
        "toolCalls": 0,
        "toolBreakdown": {},
        "classified": {},
        "checks": {"evaluation_result": observation},
        "failures": [],
        "workDir": work_dir,
        "sessionId": "session-1",
        "transcriptDigest": digest,
        "transcriptRetention": "retained",
        "transcriptPath": ".cli-agent-evals/transcripts/sample.transcript"
    });
    json!({
        "reportVersion": "cli-agent-evals.report/v1",
        "ok": true,
        "generatedAt": "2026-09-10T00:00:00Z",
        "suite": "engineering-assurance-onboarding",
        "agent": "codex",
        "model": "model-a",
        "repeats": 1,
        "results": [{
            "id": "EA-001",
            "useCase": "existing-profile",
            "ok": true,
            "passRate": "1/1",
            "aggregate": {"latencyMs": {"p50": 1, "p95": 1}},
            "runs": [sample]
        }],
        "aggregates": {"successRate": "1/1"}
    })
}

fn failed_report_value(mut value: Value) -> Value {
    value["ok"] = json!(false);
    value["results"][0]["id"] = json!("EA-002");
    value["results"][0]["useCase"] = json!("no-profile");
    value["results"][0]["ok"] = json!(false);
    value["results"][0]["passRate"] = json!("0/1");
    value["results"][0]["runs"][0]["ok"] = json!(false);
    value["results"][0]["runs"][0]["exitReason"] = json!("timeout");
    value["results"][0]["runs"][0]["checks"] = json!({});
    value["results"][0]["runs"][0]["failures"] = json!(["timed out"]);
    value["results"][0]["runs"][0]["transcriptRetention"] = json!("not-retained");
    value["results"][0]["runs"][0]
        .as_object_mut()
        .expect("failed sample must be an object")
        .remove("transcriptPath");
    value
}

#[test]
#[trace("TC-129", "FR-017-AC-1", "FR-017-CON-1", "FR-017-CON-3")]
fn tc_129_cli_writes_the_retained_artifact_and_reports_incomplete_matrix() {
    let repository = TempDir::new().expect("repository fixture must be creatable");
    let workspace = TempDir::new().expect("workspace fixture must be creatable");
    let work_dir = workspace.path().join("codex-existing-profile");
    let transcript = work_dir.join(".cli-agent-evals/transcripts/sample.transcript");
    fs::create_dir_all(transcript.parent().expect("transcript must have a parent"))
        .expect("transcript parent must be creatable");
    let transcript_bytes = b"retained transcript\n";
    fs::write(&transcript, transcript_bytes).expect("transcript must be writable");
    let mut digest = String::with_capacity(64);
    for byte in Sha256::digest(transcript_bytes) {
        let _ = write!(&mut digest, "{byte:02x}");
    }
    let report_path = repository.path().join("reports/codex.json");
    fs::create_dir_all(report_path.parent().expect("report must have a parent"))
        .expect("report parent must be creatable");
    fs::write(
        &report_path,
        serde_json::to_vec(&valid_partial_report(&work_dir, &digest)).expect("report must encode"),
    )
    .expect("report must be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args([
            "evaluation-aggregate",
            "--root",
            repository.path().to_str().expect("root must be UTF-8"),
            "--workspace-root",
            workspace.path().to_str().expect("workspace must be UTF-8"),
            "--report",
            "reports/codex.json",
            "--source-revision",
            SOURCE_REVISION,
            "--output",
            "artifacts/aggregate.json",
        ])
        .output()
        .expect("evaluation aggregate command must terminate");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("stdout must be UTF-8");
    assert!(stdout.contains("evaluation aggregate: 1/28 complete"));
    let artifact: Value = serde_json::from_slice(
        &fs::read(repository.path().join("artifacts/aggregate.json"))
            .expect("aggregate artifact must be readable"),
    )
    .expect("aggregate artifact must decode");
    assert_eq!(artifact["revision"], "evaluation-aggregate-v1");
    assert_eq!(artifact["source_revision"], SOURCE_REVISION);
    assert_eq!(artifact["complete_cells"], 1);
    assert_eq!(artifact["required_cells"], 28);
    assert_eq!(artifact["models"]["codex"], "model-a");
    assert_eq!(artifact["reports"][0]["path"], "reports/codex.json");
    assert_eq!(
        artifact["reports"][0]["digest"].as_str().map(str::len),
        Some(64)
    );
}

#[test]
#[trace("TC-129", "FR-017-AC-1", "FR-017-CON-1", "FR-017-CON-3")]
fn tc_129_rust_and_retained_python_emit_the_same_aggregate_observation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    fs::create_dir_all(root.join("target")).expect("ignored target directory must be creatable");
    let repository_fixture = tempfile::Builder::new()
        .prefix("evaluation-report-parity-")
        .tempdir_in(root.join("target"))
        .expect("repository fixture must be creatable beneath the source root");
    let workspace = TempDir::new().expect("workspace fixture must be creatable");
    let work_dir = workspace.path().join("codex-existing-profile");
    let transcript = work_dir.join(".cli-agent-evals/transcripts/sample.transcript");
    fs::create_dir_all(transcript.parent().expect("transcript must have a parent"))
        .expect("transcript parent must be creatable");
    let transcript_bytes = b"same-revision retained transcript\n";
    fs::write(&transcript, transcript_bytes).expect("transcript must be writable");
    let mut digest = String::with_capacity(64);
    for byte in Sha256::digest(transcript_bytes) {
        let _ = write!(&mut digest, "{byte:02x}");
    }
    let report = repository_fixture.path().join("codex.json");
    fs::write(
        &report,
        serde_json::to_vec(&valid_partial_report(&work_dir, &digest)).expect("report must encode"),
    )
    .expect("report must be writable");
    let failed_report = repository_fixture.path().join("failed.json");
    let failed_value = failed_report_value(valid_partial_report(&work_dir, &digest));
    fs::write(
        &failed_report,
        serde_json::to_vec(&failed_value).expect("failed report must encode"),
    )
    .expect("failed report must be writable");
    let python_artifact = repository_fixture.path().join("python.json");
    let rust_artifact = repository_fixture.path().join("rust.json");

    let python = Command::new("python3")
        .arg(root.join("scripts/aggregate_agent_eval_reports.py"))
        .args(["--report", report.to_str().expect("report must be UTF-8")])
        .args([
            "--report",
            failed_report.to_str().expect("failed report must be UTF-8"),
        ])
        .args(["--source-revision", SOURCE_REVISION])
        .args([
            "--output",
            python_artifact
                .to_str()
                .expect("Python output must be UTF-8"),
        ])
        .current_dir(root)
        .output()
        .expect("retained Python aggregate command must terminate");
    assert_eq!(
        python.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&python.stderr)
    );

    let report_relative = report
        .strip_prefix(root)
        .expect("report must be beneath the source root");
    let failed_report_relative = failed_report
        .strip_prefix(root)
        .expect("failed report must be beneath the source root");
    let rust_relative = rust_artifact
        .strip_prefix(root)
        .expect("Rust output must be beneath the source root");
    let rust = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["evaluation-aggregate", "--root"])
        .arg(root)
        .arg("--workspace-root")
        .arg(workspace.path())
        .arg("--report")
        .arg(report_relative)
        .arg("--report")
        .arg(failed_report_relative)
        .args(["--source-revision", SOURCE_REVISION])
        .arg("--output")
        .arg(rust_relative)
        .output()
        .expect("Rust aggregate command must terminate");
    assert_eq!(
        rust.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&rust.stderr)
    );

    let mut python_value: Value = serde_json::from_slice(
        &fs::read(python_artifact).expect("Python artifact must be readable"),
    )
    .expect("Python artifact must decode");
    let mut rust_value: Value =
        serde_json::from_slice(&fs::read(rust_artifact).expect("Rust artifact must be readable"))
            .expect("Rust artifact must decode");
    python_value
        .as_object_mut()
        .expect("Python artifact must be an object")
        .remove("generated_at");
    rust_value
        .as_object_mut()
        .expect("Rust artifact must be an object")
        .remove("generated_at");
    assert_eq!(rust_value, python_value);
}
