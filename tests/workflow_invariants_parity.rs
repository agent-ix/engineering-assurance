// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Additive-migration parity with the retained JavaScript invariant provider.

use std::{
    io::Write,
    process::{Command, Stdio},
};

use engineering_assurance::workflow_invariants::{
    InvariantName, REQUEST_PROTOCOL, evaluate_request_bytes,
};
use ix_trace_rs::trace;
use serde_json::{Map, Value, json};

const EVALUATED_AT: &str = "2026-09-10T12:00:00Z";
const REFERENCE: &str = r#"
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const request = JSON.parse(fs.readFileSync(0, "utf8"));
Date.now = () => Date.parse(request.evaluated_at);
const provider = pathToFileURL(path.resolve(
  "engineering_assurance/skills/assurance-onboarding/scripts/invariants.js",
)).href;
const { invariants } = await import(provider);
const outcomes = request.invariants.map((name) => {
  const verdict = invariants[name]({ instance: request.instance });
  return verdict === true
    ? { invariant: name, status: "passed" }
    : {
        invariant: name,
        status: "failed",
        code: verdict.code,
        details: verdict.details ?? {},
      };
});
process.stdout.write(JSON.stringify(outcomes));
"#;

fn canonical_names() -> Vec<&'static str> {
    InvariantName::ALL
        .into_iter()
        .map(InvariantName::as_str)
        .collect()
}

fn request(instance: &Value, invariants: &[&str]) -> Value {
    json!({
        "protocol": REQUEST_PROTOCOL,
        "invariants": invariants,
        "instance": instance,
        "evaluated_at": EVALUATED_AT,
    })
}

fn passing_projection() -> Value {
    serde_json::from_str(include_str!(
        "fixtures/workflow-invariants/passing-projection.json"
    ))
    .expect("passing projection fixture must be valid JSON")
}

fn workflow_projection(
    def_name: &str,
    terminal_transitions: &[&str],
    item_kinds: &[&str],
) -> Value {
    let catalog = passing_projection();
    let catalog_items = catalog["items"]
        .as_object()
        .expect("fixture item catalog must be an object");
    let items = item_kinds
        .iter()
        .map(|kind| {
            (
                (*kind).to_owned(),
                catalog_items
                    .get(*kind)
                    .unwrap_or_else(|| panic!("fixture item kind {kind} must exist"))
                    .clone(),
            )
        })
        .collect::<Map<_, _>>();
    let gate_config = terminal_transitions
        .iter()
        .map(|transition| ((*transition).to_owned(), json!("hitl")))
        .collect::<Map<_, _>>();
    json!({"defName": def_name, "gateConfig": gate_config, "items": items})
}

