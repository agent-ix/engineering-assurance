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
            {"component": "engineering-assurance", "version": "0.3.1"}
        ]
    })
}

#[trace("TC-098", "FR-014-AC-2")]
#[test]
fn tc_098_machine_result_is_one_versioned_json_value() {
    let output = run(&exact_request());
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    assert!(!output.stdout[..output.stdout.len() - 1].contains(&b'\n'));
    let result: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be one JSON value");
    assert_eq!(result["protocol"], RESULT_PROTOCOL);
    assert_eq!(result["outcome"], "compatible");
    assert_eq!(result["versions_compatible"], true);
    assert_eq!(result["human_acceptance_recorded"], true);
    assert_eq!(result["gate_satisfied"], true);
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

/// The hosted workflow definition, read at compile time rather than at run time.
///
/// `include_str!` keeps this a fact about the source tree the test was built
/// from. Reading the file at run time would make the assertion depend on the
/// working directory the test happened to be launched in, and a test that can
/// silently look at the wrong file is the same class of defect as the one this
/// assertion exists to catch.
const HOSTED_WORKFLOW: &str = include_str!("../.github/workflows/ci.yml");

/// The reviewed matrix, read at compile time from the same tree.
const REVIEWED_MATRIX: &str = include_str!("../engineering_assurance/compatibility-matrix.json");

#[trace("TC-130", "FR-012-AC-10")]
#[test]
fn tc_130_hosted_ci_installs_the_version_the_reviewed_matrix_pins() {
    let matrix: serde_json::Value =
        serde_json::from_str(REVIEWED_MATRIX).expect("the reviewed matrix must be JSON");
    let pinned = matrix["components"]
        .as_array()
        .expect("the matrix lists its components")
        .iter()
        .find(|component| component["name"] == "quire-cli")
        .and_then(|component| component["version"].as_str())
        .expect("the matrix pins quire-cli");

    // One fact lives in two files. Hosted CI installs a Quire CLI by literal
    // version and the reviewed matrix pins one, and nothing derived either from
    // the other; the two drifted to 0.30.2 against 0.31.0 and stayed that way,
    // because the only thing that would have noticed is a command no target
    // runs. Under the classifier 0.30.2 is neither the pin nor a version the
    // matrix rules out, so it classifies unknown and withholds the gate — which
    // is the correct verdict about a toolchain nobody reviewed, reached by a
    // gate nobody was running. This assertion is the detector: it needs no
    // toolchain on PATH, so it runs wherever `cargo test` runs.
    let installed: Vec<&str> = HOSTED_WORKFLOW
        .lines()
        .filter(|line| line.contains("@agent-ix/quire-cli@"))
        .collect();
    assert!(
        !installed.is_empty(),
        "hosted CI installs no Quire CLI; this assertion would then pass over nothing"
    );
    for line in installed {
        assert!(
            line.contains(&format!("@agent-ix/quire-cli@{pinned}")),
            "hosted CI installs a Quire CLI the reviewed matrix does not pin ({pinned}): {}",
            line.trim()
        );
    }
}
