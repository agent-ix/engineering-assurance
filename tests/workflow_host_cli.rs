// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Live ix-flow lifecycle coverage for the Rust workflow-host boundary.

use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use engineering_assurance::{
    discovery::expected_workflows,
    workflow::{REQUEST_PROTOCOL, RESULT_PROTOCOL},
};
use ix_trace_rs::trace;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(name: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "engineering-assurance-workflow-{name}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("unique state directory must be creatable");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn skill_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("engineering_assurance/skills/assurance-onboarding")
        .canonicalize()
        .expect("canonical skill fixture must resolve")
}

fn binding(run_id: &str) -> Value {
    json!({
        "run_id": run_id,
        "repository_id": "fictional-repository@revision-1",
        "workflow": "architecture-evaluation",
        "workflow_version": "0.1.0",
        "decision_boundary": "one fictional architecture boundary",
        "decision_owner": "architecture-owner"
    })
}

fn request(state_dir: &Path, run_id: &str, operation: &str, choice: Option<&str>) -> Value {
    let mut request = json!({
        "protocol": REQUEST_PROTOCOL,
        "operation": operation,
        "state_dir": state_dir,
        "skill_root": skill_root(),
        "ix_flow_executable": "ix-flow",
        "binding": binding(run_id)
    });
    if let Some(choice) = choice {
        request["choice"] = Value::String(choice.to_owned());
    }
    request
}

fn run_host(request: &Value) -> Output {
    run_host_bytes(&serde_json::to_vec(request).expect("request must serialize"))
}

fn run_host_bytes(encoded: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .arg("workflow-host")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Cargo-built Engineering Assurance CLI must start");
    child
        .stdin
        .take()
        .expect("piped stdin must exist")
        .write_all(encoded)
        .expect("request must be writable");
    child.wait_with_output().expect("CLI must terminate")
}

fn result(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout must contain one JSON value")
}

fn ix_flow(state_dir: &Path, arguments: &[&str]) -> Value {
    let payload = ix_flow_raw(state_dir, arguments);
    assert_eq!(payload["ok"], true, "ix-flow command failed: {payload}");
    payload
}

fn ix_flow_raw(state_dir: &Path, arguments: &[&str]) -> Value {
    let output = Command::new("ix-flow")
        .args(arguments)
        .args(["--state-dir", state_dir.to_str().expect("UTF-8 state path")])
        .arg("--json")
        .output()
        .expect("the accepted ix-flow candidate must be installed");
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("ix-flow stdout must be JSON");
    payload
}

fn add_item(state_dir: &Path, run_id: &str, kind: &str, item: &Value) {
    ix_flow(
        state_dir,
        &[
            "add-item",
            run_id,
            kind,
            "--item",
            &serde_json::to_string(item).expect("item must serialize"),
        ],
    );
}

fn prepare_decision_ready(state_dir: &Path, run_id: &str) {
    let started = run_host(&request(state_dir, run_id, "start_or_resume", None));
    assert!(
        started.status.success(),
        "{}",
        String::from_utf8_lossy(&started.stderr)
    );
    populate_decision_ready(state_dir, run_id);
}

fn populate_decision_ready(state_dir: &Path, run_id: &str) {
    populate_decision_inputs(state_dir, run_id);
    ix_flow(state_dir, &["advance", run_id, "decision_ready"]);
}

fn populate_decision_inputs(state_dir: &Path, run_id: &str) {
    ix_flow(
        state_dir,
        &[
            "record-answers",
            run_id,
            "architecture",
            "--answers",
            r#"{"scope":"fictional component","description_path":"spec/AD-001.md","concerns":["retained response"],"owner":"architecture-owner"}"#,
        ],
    );
    ix_flow(state_dir, &["advance", run_id, "scenarios_ready"]);
    add_item(
        state_dir,
        run_id,
        "artifact_validation",
        &json!({
            "id": "artifact-1",
            "artifact_type": "ArchitectureDescription",
            "path": "spec/AD-001.md",
            "valid": true
        }),
    );
    add_item(
        state_dir,
        run_id,
        "architecture_scenario",
        &json!({
            "id": "scenario-1",
            "concern": "retained response",
            "stimulus": "accepted request",
            "environment": "fictional runtime",
            "response": "retain result",
            "measure": "all retained"
        }),
    );
    add_item(
        state_dir,
        run_id,
        "review_validation",
        &json!({
            "id": "review-1",
            "artifact_type": "SpecReview",
            "analysis": "architecture-evaluation",
            "path": "reviews/SR-001.md",
            "subject_path": "spec/AD-001.md",
            "valid": true
        }),
    );
    add_item(
        state_dir,
        run_id,
        "operator_observation",
        &json!({"id": "observation-1", "elapsed_minutes": 3, "command_count": 4}),
    );
}

