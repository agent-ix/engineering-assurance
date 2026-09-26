//! Port of `tests/test_workflows.py`; see `main.rs`.
use super::common::{package_root, read, root, yaml};
use ix_trace_rs::trace;
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn pilot() -> PathBuf {
    root().join("pilots/assurance-workflows")
}

fn canonical() -> PathBuf {
    package_root().join("skills/assurance-onboarding")
}

/// Every `<dir>/*/def.yaml`, keyed by the workflow directory name.
fn load_definitions(dir: &Path) -> Map<String, Value> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .map(|e| e.expect("dir entry").path().join("def.yaml"))
        .filter(|p| p.is_file())
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let name = p
                .parent()
                .and_then(Path::file_name)
                .and_then(|n| n.to_str())
                .expect("dir name")
                .to_string();
            (name, yaml(&read(&p)))
        })
        .collect()
}

fn definitions() -> Map<String, Value> {
    load_definitions(&pilot().join("workflows"))
}

fn names(map: &Map<String, Value>) -> BTreeSet<String> {
    map.keys().cloned().collect()
}

fn expected_names() -> BTreeSet<String> {
    [
        "assurance-intake",
        "architecture-evaluation",
        "measurement-promotion",
        "change-assurance",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn run_invariant(tmp: &Path, name: &str, instance: &Value) -> Value {
    let module = format!("file://{}", pilot().join("scripts/invariants.js").display());
    let script = tmp.join("check.mjs");
    let text = format!(
        "import {{ invariants }} from {};\nconst result = await invariants[{}]({{ instance: {} }});\nconsole.log(JSON.stringify(result));\n",
        Value::String(module),
        Value::String(name.to_string()),
        instance
    );
    std::fs::write(&script, text).expect("write script");
    let out = Command::new("node")
        .arg(&script)
        .output()
        .expect("run node");
    assert!(
        out.status.success(),
        "node failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("json output")
}

fn ix_flow() -> String {
    if let Some(v) = std::env::var_os("IX_FLOW_BIN").filter(|v| !v.is_empty()) {
        return v.to_string_lossy().into_owned();
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .map(|d| d.join("ix-flow"))
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().into_owned())
        .expect("ix-flow is required by the integration contract")
}

fn assert_ix_flow_loads(search: &Path, id_prefix: &str, tmp: &Path) {
    let exe = ix_flow();
    for name in definitions().keys() {
        let out = Command::new(&exe)
            .args(["run", name, "--path"])
            .arg(search)
            .args(["--id", &format!("{id_prefix}-{name}"), "--state-dir"])
            .arg(tmp.join(name))
            .arg("--json")
            .output()
            .expect("run ix-flow");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let payload: Value = serde_json::from_slice(&out.stdout).expect("json");
        assert_eq!(payload["data"]["defName"], json!(name));
    }
}

#[test]
#[trace("TC-027", "FR-005-AC-2")]
fn every_terminal_transition_is_human_gated() {
    for definition in definitions().values() {
        let terminal: BTreeSet<&str> = definition["phases"]
            .as_array()
            .expect("phases")
            .iter()
            .filter(|p| p.get("terminal").is_some_and(|t| t.as_bool() == Some(true)))
            .map(|p| p["name"].as_str().expect("name"))
            .collect();
        let transitions: Vec<&Value> = definition["transitions"]
            .as_array()
            .expect("transitions")
            .iter()
            .filter(|t| terminal.contains(t["to"].as_str().expect("to")))
            .collect();
        let targets: BTreeSet<&str> = transitions
            .iter()
            .map(|t| t["to"].as_str().expect("to"))
            .collect();
        assert_eq!(targets, terminal);
        assert!(
            transitions
                .iter()
                .all(|t| t["defaultGate"] == json!("hitl"))
        );
        assert!(transitions.iter().all(|t| t["invariants"] == json!([])));
    }
}

#[test]
#[trace("TC-030", "FR-005-AC-5")]
fn terminal_gate_override_fails() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let result = run_invariant(
        tmp.path(),
        "shared.terminal_gates",
        &json!({
            "defName": "change-assurance",
            "items": {},
            "gateConfig": {
                "decision_ready->approved": "auto",
                "decision_ready->rejected": "hitl",
            },
        }),
    );
    assert_eq!(result["ok"], json!(false));
    assert_eq!(result["code"], json!("terminal_gate_override"));
}

#[test]
#[trace("TC-160", "FR-025-AC-2")]
fn adjacent_measurement_promotion_passes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let request = json!({
        "interviewId": "promotion",
        "plan_path": "fixtures/MP-001.md",
        "definition_version": "v1",
        "prior_stage": "baseline",
        "proposed_stage": "branch-comparison",
    });
    let evidence = json!({
        "plan_path": "fixtures/MP-001.md",
        "definition_version": "v1",
        "prior_stage": "baseline",
        "proposed_stage": "branch-comparison",
        "stability": "fixed fixture",
        "decision_yield": "one changed decision",
        "limitations": "fictional sample",
        "owner": "measurement-owner",
    });
    let result = run_invariant(
        tmp.path(),
        "measurement.promotion_ready",
        &json!({
            "defName": "measurement-promotion",
            "items": {
                "promotion_request": [request],
                "promotion_evidence": [evidence],
            },
        }),
    );
    assert_eq!(result, json!(true));
}

#[test]
#[trace("TC-035", "FR-007-AC-1")]
#[trace("TC-040", "NFR-003-AC-3")]
fn ix_flow_can_load_every_definition() {
    let tmp = tempfile::tempdir().expect("tempdir");
    assert_ix_flow_loads(&pilot(), "test", tmp.path());
}

#[test]
#[trace("TC-036", "FR-007-AC-2")]
fn pilot_and_canonical_workflows_are_equivalent() {
    let pilot = definitions();
    let canonical = load_definitions(&canonical().join("workflows"));
    assert_eq!(names(&pilot), expected_names());
    assert_eq!(canonical, pilot);
}

/// The pilot path keeps exactly the four workflows present before promotion:
/// a fifth, or a missing one, fails.
#[test]
#[trace("TC-043", "FR-007-AC-4", "FR-007-CON-1")]
fn compatible_pilot_inventory_is_exact() {
    assert_eq!(names(&definitions()), expected_names());
}

#[test]
#[trace("TC-036", "FR-007-AC-2")]
fn pilot_invariant_surface_delegates_to_canonical() {
    let compatibility = read(&pilot().join("scripts/invariants.js"));
    assert!(compatibility.contains("engineering_assurance/skills/assurance-onboarding"));
}

#[test]
#[trace("TC-011", "FR-002-AC-3")]
#[trace("TC-035", "FR-007-AC-1")]
fn ix_flow_can_load_every_canonical_definition() {
    let tmp = tempfile::tempdir().expect("tempdir");
    assert_ix_flow_loads(&canonical(), "canonical", tmp.path());
}

/// The promotion workflow accepts a typed checker result and the profile's
/// policy as run items, and an accepted verdict still reaches `promoted` only
/// through the human-gated terminal transition that human decisions own.
#[test]
#[trace("TC-164", "FR-025-AC-6")]
fn measurement_promotion_declares_checker_and_policy_items() {
    let all = definitions();
    let definition = &all["measurement-promotion"];
    let schemas = &definition["itemSchemas"];
    assert_eq!(
        schemas["measurement_verdict"]["required"],
        json!([
            "id",
            "schema",
            "planId",
            "definitionVersion",
            "verdict",
            "reasons",
            "claimed",
            "candidate",
            "decisions",
            "findings",
            "regressedRuns",
            "orderSource",
            "counts"
        ])
    );
    assert_eq!(
        schemas["measurement_policy"]["required"],
        json!(["id", "profile_path", "mode", "stages"])
    );
    let transitions = definition["transitions"].as_array().expect("transitions");
    let evidence_ready: Vec<&Value> = transitions
        .iter()
        .filter(|t| t["from"] == json!("evidence_ready"))
        .collect();
    let targets: Vec<&Value> = evidence_ready.iter().map(|t| &t["to"]).collect();
    assert_eq!(targets, vec![&json!("decision_ready")]);
    let invariants = evidence_ready[0]["invariants"]
        .as_array()
        .expect("invariants");
    assert!(invariants.contains(&json!("measurement.promotion_ready")));
    assert!(invariants.contains(&json!("shared.exceptions_ready")));
    let terminal: Vec<&Value> = transitions
        .iter()
        .filter(|t| t["from"] == json!("decision_ready"))
        .collect();
    let set: BTreeSet<&str> = terminal
        .iter()
        .map(|t| t["to"].as_str().expect("to"))
        .collect();
    assert_eq!(set, BTreeSet::from(["promoted", "not_promoted"]));
    assert!(terminal.iter().all(|t| t["defaultGate"] == json!("hitl")));
    let canonical = yaml(&read(
        &canonical().join("workflows/measurement-promotion/def.yaml"),
    ));
    assert_eq!(&canonical, definition);
}

/// Through the pilot surface ix-flow loads, a `require` policy for the proposed
/// stage refuses a rejected checker result with a typed code.
#[test]
#[trace("TC-164", "FR-025-AC-3")]
fn required_checker_refuses_rejected_promotion() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let evidence = json!({
        "plan_path": "fixtures/MP-001.md",
        "definition_version": "v1",
        "prior_stage": "target",
        "proposed_stage": "gate",
        "stability": "fixed fixture",
        "decision_yield": "one changed decision",
        "limitations": "fictional sample",
        "owner": "measurement-owner",
        "plan_id": "MP-001",
        "candidate": "collection-2",
    });
    let result = run_invariant(
        tmp.path(),
        "measurement.promotion_ready",
        &json!({
            "defName": "measurement-promotion",
            "items": {
                "promotion_request": [{
                    "interviewId": "promotion",
                    "plan_path": "fixtures/MP-001.md",
                    "definition_version": "v1",
                    "prior_stage": "target",
                    "proposed_stage": "gate",
                }],
                "promotion_evidence": [evidence],
                "measurement_policy": [{
                    "profile_path": "fixtures/AP-001.md",
                    "mode": "require",
                    "stages": ["gate"],
                }],
                "measurement_verdict": [{
                    "schema": "quoin.measurement-verdict.v1",
                    "planId": "MP-001",
                    "definitionVersion": "v1",
                    "verdict": "reject",
                    "reasons": ["rule_not_met"],
                    "claimed": null,
                    "candidate": "collection-2",
                    "decisions": [],
                    "findings": [],
                    "regressedRuns": [],
                    "orderSource": "git-first-parent-add",
                    "counts": {},
                }],
            },
        }),
    );
    assert_eq!(result["ok"], json!(false));
    assert_eq!(result["code"], json!("promotion_checker_not_accepted"));
    assert_eq!(result["details"]["checker"]["verdict"], json!("reject"));
}
