// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Machine-boundary tests for deterministic workflow-invariant evaluation.

use std::{
    io::Write,
    process::{Command, Stdio},
};

use engineering_assurance::workflow_invariants::{REQUEST_PROTOCOL, RESULT_PROTOCOL};
use ix_trace_rs::trace;
use serde_json::{Value, json};

fn run_bytes(encoded: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .arg("workflow-invariants")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the Cargo-built CLI must start");
    child
        .stdin
        .take()
        .expect("piped stdin must exist")
        .write_all(encoded)
        .expect("test request must be writable");
    child.wait_with_output().expect("CLI must terminate")
}

fn run(request: &Value) -> std::process::Output {
    run_bytes(&serde_json::to_vec(request).expect("test request must serialize"))
}

fn observation_request() -> Value {
    json!({
        "protocol": REQUEST_PROTOCOL,
        "invariants": ["shared.observation_ready"],
        "instance": {
            "defName": "assurance-intake",
            "items": {
                "operator_observation": [{
                    "elapsed_minutes": 0,
                    "command_count": 1,
                }],
            },
        },
        "evaluated_at": "2026-09-10T12:00:00Z",
    })
}

#[trace("TC-106", "FR-016-AC-2", "FR-016-CON-3")]
#[test]
fn tc_106_cli_emits_one_ordered_versioned_result() {
    let output = run(&observation_request());
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    assert!(!output.stdout[..output.stdout.len() - 1].contains(&b'\n'));
    let result: Value = serde_json::from_slice(&output.stdout).expect("stdout must be JSON");
    assert_eq!(result["protocol"], RESULT_PROTOCOL);
    assert_eq!(
        result["outcomes"][0]["invariant"],
        "shared.observation_ready"
    );
    assert_eq!(result["outcomes"][0]["status"], "passed");
}

#[trace("TC-106", "FR-016-AC-2", "FR-016-CON-3")]
#[test]
fn tc_106_cli_refuses_malformed_requests_as_machine_errors() {
    for encoded in [b"{not-json".to_vec(), vec![b' '; 8 * 1024 * 1024 + 1]] {
        let output = run_bytes(&encoded);
        assert_eq!(output.status.code(), Some(2));
        assert!(!output.stderr.is_empty());
        let result: Value = serde_json::from_slice(&output.stdout).expect("stdout must be JSON");
        assert_eq!(result["protocol"], "engineering-assurance.error/v1");
        assert_eq!(result["capability"], "workflow-invariants");
        assert_eq!(result["code"], "workflow_invariant_request_invalid");
    }
}