fn tree_digest(root: &Path) -> Vec<u8> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)
            .expect("state directory must be readable")
            .collect::<Result<Vec<_>, _>>()
            .expect("state entries must be readable")
        {
            let kind = entry.file_type().expect("state entry must be classifiable");
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                files.push(entry.path());
            }
        }
    }
    files.sort();
    let mut digest = Sha256::new();
    for path in files {
        digest.update(
            path.strip_prefix(root)
                .expect("state path must be relative")
                .as_os_str()
                .as_encoded_bytes(),
        );
        digest.update(fs::read(path).expect("state file must be readable"));
    }
    digest.finalize().to_vec()
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir(destination).expect("copied skill directory must be creatable");
    for entry in fs::read_dir(source)
        .expect("source skill must be readable")
        .collect::<Result<Vec<_>, _>>()
        .expect("source skill entries must be readable")
    {
        let target = destination.join(entry.file_name());
        if entry
            .file_type()
            .expect("source skill entry must be classifiable")
            .is_dir()
        {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("source skill file must be copied");
        }
    }
}

fn run_state_path(state_dir: &Path, run_id: &str) -> PathBuf {
    state_dir.join("instances").join(format!("{run_id}.json"))
}

fn gate_token(payload: &Value, outcome: &str) -> String {
    let selected = payload["open_gates"]
        .as_array()
        .expect("open gates must be an array")
        .iter()
        .filter(|gate| gate["to"] == outcome)
        .collect::<Vec<_>>();
    let [gate] = selected.as_slice() else {
        panic!("expected exactly one {outcome} gate: {payload}");
    };
    gate["token"]
        .as_str()
        .expect("gate token must be a string")
        .to_owned()
}

