// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary tests for the opt-in role-based review gate (FR-059, FR-061,
//! FR-062, FR-063 and FR-065).

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
};

use ix_trace_rs::trace;
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

fn collect_request_with_fake_github(
    rest_response: &str,
    graphql_response: &str,
) -> (tempfile::TempDir, Value, std::path::PathBuf) {
    let temp = tempfile::tempdir().expect("temporary directory");
    let policy_repo = temp.path().join("policy");
    fs::create_dir_all(policy_repo.join(".engineering-assurance")).expect("policy directory");
    let init = Command::new("git")
        .args(["init", "--quiet", "--initial-branch=main"])
        .current_dir(&policy_repo)
        .status()
        .expect("git init starts");
    assert!(init.success(), "git init succeeds");
    let config = Command::new("git")
        .args(["config", "user.name", "Review Gate Fixture"])
        .current_dir(&policy_repo)
        .status()
        .expect("git config starts");
    assert!(config.success(), "git config succeeds");
    let config = Command::new("git")
        .args([
            "config",
            "user.email",
            "review-gate-fixture@example.invalid",
        ])
        .current_dir(&policy_repo)
        .status()
        .expect("git config starts");
    assert!(config.success(), "git config succeeds");
    fs::write(
        policy_repo.join(".engineering-assurance/approval-policy.json"),
        serde_json::to_vec(&policy()).expect("policy serializes"),
    )
    .expect("policy file writes");
    let add = Command::new("git")
        .args(["add", ".engineering-assurance/approval-policy.json"])
        .current_dir(&policy_repo)
        .status()
        .expect("git add starts");
    assert!(add.success(), "git add succeeds");
    let commit = Command::new("git")
        .args(["commit", "--quiet", "-m", "fixture policy"])
        .current_dir(&policy_repo)
        .status()
        .expect("git commit starts");
    assert!(commit.success(), "git commit succeeds");
    let base_sha = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&policy_repo)
        .output()
        .expect("git rev-parse starts");
    assert!(base_sha.status.success(), "git rev-parse succeeds");
    let base_sha = String::from_utf8(base_sha.stdout)
        .expect("commit id is UTF-8")
        .trim()
        .to_owned();

    let fake_bin = temp.path().join("bin");
    fs::create_dir(&fake_bin).expect("fake bin directory");
    let fake_gh = fake_bin.join("gh");
    fs::write(
        &fake_gh,
        "#!/bin/sh\ncase \"$*\" in\n  *graphql*) touch \"$FAKE_GRAPHQL_MARKER\"; printf '%s\\n' \"$FAKE_GRAPHQL_RESPONSE\" ;;\n  *'/pulls/1/reviews'*) printf '%s\\n' \"$FAKE_REST_RESPONSE\" ;;\n  *'/user/'*) printf '{\"type\":\"User\"}\\n' ;;\n  *) echo \"unexpected gh invocation\" >&2; exit 2 ;;\nesac\n",
    )
    .expect("fake gh writes");
    fs::set_permissions(&fake_gh, fs::Permissions::from_mode(0o755))
        .expect("fake gh is executable");
    let graphql_marker = temp.path().join("graphql-called");
    let original_path = std::env::var_os("PATH").expect("PATH exists");
    let path = std::env::join_paths(
        std::iter::once(fake_bin).chain(std::env::split_paths(&original_path)),
    )
    .expect("fake PATH joins");
    let collector = Command::new("bash")
        .args([
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/scripts/prepare-review-gate-request.sh"
            ),
            policy_repo.to_str().expect("policy path is UTF-8"),
            &base_sha,
            "agent-ix/ea-109-sandbox-20261114",
            "1",
        ])
        .env("PATH", path)
        .env(
            "REVIEW_GATE_HEAD_SHA",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )
        .env("FAKE_REST_RESPONSE", rest_response)
        .env("FAKE_GRAPHQL_RESPONSE", graphql_response)
        .env("FAKE_GRAPHQL_MARKER", &graphql_marker)
        .output()
        .expect("request collector starts");
    assert!(
        collector.status.success(),
        "request collector failed: {}",
        String::from_utf8_lossy(&collector.stderr)
    );
    let request = serde_json::from_slice(&collector.stdout).expect("collector emits JSON");
    (temp, request, graphql_marker)
}

