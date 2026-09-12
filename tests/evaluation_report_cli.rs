// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Direct command coverage for retained evaluation-report aggregation.

use std::{
    fmt::Write as _,
    fs,
    process::{Command, Output},
};

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
        "host": "claude",
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
        "agent": "claude",
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

fn verify_artifact(repository: &TempDir, workspace: &TempDir) -> Output {
    Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args([
            "evaluation-aggregate-verify",
            "--root",
            repository.path().to_str().expect("root must be UTF-8"),
            "--workspace-root",
            workspace.path().to_str().expect("workspace must be UTF-8"),
            "--artifact",
            "artifacts/aggregate.json",
            "--source-revision",
            SOURCE_REVISION,
        ])
        .output()
        .expect("evaluation aggregate verification command must terminate")
}

#[test]
#[trace("TC-129", "FR-017-AC-1", "FR-017-CON-1", "FR-017-CON-3")]
fn tc_129_cli_writes_the_retained_artifact_and_reports_incomplete_matrix() {
    let repository = TempDir::new().expect("repository fixture must be creatable");
    let workspace = TempDir::new().expect("workspace fixture must be creatable");
    let work_dir = workspace.path().join("claude-existing-profile");
    let transcript = work_dir.join(".cli-agent-evals/transcripts/sample.transcript");
    fs::create_dir_all(transcript.parent().expect("transcript must have a parent"))
        .expect("transcript parent must be creatable");
    let transcript_bytes = b"retained transcript\n";
    fs::write(&transcript, transcript_bytes).expect("transcript must be writable");
    let mut digest = String::with_capacity(64);
    for byte in Sha256::digest(transcript_bytes) {
        let _ = write!(&mut digest, "{byte:02x}");
    }
    let report_path = repository.path().join("reports/claude.json");
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
            "reports/claude.json",
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
    assert!(stdout.contains("evaluation aggregate: 1/7 complete"));
    let artifact: Value = serde_json::from_slice(
        &fs::read(repository.path().join("artifacts/aggregate.json"))
            .expect("aggregate artifact must be readable"),
    )
    .expect("aggregate artifact must decode");
    assert_eq!(artifact["revision"], "evaluation-aggregate-v1");
    assert_eq!(artifact["source_revision"], SOURCE_REVISION);
    assert_eq!(artifact["complete_cells"], 1);
    assert_eq!(artifact["required_cells"], 7);
    assert_eq!(artifact["models"]["claude"], "model-a");
    assert_eq!(artifact["reports"][0]["path"], "reports/claude.json");
    assert_eq!(
        artifact["reports"][0]["digest"].as_str().map(str::len),
        Some(64)
    );

    let verification = verify_artifact(&repository, &workspace);
    assert!(
        verification.status.success(),
        "{}",
        String::from_utf8_lossy(&verification.stderr)
    );

    let mut altered = artifact;
    altered["models"]["codex"] = json!("different-model");
    fs::write(
        repository.path().join("artifacts/aggregate.json"),
        serde_json::to_vec(&altered).expect("altered artifact must encode"),
    )
    .expect("altered artifact must be writable");
    let mismatch = verify_artifact(&repository, &workspace);
    assert_eq!(mismatch.status.code(), Some(2));
    let mismatch_error: Value =
        serde_json::from_slice(&mismatch.stdout).expect("mismatch error must decode");
    assert_eq!(
        mismatch_error["code"],
        "evaluation_report_artifact_mismatch"
    );
}

#[test]
#[trace("TC-051", "FR-006-AC-7")]
fn tc_051_verification_rejects_an_aggregate_retained_for_another_revision() {
    // Retained evaluation evidence is only evidence about the revision it was
    // produced from. An aggregate that is internally consistent but was
    // retained for a different revision is the case that would otherwise let
    // yesterday's passing run certify today's tree, so verification refuses it
    // on the revision alone rather than on any byte difference in the reports.
    let repository = TempDir::new().expect("repository fixture must be creatable");
    let workspace = TempDir::new().expect("workspace fixture must be creatable");
    let work_dir = workspace.path().join("claude-existing-profile");
    let transcript = work_dir.join(".cli-agent-evals/transcripts/sample.transcript");
    fs::create_dir_all(transcript.parent().expect("transcript must have a parent"))
        .expect("transcript parent must be creatable");
    let transcript_bytes = b"retained transcript\n";
    fs::write(&transcript, transcript_bytes).expect("transcript must be writable");
    let mut digest = String::with_capacity(64);
    for byte in Sha256::digest(transcript_bytes) {
        let _ = write!(&mut digest, "{byte:02x}");
    }
    let report_path = repository.path().join("reports/claude.json");
    fs::create_dir_all(report_path.parent().expect("report must have a parent"))
        .expect("report parent must be creatable");
    fs::write(
        &report_path,
        serde_json::to_vec(&valid_partial_report(&work_dir, &digest)).expect("report must encode"),
    )
    .expect("report must be writable");

    let aggregated = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args([
            "evaluation-aggregate",
            "--root",
            repository.path().to_str().expect("root must be UTF-8"),
            "--workspace-root",
            workspace.path().to_str().expect("workspace must be UTF-8"),
            "--report",
            "reports/claude.json",
            "--source-revision",
            SOURCE_REVISION,
            "--output",
            "artifacts/aggregate.json",
        ])
        .output()
        .expect("evaluation aggregate command must terminate");
    assert_eq!(aggregated.status.code(), Some(1));

    // The unchanged artifact verifies, so the refusal below is caused by the
    // retained revision rather than by an artifact that never verified at all.
    assert!(verify_artifact(&repository, &workspace).status.success());

    let artifact_path = repository.path().join("artifacts/aggregate.json");
    let mut artifact: Value = serde_json::from_slice(
        &fs::read(&artifact_path).expect("aggregate artifact must be readable"),
    )
    .expect("aggregate artifact must decode");
    artifact["source_revision"] = json!("c".repeat(40));
    fs::write(
        &artifact_path,
        serde_json::to_vec(&artifact).expect("retained artifact must encode"),
    )
    .expect("retained artifact must be writable");

    let refused = verify_artifact(&repository, &workspace);
    assert_eq!(refused.status.code(), Some(2));
    let error: Value =
        serde_json::from_slice(&refused.stdout).expect("refusal must decode as a machine error");
    assert_eq!(error["code"], "evaluation_report_artifact_invalid");
}