#[trace("TC-107", "FR-016-AC-3", "FR-016-CON-1", "FR-016-CON-5")]
#[trace("TC-026", "FR-005-AC-1", "US-003-EX-1")]
#[test]
fn tc_107_new_resume_and_pristine_interruption_recovery_preserve_binding() {
    let state = TestDirectory::new("resume");
    let first = run_host(&request(state.path(), "new-run", "start_or_resume", None));
    assert!(first.status.success());
    let first_result = result(&first);
    assert_eq!(first_result["protocol"], RESULT_PROTOCOL);
    assert_eq!(first_result["snapshot"]["phase"], "capture");
    assert_eq!(first_result["snapshot"]["state_version"], 1);

    let resumed = run_host(&request(state.path(), "new-run", "start_or_resume", None));
    assert!(resumed.status.success());
    assert_eq!(result(&resumed), first_result);

    // Resuming an unchanged run proves idempotence but not that a completed
    // phase survives. FR-005-AC-1 is about the operator who stopped partway:
    // the run is advanced here first, so a resume that silently restarted the
    // workflow would report `capture` again and fail instead of passing as an
    // identical snapshot.
    ix_flow(
        state.path(),
        &[
            "record-answers",
            "new-run",
            "architecture",
            "--answers",
            r#"{"scope":"fictional component","description_path":"spec/AD-001.md","concerns":["retained response"],"owner":"architecture-owner"}"#,
        ],
    );
    ix_flow(state.path(), &["advance", "new-run", "scenarios_ready"]);
    let advanced = run_host(&request(state.path(), "new-run", "start_or_resume", None));
    assert!(
        advanced.status.success(),
        "{}",
        String::from_utf8_lossy(&advanced.stderr)
    );
    let advanced_snapshot = result(&advanced)["snapshot"].clone();
    assert_eq!(advanced_snapshot["phase"], "scenarios_ready");
    assert!(
        advanced_snapshot["state_version"]
            .as_u64()
            .expect("a state version must be an integer")
            > first_result["snapshot"]["state_version"]
                .as_u64()
                .expect("a state version must be an integer"),
        "resume must observe the run ix-flow advanced, not a replacement"
    );

    ix_flow(
        state.path(),
        &[
            "run",
            "architecture-evaluation",
            "--path",
            skill_root().to_str().expect("UTF-8 skill path"),
            "--id",
            "interrupted-run",
        ],
    );
    let recovered = run_host(&request(
        state.path(),
        "interrupted-run",
        "start_or_resume",
        None,
    ));
    assert!(recovered.status.success());
    let status = ix_flow(state.path(), &["status", "interrupted-run"]);
    assert_eq!(status["data"]["stateVersion"], 1);
    assert_eq!(
        status["data"]["items"]["run_binding"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
}

/// The phase and transition shape of one canonical workflow definition.
#[derive(serde::Deserialize)]
struct Definition {
    phases: Vec<DefinitionPhase>,
    transitions: Vec<DefinitionTransition>,
}

/// One declared phase of a canonical workflow.
#[derive(serde::Deserialize)]
struct DefinitionPhase {
    name: String,
    #[serde(default)]
    terminal: bool,
}

/// One declared transition of a canonical workflow.
#[derive(serde::Deserialize)]
struct DefinitionTransition {
    to: String,
    #[serde(rename = "defaultGate")]
    default_gate: String,
}

#[trace("TC-027", "FR-005-AC-2")]
#[test]
fn tc_027_every_canonical_terminal_transition_is_human_gated() {
    // The population is taken from the promoted workflow names rather than from
    // whatever the directory happens to list. A check written over a listing
    // passes over an empty or renamed workflows directory, which is the exact
    // condition that would leave a terminal transition ungated and unnoticed.
    let workflows = expected_workflows();
    assert_eq!(workflows.len(), 4, "the promoted set must not be empty");
    let root = skill_root().join("workflows");

    for workflow in &workflows {
        let path = root.join(workflow).join("def.yaml");
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let definition: Definition = yaml_serde::from_slice(&bytes)
            .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()));

        let terminal = definition
            .phases
            .iter()
            .filter(|phase| phase.terminal)
            .map(|phase| phase.name.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            terminal.len(),
            2,
            "{workflow} must declare an accepting and a rejecting terminal phase"
        );

        let into_terminal = definition
            .transitions
            .iter()
            .filter(|transition| terminal.contains(&transition.to))
            .collect::<Vec<_>>();
        assert_eq!(
            into_terminal
                .iter()
                .map(|transition| transition.to.clone())
                .collect::<BTreeSet<_>>(),
            terminal,
            "{workflow} must reach every terminal phase it declares"
        );
        // A single `auto` here is the whole defect: ix-flow would advance the
        // run to its terminal outcome with no person selecting it, and every
        // downstream record would still read as an owner decision.
        for transition in into_terminal {
            assert_eq!(
                transition.default_gate, "hitl",
                "{workflow} leaves the transition to {} ungated",
                transition.to
            );
        }
    }
}

#[trace("TC-107", "FR-016-AC-3", "FR-016-CON-1", "FR-016-CON-6")]
#[test]
fn tc_107_stale_pristine_run_is_refused_before_binding_mutation() {
    let state = TestDirectory::new("stale-pristine");
    let stale_skill = state.path().join("stale-skill");
    copy_tree(&skill_root(), &stale_skill);
    let definition = stale_skill.join("workflows/architecture-evaluation/def.yaml");
    let source = fs::read_to_string(&definition).expect("copied definition must be readable");
    assert!(source.contains("version: 0.1.0"));
    fs::write(
        &definition,
        source.replace("version: 0.1.0", "version: 0.0.9"),
    )
    .expect("copied definition version must be replaceable");
    ix_flow(
        state.path(),
        &[
            "run",
            "architecture-evaluation",
            "--path",
            stale_skill.to_str().expect("UTF-8 copied skill path"),
            "--id",
            "stale-pristine-run",
        ],
    );
    let before = tree_digest(state.path());

    let output = run_host(&request(
        state.path(),
        "stale-pristine-run",
        "start_or_resume",
        None,
    ));
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(result(&output)["code"], "workflow_binding_invalid");
    assert_eq!(tree_digest(state.path()), before);
    assert_eq!(
        ix_flow(state.path(), &["status", "stale-pristine-run"])["data"]["stateVersion"],
        0
    );
}