#[trace("FR-059-AC-1", "FR-061-AC-1", "FR-065-AC-10")]
#[test]
fn agent_authored_pull_request_passes_with_distinct_human_role_approvals() {
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

    let workflow = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/.github/workflows/review-gate.yml"
    ))
    .expect("review-gate workflow exists");
    assert!(workflow.contains("  pull_request:"));
    assert!(!workflow.contains("github.event.pull_request.user.type"));
}

#[trace("FR-059-AC-6", "FR-062-AC-1")]
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

#[trace("FR-061-AC-4", "FR-070-AC-2", "FR-070-AC-3")]
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
    assert!(workflow.contains("bash engine/scripts/prepare-review-gate-request.sh"));
    assert!(!workflow.contains("id-token:"));
    assert!(!workflow.contains("pull_request_target"));
    assert!(!workflow.contains("workflow_run"));
    assert!(workflow.contains("ref: ${{ github.event.pull_request.base.sha || github.sha }}"));

    let gate_source =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/approval_gate.rs"))
            .expect("approval gate implementation exists");
    let gate_host = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/approval_gate_host.rs"
    ))
    .expect("approval gate adapter exists");
    for signing_symbol in ["sign_approval", "sign_statement", "gitsign", "cosign"] {
        assert!(!gate_source.contains(signing_symbol));
        assert!(!gate_host.contains(signing_symbol));
        assert!(!workflow.contains(signing_symbol));
    }

    let ruleset: Value =
        serde_json::from_str(include_str!("../examples/main-review-gate-ruleset.json"))
            .expect("review-gate ruleset is JSON");
    let rules = ruleset["rules"].as_array().expect("rules");
    let rule_types: Vec<_> = rules
        .iter()
        .filter_map(|rule| rule["type"].as_str())
        .collect();
    assert!(rule_types.contains(&"pull_request"));
    assert!(rule_types.contains(&"required_status_checks"));
    assert!(rule_types.contains(&"non_fast_forward"));
    assert!(rule_types.contains(&"deletion"));
    assert!(!rule_types.contains(&"merge_queue"));
    let pull_request = rules
        .iter()
        .find(|rule| rule["type"] == "pull_request")
        .expect("pull-request protection");
    assert_eq!(
        pull_request["parameters"]["dismiss_stale_reviews_on_push"],
        true
    );
    assert_eq!(
        pull_request["parameters"]["require_last_push_approval"],
        true
    );
    assert_eq!(
        pull_request["parameters"]["require_code_owner_review"],
        false
    );
    assert_eq!(
        pull_request["parameters"]["required_review_thread_resolution"],
        false
    );
    assert_eq!(
        pull_request["parameters"]["required_approving_review_count"],
        1
    );
    let status_checks = rules
        .iter()
        .find(|rule| rule["type"] == "required_status_checks")
        .expect("required review-gate status");
    assert_eq!(
        status_checks["parameters"]["strict_required_status_checks_policy"],
        true
    );
    assert_eq!(
        status_checks["parameters"]["required_status_checks"][0]["context"],
        "review-gate / review-gate"
    );
    assert_eq!(
        ruleset["bypass_actors"][0]["actor_type"],
        "OrganizationAdmin"
    );
    assert_eq!(ruleset["bypass_actors"][0]["actor_id"], Value::Null);
    assert_eq!(ruleset["bypass_actors"][0]["bypass_mode"], "pull_request");
}

#[trace("FR-059-AC-6", "FR-063-AC-6")]
#[test]
fn unavailable_source_metadata_never_yields_approval() {
    let mut request = request(vec![review("101", "security", "APPROVED", false)]);
    request["source_unavailable"] = json!(true);
    let result = run(request);
    assert_eq!(result["status"], "unavailable");
    assert_eq!(result["assignments"], json!([]));
    assert_eq!(result["error"], "approval_source_unavailable");
}

