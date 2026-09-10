// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Machine-boundary tests for compatibility classification.

use std::{
    io::Write,
    process::{Command, Stdio},
};

use engineering_assurance::compatibility::{REQUEST_PROTOCOL, RESULT_PROTOCOL};
use ix_trace_rs::trace;

fn run(request: &serde_json::Value) -> std::process::Output {
    run_bytes(&serde_json::to_vec(request).expect("test request must serialize"))
}

fn run_bytes(encoded: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .arg("compatibility")
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

fn exact_request() -> serde_json::Value {
    serde_json::json!({
        "protocol": REQUEST_PROTOCOL,
        "observed": [
            {"component": "quire-cli", "version": "0.31.0"},
            {"component": "quoin", "version": "0.23.1"},
            {"component": "ix-flow", "version": "0.2.3"},
            {"component": "engineering-assurance", "version": "0.2.0"}
        ]
    })
}

#[trace("TC-098", "FR-014-AC-2")]
#[test]
fn tc_098_machine_result_is_one_versioned_json_value() {
    let output = run(&exact_request());
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    assert!(!output.stdout[..output.stdout.len() - 1].contains(&b'\n'));
    let result: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be one JSON value");
    assert_eq!(result["protocol"], RESULT_PROTOCOL);
    assert_eq!(result["outcome"], "withheld");
    assert_eq!(result["versions_compatible"], true);
    assert_eq!(result["human_acceptance_recorded"], false);
    assert_eq!(result["gate_satisfied"], false);
    assert_eq!(result["components"].as_array().map(Vec::len), Some(4));
}

#[trace("TC-099", "FR-014-AC-3")]
#[test]
fn tc_099_unknown_protocol_fails_with_structured_result() {
    let output = run(&serde_json::json!({
        "protocol": "engineering-assurance.compatibility-request/v2",
        "observed": []
    }));
    assert_eq!(output.status.code(), Some(2));
    assert!(!output.stderr.is_empty());
    let result: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be one JSON error value");
    assert_eq!(result["protocol"], "engineering-assurance.error/v1");
    assert_eq!(result["capability"], "compatibility");
    assert_eq!(result["code"], "unsupported_compatibility_protocol");
}

#[trace("TC-099", "FR-014-AC-3")]
#[test]
fn tc_099_incompatible_versions_withhold_without_becoming_a_parse_error() {
    let mut request = exact_request();
    request["observed"][1]["version"] = serde_json::json!("0.22.5");
    let output = run(&request);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let result: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be one JSON result");
    assert_eq!(result["protocol"], RESULT_PROTOCOL);
    assert_eq!(result["outcome"], "withheld");
    assert_eq!(result["components"][1]["verdict"], "incompatible");
}

#[trace("TC-099", "FR-014-AC-3")]
#[test]
fn tc_099_malformed_and_extended_requests_fail_as_structured_errors() {
    for encoded in [
        br#"{"protocol": "engineering-assurance.compatibility-request/v1""#.as_slice(),
        br#"{"protocol":"engineering-assurance.compatibility-request/v1","observed":[],"unexpected":true}"#.as_slice(),
    ] {
        let output = run_bytes(encoded);
        assert_eq!(output.status.code(), Some(2));
        assert!(!output.stderr.is_empty());
        let result: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("stdout must be one JSON error value");
        assert_eq!(result["protocol"], "engineering-assurance.error/v1");
        assert_eq!(result["code"], "invalid_compatibility_request");
    }
}