#[trace("TC-107", "FR-016-AC-3", "FR-016-CON-1", "FR-016-CON-6")]
#[test]
fn tc_107_broken_event_chain_is_refused_without_state_mutation() {
    let state = TestDirectory::new("broken-chain");
    let run_id = "broken-chain-run";
    let started = run_host(&request(state.path(), run_id, "start_or_resume", None));
    assert!(started.status.success());

    let state_path = run_state_path(state.path(), run_id);
    let mut persisted: Value = serde_json::from_slice(
        &fs::read(&state_path).expect("persisted ix-flow run must be readable"),
    )
    .expect("persisted ix-flow run must be JSON");
    let events = persisted["events"]
        .as_array_mut()
        .expect("persisted events must be an array");
    let first = events
        .first_mut()
        .expect("persisted run must contain a creation event");
    first["hash"] = Value::String("0".repeat(64));
    fs::write(
        &state_path,
        serde_json::to_vec_pretty(&persisted).expect("tampered run must serialize"),
    )
    .expect("test must be able to tamper the persisted fixture");
    let before = tree_digest(state.path());

    let output = run_host(&request(state.path(), run_id, "start_or_resume", None));
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(result(&output)["code"], "ix_flow_command_failed");
    assert!(String::from_utf8_lossy(&output.stderr).contains("event_chain_invalid"));
    assert_eq!(tree_digest(state.path()), before);
}

#[trace("TC-107", "FR-016-AC-3", "FR-016-CON-1", "FR-016-CON-6")]
#[trace("TC-048", "FR-005-AC-7")]
#[test]
fn tc_107_binding_and_phase_refusals_leave_ix_flow_history_unchanged() {
    let state = TestDirectory::new("refusal");
    let started = run_host(&request(state.path(), "bound-run", "start_or_resume", None));
    assert!(started.status.success());
    let before = tree_digest(state.path());

    // Every field of the binding is exercised, not one of them. The binding
    // exists to keep one run from governing a second decision boundary, and a
    // reused run id that changes only the boundary or the definition version is
    // the contamination the criterion names — each was refused by code no
    // assertion reached.
    for (field, value) in [
        ("repository_id", "other@revision"),
        ("decision_boundary", "a different architecture boundary"),
        ("workflow_version", "0.1.1"),
        ("workflow", "assurance-intake"),
    ] {
        let mut mismatched = request(state.path(), "bound-run", "start_or_resume", None);
        mismatched["binding"][field] = Value::String(value.to_owned());
        let mismatch = run_host(&mismatched);
        assert_eq!(
            mismatch.status.code(),
            Some(2),
            "a changed {field} was accepted"
        );
        assert_eq!(result(&mismatch)["code"], "workflow_binding_invalid");
        assert_eq!(
            tree_digest(state.path()),
            before,
            "a refused {field} mismatch changed ix-flow state"
        );
    }

    let invalid_phase = run_host(&request(
        state.path(),
        "bound-run",
        "decide",
        Some("accept"),
    ));
    assert_eq!(invalid_phase.status.code(), Some(2));
    assert_eq!(
        result(&invalid_phase)["code"],
        "workflow_transition_invalid"
    );
    assert_eq!(tree_digest(state.path()), before);
}

