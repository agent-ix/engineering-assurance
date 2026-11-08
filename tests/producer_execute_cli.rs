// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary tests for the `producer-execute` command (FR-029).

use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const REQUEST_SCHEMA: &str =
    include_str!("../src/producer_execute_schemas/producer-execution-request-v1.schema.json");
const RESULT_SCHEMA: &str =
    include_str!("../src/producer_execute_schemas/producer-execution-result-v1.schema.json");

fn fixture() -> String {
    fs::canonicalize(env!("CARGO_BIN_EXE_producer-execution-fixture"))
        .expect("Rust fixture executable must be available")
        .display()
        .to_string()
}

fn request(root: &Path, arguments: &[&str]) -> Value {
    json!({
        "protocol": "engineering-assurance.producer-execution-request/v1",
        "producer": {"name": "rust-fixture-producer", "version": "1.0.0", "executable": fixture()},
        "caller": {"kind": "fixture.request", "version": "1"},
        "procedure": "direct",
        "capabilityRoot": fs::canonicalize(root).expect("root").display().to_string(),
        "arguments": arguments
            .iter()
            .map(|value| json!({"kind": "literal", "value": value}))
            .collect::<Vec<_>>(),
        "environment": {},
        "inputs": [],
        "stdin": {"kind": "null"},
        "outputs": [],
        "containment": {
            "profile": "process-group-v1",
            "contract": {"kind": "fixture.cooperative-confinement", "version": "1"}
        },
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
            "adapter": {
                "kind": "engineering-assurance.cli-process-evidence-adapter",
                "version": "1"
            },
            "exitCodes": {"kind": "exact", "codes": [0]}
        }
    })
}

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .arg("producer-execute")
        .args(arguments)
        .output()
        .expect("the Cargo-built CLI must start")
}

fn run_request(request: &Value) -> Output {
    run(&["--request", &request.to_string()])
}

fn result_line(output: &Output) -> Value {
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    let body = &output.stdout[..output.stdout.len() - 1];
    assert!(!body.contains(&b'\n'));
    let value: Value = serde_json::from_slice(body).expect("stdout must be one JSON document");
    assert_eq!(
        serde_json_canonicalizer::to_vec(&value).expect("canonicalizes"),
        body,
        "the result line must be its own RFC 8785 canonicalization"
    );
    value
}

fn error_code(output: &Output) -> String {
    assert_eq!(output.status.code(), Some(2));
    assert!(!output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).expect("machine error on stdout");
    assert_eq!(value["protocol"], "engineering-assurance.error/v1");
    assert_eq!(value["capability"], "producer-execute");
    value["code"].as_str().expect("code").to_owned()
}

fn validator(schema: &str) -> jsonschema::Validator {
    let schema: Value = serde_json::from_str(schema).expect("schema is JSON");
    jsonschema::options()
        .build(&schema)
        .expect("published schema compiles")
}

#[trace("TC-214", "FR-029-AC-1")]
#[test]
fn tc_214_valid_request_prints_one_canonical_result_line() {
    let root = tempfile::tempdir().expect("temporary root");
    let value = result_line(&run_request(&request(root.path(), &["emit", "hello"])));
    assert_eq!(
        value["protocol"],
        "engineering-assurance.producer-execution-result/v1"
    );
    assert_eq!(value["producer"]["name"], "rust-fixture-producer");
    assert!(value["timing"]["totalNanos"].is_u64());
    if cfg!(target_os = "linux") {
        assert_eq!(value["state"]["kind"], "completed");
        assert_eq!(value["state"]["observation"]["exitCode"], 0);
        assert_eq!(
            value["process"]["stdout"]["bytes"],
            json!(b"hello".to_vec())
        );
    } else {
        assert_eq!(value["state"]["kind"], "containment_failure");
    }
}