#[trace("FR-063-AC-4")]
#[test]
fn a_successful_empty_review_list_is_available_and_fails_as_missing_roles() {
    let (_temp, request, graphql_marker) = collect_request_with_fake_github("[[]]", "");
    assert_eq!(request["source_unavailable"], false);
    assert_eq!(request["reviews"], json!([]));
    assert!(
        !graphql_marker.exists(),
        "empty reviews need no edit metadata"
    );

    let result = run(request);
    assert_eq!(result["status"], "unsatisfied");
    assert_eq!(result["error"], Value::Null);
    assert_eq!(
        result["missing"].as_array().expect("missing roles").len(),
        2
    );
    assert!(
        result["missing"]
            .as_array()
            .expect("missing roles")
            .iter()
            .all(|role| role["reason"] == "no_approval")
    );
}

#[trace("FR-059-AC-6", "FR-063-AC-6")]
#[test]
fn malformed_review_edit_metadata_remains_source_unavailable() {
    let rest_response = concat!(
        "[[{\"id\":1,\"user\":{\"id\":101,\"type\":\"User\"},",
        "\"state\":\"APPROVED\",\"body\":\"Signed-off-meaning: reviewed\\nRole: security\",",
        "\"submitted_at\":\"2026-11-14T03:00:00Z\",",
        "\"commit_id\":\"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"}]]"
    );
    let (_temp, request, graphql_marker) = collect_request_with_fake_github(rest_response, "[]");
    assert_eq!(request["source_unavailable"], true);
    assert!(
        graphql_marker.exists(),
        "review items require edit metadata"
    );

    let result = run(request);
    assert_eq!(result["status"], "unavailable");
    assert_eq!(result["error"], "approval_source_unavailable");
}

#[trace("FR-061-AC-1", "FR-065-AC-6", "TC-526")]
#[test]
fn review_gate_requires_all_three_distinct_roles_before_merge() {
    let policy = json!({
        "protocol": "engineering-assurance.approval-policy/v1",
        "required_roles": ["security", "technical", "stakeholder"],
        "individuals": {
            "alice": [{"source": "github", "id": "101"}],
            "bob": [{"source": "github", "id": "202"}],
            "carol": [{"source": "github", "id": "404"}]
        },
        "roles": {
            "security": ["alice"],
            "technical": ["bob"],
            "stakeholder": ["carol"]
        },
        "agents": [{"source": "github", "id": "303"}]
    });
    let request_for_reviews = |reviews| {
        json!({
            "protocol": REQUEST_PROTOCOL,
            "current_head": "head-1",
            "policy": policy,
            "reviews": reviews,
            "account_types": [
                {"account": {"source": "github", "id": "101"}, "account_type": "human"},
                {"account": {"source": "github", "id": "202"}, "account_type": "human"},
                {"account": {"source": "github", "id": "404"}, "account_type": "human"}
            ]
        })
    };
    let all_roles = run(request_for_reviews(vec![
        review("101", "security", "APPROVED", false),
        review("202", "technical", "APPROVED", false),
        review("404", "stakeholder", "APPROVED", false),
    ]));
    assert_eq!(all_roles["status"], "satisfied");
    assert_eq!(
        all_roles["assignments"]
            .as_array()
            .expect("assignments")
            .len(),
        3
    );

    let missing_stakeholder = run(request_for_reviews(vec![
        review("101", "security", "APPROVED", false),
        review("202", "technical", "APPROVED", false),
    ]));
    assert_eq!(missing_stakeholder["status"], "unsatisfied");
    assert_eq!(missing_stakeholder["missing"][0]["role"], "stakeholder");
}

#[trace("FR-063-AC-1")]
#[test]
fn an_empty_required_role_fails_visibly() {
    let mut request = request(Vec::new());
    request["policy"]["required_roles"]
        .as_array_mut()
        .expect("required roles")
        .push(json!("stakeholder"));
    request["policy"]["roles"]["stakeholder"] = json!([]);

    let result = run(request);
    assert_eq!(result["status"], "unsatisfied");
    assert_eq!(result["error"], "approval_role_unstaffed");
    assert_eq!(result["missing"][0]["role"], "stakeholder");
}