#[trace("TC-107", "FR-016-AC-3", "FR-016-CON-1", "FR-016-CON-5")]
#[trace("TC-003", "StR-001-VC-3")]
#[trace("TC-028", "FR-005-AC-3", "US-003-EX-2")]
#[trace("TC-029", "FR-005-AC-4")]
#[trace("TC-047", "FR-005-AC-6")]
#[test]
fn tc_107_explicit_decisions_are_human_attributed_idempotent_and_conflict_safe() {
    for (run_id, choice, outcome, opposite_outcome) in [
        ("accepted-run", "accept", "accepted", "rejected"),
        ("rejected-run", "reject", "rejected", "accepted"),
    ] {
        let state = TestDirectory::new(run_id);
        prepare_decision_ready(state.path(), run_id);

        // Nothing may be acknowledged before the owner decides. Without this,
        // an already-acknowledged gate would satisfy every assertion below and
        // the run would report an attributed decision nobody made.
        let before = ix_flow(state.path(), &["status", run_id]);
        assert!(
            acknowledgements(&before).is_empty(),
            "a decision was already acknowledged before the owner chose"
        );

        let before_choice = tree_digest(state.path());
        let no_choice = run_host(&request(state.path(), run_id, "decide", None));
        assert!(no_choice.status.success());
        let waiting = result(&no_choice)["snapshot"].clone();
        assert_eq!(waiting["phase"], "decision_ready");
        // A run left at its gate must also have opened no gate: an open gate is
        // a token an automated caller could acknowledge, so "non-terminal" and
        // "nothing is pending acknowledgement" are two different properties.
        assert_eq!(
            waiting["open_gates"].as_array().map(Vec::len),
            Some(0),
            "an undecided run must leave no gate open"
        );
        assert_eq!(tree_digest(state.path()), before_choice);

        let decided = run_host(&request(state.path(), run_id, "decide", Some(choice)));
        assert!(
            decided.status.success(),
            "{}",
            String::from_utf8_lossy(&decided.stderr)
        );
        let event = result(&decided)["decision"].clone();
        assert_eq!(event["choice"], choice);
        assert_eq!(event["outcome"], outcome);
        assert_eq!(event["owner"], "architecture-owner");
        assert_eq!(event["run_id"], run_id);
        assert_eq!(event["workflow"], "architecture-evaluation");
        assert_eq!(event["workflow_version"], "0.1.0");
        assert!(
            event["timestamp"]
                .as_str()
                .expect("a decision timestamp must be a string")
                .ends_with('Z'),
            "the decision timestamp must be recorded in universal time"
        );
        let after_choice = tree_digest(state.path());

        let repeated = run_host(&request(state.path(), run_id, "decide", Some(choice)));
        assert!(repeated.status.success());
        assert_eq!(result(&repeated)["decision"], event);
        assert_eq!(tree_digest(state.path()), after_choice);

        let opposite = if choice == "accept" {
            "reject"
        } else {
            "accept"
        };
        let conflict = run_host(&request(state.path(), run_id, "decide", Some(opposite)));
        assert_eq!(conflict.status.code(), Some(2));
        assert_eq!(result(&conflict)["code"], "workflow_decision_conflict");
        assert_eq!(tree_digest(state.path()), after_choice);

        let status = ix_flow(state.path(), &["status", run_id]);
        assert_eq!(acknowledgements(&status).len(), 1);
        assert_eq!(status["data"]["phase"], outcome);
        // The opposite outcome must be absent from the history, not merely
        // absent from the current phase: a run that reached the other terminal
        // state and was walked back would still read correctly at the end.
        assert!(
            !status["data"]["events"]
                .as_array()
                .expect("events must be an array")
                .iter()
                .any(|item| item["payload"]["to"] == opposite_outcome),
            "the opposite terminal outcome appears in the run history"
        );
    }
}

/// Every human gate acknowledgement recorded in one ix-flow status payload.
fn acknowledgements(status: &Value) -> Vec<&Value> {
    status["data"]["events"]
        .as_array()
        .expect("events must be an array")
        .iter()
        .filter(|item| item["kind"] == "gate.acknowledged")
        .collect()
}

#[trace("TC-107", "FR-016-AC-3", "FR-016-CON-1", "FR-016-CON-5")]
#[test]
fn tc_107_retries_after_gate_defer_and_acknowledgement_converge() {
    for interrupted_after_ack in [false, true] {
        let state = TestDirectory::new(if interrupted_after_ack {
            "after-ack"
        } else {
            "after-defer"
        });
        let run_id = if interrupted_after_ack {
            "after-ack-run"
        } else {
            "after-defer-run"
        };
        prepare_decision_ready(state.path(), run_id);
        let deferred = ix_flow_raw(state.path(), &["advance", run_id, "accepted"]);
        assert_eq!(deferred["state"], "gate_deferred");
        if interrupted_after_ack {
            let token = gate_token(&deferred, "accepted");
            ix_flow(
                state.path(),
                &[
                    "ack",
                    run_id,
                    &token,
                    "--reviewer",
                    "architecture-owner",
                    "--kind",
                    "decision",
                    "--note",
                    "accept",
                ],
            );
        }
        let completed = run_host(&request(state.path(), run_id, "decide", Some("accept")));
        assert!(
            completed.status.success(),
            "{}",
            String::from_utf8_lossy(&completed.stderr)
        );
        assert_eq!(result(&completed)["decision"]["outcome"], "accepted");
    }
}