#[trace("TC-215", "FR-029-AC-2")]
#[test]
fn tc_215_bad_requests_yield_typed_errors_without_launching() {
    let root = tempfile::tempdir().expect("temporary root");
    let marker = root.path().join("launched");
    let touch = marker.display().to_string();

    assert_eq!(
        error_code(&run(&["--request", "{not json"])),
        "producer_execute_request_unreadable"
    );

    let mut unknown = request(root.path(), &["touch", &touch]);
    unknown["surprise"] = json!(true);
    assert_eq!(
        error_code(&run_request(&unknown)),
        "producer_execute_request_invalid"
    );

    let mut wrong_type = request(root.path(), &["touch", &touch]);
    wrong_type["budget"]["timeoutMillis"] = json!("soon");
    assert_eq!(
        error_code(&run_request(&wrong_type)),
        "producer_execute_request_invalid"
    );

    let mut relative_root = request(root.path(), &["touch", &touch]);
    relative_root["capabilityRoot"] = json!("relative-capability");
    assert_eq!(
        error_code(&run_request(&relative_root)),
        "producer_execute_request_structure_invalid"
    );

    let mut over_limit = request(root.path(), &["touch", &touch]);
    over_limit["budget"]["timeoutMillis"] = json!(86_400_001_u64);
    assert_eq!(
        error_code(&run_request(&over_limit)),
        "producer_execute_request_structure_invalid"
    );

    let mut protocol = request(root.path(), &["touch", &touch]);
    protocol["protocol"] = json!("engineering-assurance.producer-execution-request/v2");
    assert_eq!(
        error_code(&run_request(&protocol)),
        "producer_execute_request_structure_invalid"
    );

    assert!(
        !marker.exists(),
        "no producer may launch on a refused request"
    );
}

#[trace("TC-216", "FR-029-AC-3")]
#[test]
fn tc_216_non_completed_states_are_results_at_exit_zero() {
    let root = tempfile::tempdir().expect("temporary root");
    let marker = root.path().join("launched");

    let mut foreign = request(root.path(), &["touch", &marker.display().to_string()]);
    foreign["response"]["adapter"]["kind"] = json!("consumer.adapter");
    let value = result_line(&run_request(&foreign));
    assert_eq!(value["state"]["kind"], "refused");
    assert_eq!(value["state"]["reason"], "adapter_binding");
    assert!(!marker.exists());

    let mut missing = request(root.path(), &["emit", "x"]);
    missing["producer"]["executable"] = json!(root.path().join("absent").display().to_string());
    let state = result_line(&run_request(&missing))["state"]["kind"].clone();
    assert!(
        state == "unavailable" || state == "containment_failure",
        "unexpected state {state}"
    );

    if cfg!(target_os = "linux") {
        let mut slow = request(root.path(), &["wait", "60000"]);
        slow["budget"]["timeoutMillis"] = json!(100);
        assert_eq!(
            result_line(&run_request(&slow))["state"]["kind"],
            "timed_out"
        );
    }
}

#[trace("TC-217", "FR-029-AC-4")]
#[test]
fn tc_217_published_schemas_are_printed_and_enforced() {
    for (kind, published) in [("request", REQUEST_SCHEMA), ("result", RESULT_SCHEMA)] {
        let output = run(&["--schema", kind]);
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stdout, published.as_bytes());
    }

    let root = tempfile::tempdir().expect("temporary root");
    let good = request(root.path(), &["emit", "x"]);
    let request_schema = validator(REQUEST_SCHEMA);
    assert!(request_schema.is_valid(&good));
    for field in good.as_object().expect("object").keys() {
        let mut mutant = good.clone();
        mutant.as_object_mut().expect("object").remove(field);
        assert!(
            !request_schema.is_valid(&mutant),
            "{field} must be required"
        );
    }
    let mut unknown = good.clone();
    unknown["surprise"] = json!(1);
    assert!(!request_schema.is_valid(&unknown));
    let mut protocol = good.clone();
    protocol["protocol"] = json!("other/v1");
    assert!(!request_schema.is_valid(&protocol));

    let result_schema = validator(RESULT_SCHEMA);
    let result = result_line(&run_request(&good));
    assert!(result_schema.is_valid(&result), "{result}");
    let mut state = result.clone();
    state["state"] = json!({"kind": "invented"});
    assert!(!result_schema.is_valid(&state));
}

#[trace("TC-218", "FR-029-AC-5")]
#[test]
fn tc_218_binary_runs_from_an_unrelated_directory_with_empty_environment() {
    let root = tempfile::tempdir().expect("temporary root");
    let elsewhere = tempfile::tempdir().expect("unrelated directory");
    let body = request(root.path(), &["emit", "ok"]).to_string();
    let mut child = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["producer-execute", "--request", "-"])
        .current_dir(elsewhere.path())
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("CLI must start");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(body.as_bytes())
        .expect("request must be writable");
    let output = child.wait_with_output().expect("CLI must terminate");
    let value = result_line(&output);
    assert_eq!(
        value["protocol"],
        "engineering-assurance.producer-execution-result/v1"
    );
}
