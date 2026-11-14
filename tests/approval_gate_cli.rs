// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary tests for the opt-in role-based review gate (FR-059, FR-061,
//! FR-062, FR-063 and FR-065).

use std::process::{Command, Stdio};

use serde_json::{Value, json};

const REQUEST_PROTOCOL: &str = "engineering-assurance.review-gate-request/v1";

fn policy() -> Value {
    json!({
        "protocol": "engineering-assurance.approval-policy/v1",
        "required_roles": ["security", "technical"],
        "individuals": {
            "alice": [{"source": "github", "id": "101"}],
            "bob": [{"source": "github", "id": "202"}]
        },
        "roles": {
            "security": ["alice"],
            "technical": ["bob"]
        },
        "agents": [{"source": "github", "id": "303"}]
    })
}

fn review(id: &str, role: &str, state: &str, edited: bool) -> Value {
    let second = id.parse::<u64>().expect("fixture review id") % 60;
    json!({
        "review_id": id.parse::<u64>().expect("fixture review id"),
        "account": {"source": "github", "id": id},
        "account_type": if id == "303" { "bot_or_app" } else { "human" },
        "state": state,
        "submitted_body": format!("Signed-off-meaning: reviewed\nRole: {role}"),
        "body_edited": edited,
        "submitted_at": format!("2026-11-14T03:00:{second:02}Z"),
        "commit": "head-1",
        "pull_request": "agent-ix/ea-109-sandbox#1"
    })
}

fn run(request: Value) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["review-gate", "--request", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("review-gate starts");
    serde_json::to_writer(child.stdin.as_mut().expect("stdin"), &request).expect("request encodes");
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("review-gate exits");
    assert!(
        output.status.success(),
        "review-gate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("result is JSON")
}

fn request(reviews: Vec<Value>) -> Value {
    json!({
        "protocol": REQUEST_PROTOCOL,
        "current_head": "head-1",
        "policy": policy(),
        "reviews": reviews,
        "account_types": [
            {"account": {"source": "github", "id": "101"}, "account_type": "human"},
            {"account": {"source": "github", "id": "202"}, "account_type": "human"}
        ]
    })
}

/// Trace: FR-059-AC-1, FR-061-AC-1, FR-065-AC-2
#[test]
fn accepts_agent_authored_change_with_distinct_human_role_approvals() {
    let result = run(request(vec![
        review("101", "security", "APPROVED", false),
        review("202", "technical", "APPROVED", false),
    ]));
    assert_eq!(result["status"], "satisfied");
    assert_eq!(
        result["assignments"].as_array().expect("assignments").len(),
        2
    );
    assert_eq!(result["assignments"][0]["account"]["id"], "101");
    assert_eq!(result["assignments"][1]["account"]["id"], "202");
}

/// Trace: FR-059-AC-6, FR-062-AC-1, FR-063-AC-1
#[test]
fn edited_reviews_and_bot_reviews_cannot_satisfy_a_role() {
    let result = run(request(vec![
        review("101", "security", "APPROVED", true),
        review("303", "technical", "APPROVED", false),
    ]));
    assert_eq!(result["status"], "unsatisfied");
    assert_eq!(result["assignments"], json!([]));
    assert_eq!(
        result["missing"].as_array().expect("missing roles").len(),
        2
    );
}

/// Trace: FR-065-AC-1, FR-070-AC-2, FR-070-AC-3
#[test]
fn workflow_is_explicitly_opt_in_and_has_no_signing_identity() {
    let workflow = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/.github/workflows/review-gate.yml"
    ))
    .expect("review-gate workflow exists");
    assert!(workflow.contains("vars.EA_REVIEW_GATE_ENABLED == 'true'"));
    assert!(workflow.contains("contents: read"));
    assert!(workflow.contains("pull-requests: read"));
    assert!(!workflow.contains("id-token:"));
    assert!(!workflow.contains("pull_request_target"));
    assert!(!workflow.contains("workflow_run"));
}

/// Trace: FR-059-AC-6, FR-059-AC-7
#[test]
fn unavailable_source_metadata_never_yields_approval() {
    let mut request = request(vec![review("101", "security", "APPROVED", false)]);
    request["source_unavailable"] = json!(true);
    let result = run(request);
    assert_eq!(result["status"], "unavailable");
    assert_eq!(result["assignments"], json!([]));
    assert_eq!(result["error"], "approval_source_unavailable");
}