#[trace("TC-107", "FR-016-AC-3", "FR-016-CON-5", "FR-016-CON-7")]
#[test]
fn tc_107_incompatible_unavailable_and_malformed_inputs_fail_before_state() {
    let state = TestDirectory::new("host-errors");
    let before = tree_digest(state.path());

    let mut wrong_version = request(state.path(), "wrong-version", "start_or_resume", None);
    wrong_version["ix_flow_executable"] =
        Value::String(env!("CARGO_BIN_EXE_engineering-assurance").to_owned());
    let output = run_host(&wrong_version);
    assert_eq!(result(&output)["code"], "ix_flow_version_incompatible");
    assert_eq!(tree_digest(state.path()), before);

    let mut unavailable = request(state.path(), "unavailable", "start_or_resume", None);
    unavailable["ix_flow_executable"] =
        Value::String(state.path().join("missing-ix-flow").display().to_string());
    let output = run_host(&unavailable);
    assert_eq!(result(&output)["code"], "ix_flow_unavailable");
    assert_eq!(tree_digest(state.path()), before);

    for encoded in [b"{not-json".to_vec(), vec![b' '; 8 * 1024 * 1024 + 1]] {
        let output = run_host_bytes(&encoded);
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(result(&output)["code"], "workflow_host_request_invalid");
    }
    assert_eq!(tree_digest(state.path()), before);

    let production = include_str!("../src/workflow_host.rs")
        .split("#[cfg(test)]")
        .next()
        .expect("production source must precede its test module");
    let process_adapter = include_str!("../src/process_host.rs");
    let host_sources = format!("{production}\n{process_adapter}");
    let forbidden = [
        "Command::new(\"sh\")",
        "Command::new(\"bash\")",
        "OsString::from(\"--gate\")",
        "OsString::from(\"--gate-mode\")",
        ".next_actions[",
        "_legacy_next_actions[",
        "fs::read(state_dir",
        "state_dir.join(",
    ];
    assert_eq!(
        forbidden
            .into_iter()
            .filter(|needle| host_sources.contains(needle))
            .collect::<BTreeSet<_>>(),
        BTreeSet::new()
    );
    assert_eq!(production.matches("process_host::run(").count(), 1);
    assert_eq!(process_adapter.matches("Command::new(").count(), 1);
    assert!(process_adapter.contains("Command::new(executable)"));
    assert_eq!(production.matches("next_actions").count(), 4);
    assert_eq!(production.matches("fs::read(").count(), 1);
    assert_eq!(production.matches("fs::canonicalize(").count(), 1);
}

#[trace("TC-107", "FR-016-AC-3", "FR-016-CON-1", "FR-016-CON-5")]
#[trace("TC-030", "FR-005-AC-5")]
#[test]
fn tc_107_automatic_terminal_gate_configuration_is_refused_without_mutation() {
    let state = TestDirectory::new("automatic-gate");
    let run_id = "automatic-gate-run";
    ix_flow(
        state.path(),
        &[
            "run",
            "architecture-evaluation",
            "--path",
            skill_root().to_str().expect("UTF-8 skill path"),
            "--id",
            run_id,
            "--gate",
            "decision_ready->accepted=auto",
        ],
    );
    let mut item = binding(run_id);
    item["id"] = Value::String("binding".to_owned());
    add_item(state.path(), run_id, "run_binding", &item);
    populate_decision_inputs(state.path(), run_id);
    let blocked = ix_flow_raw(state.path(), &["advance", run_id, "decision_ready"]);
    assert_eq!(blocked["state"], "invariant_failed");
    assert_eq!(
        blocked["error"]["details"]["invariantCode"],
        "terminal_gate_override"
    );
    let before = tree_digest(state.path());

    let output = run_host(&request(state.path(), run_id, "decide", Some("accept")));
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(result(&output)["code"], "workflow_decision_conflict");
    assert_eq!(tree_digest(state.path()), before);
    assert_eq!(
        ix_flow(state.path(), &["status", run_id])["data"]["phase"],
        "scenarios_ready"
    );
}