fn reference_outcomes(request: &Value) -> Vec<u8> {
    let mut child = Command::new("node")
        .args(["--input-type=module", "-e", REFERENCE])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("retained JavaScript provider must start during additive migration");
    child
        .stdin
        .take()
        .expect("piped stdin must exist")
        .write_all(&serde_json::to_vec(request).expect("request fixture must serialize"))
        .expect("request fixture must be writable");
    let output = child.wait_with_output().expect("provider must terminate");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn assert_parity(request: &Value) {
    let rust = evaluate_request_bytes(
        &serde_json::to_vec(request).expect("request fixture must serialize"),
    )
    .expect("Rust must accept the shared valid request domain");
    let rust_bytes = serde_json::to_vec(&rust.outcomes).expect("outcomes must serialize");
    assert_eq!(rust_bytes, reference_outcomes(request));
}

fn assert_failed(request: &Value, expected_codes: &[&str]) {
    let result = evaluate_request_bytes(
        &serde_json::to_vec(request).expect("request fixture must serialize"),
    )
    .expect("incomplete domain state must produce outcomes");
    let outcomes = serde_json::to_value(result.outcomes).expect("outcomes must serialize");
    let codes = outcomes
        .as_array()
        .expect("outcomes must be an array")
        .iter()
        .map(|outcome| outcome["code"].as_str().expect("outcome must fail"))
        .collect::<Vec<_>>();
    assert_eq!(codes, expected_codes);
}

fn passing_workflow_cases() -> Vec<(Value, Vec<&'static str>)> {
    vec![
        (
            workflow_projection(
                "assurance-intake",
                &["decision_ready->accepted", "decision_ready->rejected"],
                &[
                    "intake_request",
                    "artifact_validation",
                    "exception",
                    "operator_observation",
                ],
            ),
            vec![
                "shared.observation_ready",
                "shared.terminal_gates",
                "shared.exceptions_ready",
                "intake.scope_ready",
                "intake.artifacts_ready",
            ],
        ),
        (
            workflow_projection(
                "architecture-evaluation",
                &["decision_ready->accepted", "decision_ready->rejected"],
                &[
                    "architecture_request",
                    "artifact_validation",
                    "architecture_scenario",
                    "review_validation",
                    "operator_observation",
                ],
            ),
            vec![
                "shared.observation_ready",
                "shared.terminal_gates",
                "shared.exceptions_ready",
                "architecture.scenarios_ready",
                "architecture.review_ready",
            ],
        ),
        (
            workflow_projection(
                "measurement-promotion",
                &["decision_ready->promoted", "decision_ready->not_promoted"],
                &[
                    "promotion_request",
                    "promotion_evidence",
                    "operator_observation",
                ],
            ),
            vec![
                "shared.observation_ready",
                "shared.terminal_gates",
                "shared.exceptions_ready",
                "measurement.promotion_ready",
            ],
        ),
        (
            workflow_projection(
                "change-assurance",
                &["decision_ready->approved", "decision_ready->rejected"],
                &[
                    "change_request",
                    "impact_snapshot",
                    "assurance_snapshot",
                    "review_validation",
                    "operator_observation",
                ],
            ),
            vec![
                "shared.observation_ready",
                "shared.terminal_gates",
                "shared.exceptions_ready",
                "change.impact_ready",
                "change.snapshot_ready",
                "change.review_ready",
            ],
        ),
    ]
}

#[trace("TC-106", "FR-016-AC-2", "FR-016-CON-3")]
#[test]
fn tc_106_all_canonical_invariants_match_the_reference_in_request_order() {
    let cases = passing_workflow_cases();
    for (projection, names) in cases {
        assert_parity(&request(&projection, &names));
    }

    let change = workflow_projection(
        "change-assurance",
        &["decision_ready->approved", "decision_ready->rejected"],
        &[
            "change_request",
            "impact_snapshot",
            "assurance_snapshot",
            "review_validation",
            "operator_observation",
        ],
    );
    let mut ordered = vec![
        "shared.observation_ready",
        "shared.terminal_gates",
        "change.impact_ready",
        "change.snapshot_ready",
        "change.review_ready",
    ];
    ordered.reverse();
    assert_parity(&request(&change, &ordered));
}

#[trace("TC-106", "FR-016-AC-2", "FR-016-CON-3")]
#[test]
fn tc_106_failure_codes_and_time_boundaries_match_the_reference() {
    let empty = json!({"defName": "change-assurance", "items": {}});
    assert_parity(&request(&empty, &canonical_names()));

    let mut expired = workflow_projection(
        "assurance-intake",
        &["decision_ready->accepted", "decision_ready->rejected"],
        &["intake_request", "exception", "operator_observation"],
    );
    expired["items"]["exception"][0]["expires_at"] = json!(EVALUATED_AT);
    assert_parity(&request(&expired, &["shared.exceptions_ready"]));

    let mut future_snapshot = workflow_projection(
        "change-assurance",
        &["decision_ready->approved", "decision_ready->rejected"],
        &[
            "change_request",
            "assurance_snapshot",
            "operator_observation",
        ],
    );
    future_snapshot["items"]["assurance_snapshot"][0]["verified_at"] =
        json!("2026-09-10T12:00:01Z");
    assert_parity(&request(&future_snapshot, &["change.snapshot_ready"]));

    let selection_boundary = json!({
        "defName": "assurance-intake",
        "items": {
            "intake_request": [{"interviewId": "intake-1"}],
            "promotion_request": [{
                "interviewId": "promotion-1",
                "exceptions_expected": true,
            }],
        },
    });
    assert_parity(&request(&selection_boundary, &["shared.exceptions_ready"]));
}

#[trace("TC-106", "FR-016-AC-2", "FR-016-CON-3")]
#[test]
fn tc_106_absent_binding_subjects_cannot_match_absent_evidence_fields() {
    let mut architecture = passing_projection();
    architecture["defName"] = json!("architecture-evaluation");
    architecture["items"]
        .as_object_mut()
        .expect("items must be an object")
        .remove("architecture_request");
    architecture["items"]["artifact_validation"][0]
        .as_object_mut()
        .expect("validation must be an object")
        .remove("path");
    architecture["items"]["review_validation"][0]
        .as_object_mut()
        .expect("review must be an object")
        .remove("subject_path");
    assert_failed(
        &request(
            &architecture,
            &["architecture.scenarios_ready", "architecture.review_ready"],
        ),
        &[
            "architecture_context_incomplete",
            "architecture_review_missing",
        ],
    );

    let mut change = passing_projection();
    change["items"]
        .as_object_mut()
        .expect("items must be an object")
        .remove("change_request");
    change["items"]["assurance_snapshot"][0]
        .as_object_mut()
        .expect("snapshot must be an object")
        .remove("source_revision");
    change["items"]["review_validation"][1]
        .as_object_mut()
        .expect("review must be an object")
        .remove("source_revision");
    assert_failed(
        &request(&change, &["change.snapshot_ready", "change.review_ready"]),
        &["assurance_snapshot_invalid", "code_review_missing"],
    );
}

#[trace("TC-106", "FR-016-AC-2", "FR-016-CON-3")]
#[test]
fn tc_106_malformed_unknown_and_duplicate_requests_refuse_before_outcomes() {
    let cases = [
        (
            json!({
                "protocol": REQUEST_PROTOCOL,
                "invariants": [],
                "instance": passing_projection(),
                "evaluated_at": EVALUATED_AT,
            }),
            "workflow_invariant_request_invalid",
        ),
        (
            request(
                &passing_projection(),
                &["shared.observation_ready", "shared.observation_ready"],
            ),
            "workflow_invariant_request_invalid",
        ),
        (
            request(&passing_projection(), &["shared.not_registered"]),
            "workflow_invariant_unknown",
        ),
        (
            json!({
                "protocol": REQUEST_PROTOCOL,
                "invariants": ["shared.observation_ready"],
                "instance": {"defName": "unknown-workflow", "items": {}},
                "evaluated_at": EVALUATED_AT,
            }),
            "workflow_binding_invalid",
        ),
        (
            json!({
                "protocol": REQUEST_PROTOCOL,
                "invariants": ["shared.observation_ready"],
                "instance": passing_projection(),
                "evaluated_at": "not-an-instant",
            }),
            "workflow_invariant_request_invalid",
        ),
    ];

    for (request, expected_code) in cases {
        let error = evaluate_request_bytes(
            &serde_json::to_vec(&request).expect("request fixture must serialize"),
        )
        .expect_err("invalid request must be refused");
        assert_eq!(error.code(), expected_code);
    }

    let extended = json!({
        "protocol": REQUEST_PROTOCOL,
        "invariants": ["shared.observation_ready"],
        "instance": passing_projection(),
        "evaluated_at": EVALUATED_AT,
        "unexpected": true,
    });
    assert!(evaluate_request_bytes(&serde_json::to_vec(&extended).unwrap()).is_err());
    assert!(evaluate_request_bytes(b"{not-json").is_err());
}
