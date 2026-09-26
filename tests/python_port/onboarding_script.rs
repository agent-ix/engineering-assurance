//! Port of `tests/test_onboarding_script.py`: `onboard.js` driven through node.
//!
//! Each test returns early (printing a SKIP line) when node is not installed,
//! as the Python suite skipped.
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};
use tempfile::TempDir;

use super::common::{package_root, read, schema};

fn onboard_js() -> PathBuf {
    package_root()
        .join("skills")
        .join("assurance-onboarding")
        .join("scripts")
        .join("onboard.js")
}

/// Path of `node` on `PATH`, if installed.
fn node() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("node"))
        .find(|candidate| candidate.is_file())
}

macro_rules! require_node {
    () => {
        match node() {
            Some(node) => node,
            None => {
                eprintln!("SKIP: node is not installed");
                return;
            }
        }
    };
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("utf-8 stdout")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("utf-8 stderr")
}

/// Run `node script args...` and require success (Python `check=True`).
fn run_ok(node: &Path, script: &Path, args: &[&Path], extra: &[&str]) -> Output {
    let output = Command::new(node)
        .arg(script)
        .args(args)
        .args(extra)
        .output()
        .expect("node runs");
    assert!(output.status.success(), "node failed: {}", stderr(&output));
    output
}

fn run_onboard_json(node: &Path, tmp: &Path) -> Value {
    let target_repo = tmp.join("consumer");
    fs::create_dir(&target_repo).expect("mkdir consumer");
    let output = Command::new(node)
        .arg(onboard_js())
        .arg("--repo")
        .arg(&target_repo)
        .arg("--json")
        .output()
        .expect("node runs");
    assert!(output.status.success(), "node failed: {}", stderr(&output));
    serde_json::from_str(&stdout(&output)).expect("json report")
}

fn contains(list: &Value, item: &Value) -> bool {
    list.as_array().expect("array").contains(item)
}

fn any_warning_mentions(checklist: &Value, needle: &str) -> bool {
    checklist["warnings"]
        .as_array()
        .expect("warnings array")
        .iter()
        .any(|w| w.as_str().is_some_and(|s| s.contains(needle)))
}

/// A throwaway module copy with one probe schema, plus its `onboard.js`.
fn probe_module(tmp: &Path, probe_schema: &Value) -> PathBuf {
    let module = tmp.join("engineering_assurance");
    let script_dir = module.join("skills/assurance-onboarding/scripts");
    fs::create_dir_all(&script_dir).expect("mkdir script dir");
    fs::copy(onboard_js(), script_dir.join("onboard.js")).expect("copy onboard.js");
    fs::create_dir(module.join("skeletons")).expect("mkdir skeletons");
    fs::create_dir(module.join("schemas")).expect("mkdir schemas");
    fs::write(
        module.join("manifest.yaml"),
        "artifact_types:\n  - name: Probe\n    frontmatter_schema_ref: schemas/probe.schema.json\n",
    )
    .expect("write manifest");
    fs::write(
        module.join("schemas/probe.schema.json"),
        probe_schema.to_string(),
    )
    .expect("write schema");
    script_dir.join("onboard.js")
}

fn probe_report(node: &Path, tmp: &Path, probe_schema: &Value) -> Value {
    let script = probe_module(tmp, probe_schema);
    let target_repo = tmp.join("consumer");
    fs::create_dir(&target_repo).expect("mkdir consumer");
    let output = run_ok(
        node,
        &script,
        &[Path::new("--repo"), &target_repo],
        &["--json"],
    );
    let report: Value = serde_json::from_str(&stdout(&output)).expect("json report");
    report["artifactChecklists"]["Probe"].clone()
}

#[test]
fn onboard_js_json_checklist_lists_nested_objective_requirements() {
    // Trace: FR-020-AC-1, TC-139.
    let node = require_node!();
    let tmp = TempDir::new().unwrap();
    let report = run_onboard_json(&node, tmp.path());
    let conditional = &report["artifactChecklists"]["MeasurementPlan"]["conditionalRequired"];

    let objective_entries: Vec<&Value> = conditional
        .as_array()
        .expect("array")
        .iter()
        .filter(|entry| entry["when"] == "objective is present")
        .collect();
    assert!(!objective_entries.is_empty(), "{conditional}");
    assert!(
        objective_entries
            .iter()
            .any(|entry| contains(&entry["required"], &json!("objective.direction"))),
        "{conditional}"
    );
}

#[test]
fn onboard_js_json_checklist_lists_the_decision_rule_vocabulary() {
    // Trace: FR-021-AC-6, TC-149.
    let node = require_node!();
    let tmp = TempDir::new().unwrap();
    let report = run_onboard_json(&node, tmp.path());
    let plan = &report["artifactChecklists"]["MeasurementPlan"];
    let enums = &plan["enums"];
    assert_eq!(
        enums["statistical_design.estimator"],
        json!(["proportion", "count", "mean", "median", "ratio"])
    );
    assert_eq!(
        enums["statistical_design.decision_rule.comparator"],
        json!(["gt", "ge", "lt", "le", "eq"])
    );
    assert_eq!(
        enums["statistical_design.decision_rule.baseline"],
        json!([
            "constant-predictor",
            "prior-collection",
            "best-seen",
            "external-reference"
        ])
    );
    assert!(contains(
        &plan["exactlyOneOf"],
        &json!({
            "when": "statistical_design.decision_rule is present",
            "fields": [
                "statistical_design.decision_rule.threshold",
                "statistical_design.decision_rule.baseline",
            ],
        })
    ));
    let conditional = &plan["conditionalRequired"];
    assert!(contains(
        conditional,
        &json!({
            "when": "statistical_design.decision_rule.margin is present",
            "required": ["statistical_design.decision_rule.baseline"],
        })
    ));
    assert!(contains(
        conditional,
        &json!({
            "when": "statistical_design is present",
            "required": ["metric"],
        })
    ));
    let constraints: std::collections::HashMap<&str, &str> = plan["conditionalConstraints"]
        .as_array()
        .expect("array")
        .iter()
        .map(|entry| {
            (
                entry["when"].as_str().expect("when"),
                entry["constraint"].as_str().expect("constraint"),
            )
        })
        .collect();
    let expected_current_constraints = [
        (
            "objective.direction = higher",
            "statistical_design.decision_rule.comparator must be one of gt, ge",
        ),
        (
            "objective.direction = lower",
            "statistical_design.decision_rule.comparator must be one of lt, le",
        ),
        (
            "objective.direction = zero",
            "either (statistical_design.decision_rule.comparator must be eq) or \
             (statistical_design.decision_rule.threshold required and \
             statistical_design.decision_rule.comparator must be le and \
             statistical_design.decision_rule.threshold must be 0)",
        ),
        (
            "statistical_design.decision_rule.baseline = constant-predictor",
            "statistical_design.estimator must be proportion",
        ),
        (
            "statistical_design.decision_rule.comparator = eq",
            "statistical_design.decision_rule.baseline must not be best-seen and \
             statistical_design.decision_rule.margin must be absent and \
             statistical_design.decision_rule.margin_mode must be absent and \
             statistical_design.decision_rule.interval_level must be absent",
        ),
    ];
    for (when, constraint) in expected_current_constraints {
        assert_eq!(constraints[when], constraint, "{when}");
    }
    assert!(constraints["status = retired"].contains("legacy_retired_statistical_design"));
    assert_eq!(
        constraints["otherwise (status = retired does not hold)"],
        "statistical_design must match statistical_design"
    );
    assert_eq!(plan["warnings"], json!([]));
}

#[test]
fn onboard_js_reports_combined_then_and_warns_on_an_empty_one_of() {
    // Trace: FR-021-AC-6, TC-149.
    let node = require_node!();
    let tmp = TempDir::new().unwrap();
    let probe = probe_report(
        &node,
        tmp.path(),
        &json!({
            "type": "object",
            "properties": {
                "mode": {"enum": ["a", "b"]},
                "rule": {
                    "type": "object",
                    "properties": {"x": {"type": "number"}},
                    "oneOf": [],
                },
            },
            "allOf": [
                {
                    "if": {
                        "properties": {"mode": {"const": "a"}},
                        "required": ["mode"],
                    },
                    "then": {
                        "required": ["rule"],
                        "properties": {"kind": {"not": {"const": "z"}}},
                    },
                }
            ],
        }),
    );
    assert!(contains(
        &probe["conditionalRequired"],
        &json!({"when": "mode = a", "required": ["rule"]})
    ));
    assert_eq!(
        probe["conditionalConstraints"],
        json!([{"when": "mode = a", "constraint": "kind must not be z"}])
    );
    assert_eq!(probe["exactlyOneOf"], json!([]));
    assert!(
        any_warning_mentions(&probe, "oneOf"),
        "{}",
        probe["warnings"]
    );
}

#[test]
fn onboard_js_json_checklist_lists_protected_apparatus_and_negative_controls() {
    // Trace: FR-024-AC-6, TC-159.
    let node = require_node!();
    let tmp = TempDir::new().unwrap();
    let apparatus_path =
        schema("measurement-plan-frontmatter.schema")["$defs"]["apparatus_path"].clone();
    let report = run_onboard_json(&node, tmp.path());
    let plan = &report["artifactChecklists"]["MeasurementPlan"];
    assert_eq!(
        plan["arrays"]["protected_apparatus"],
        json!({
            "minItems": 1,
            "uniqueItems": true,
            "itemPattern": apparatus_path["pattern"],
            "itemDescription": apparatus_path["description"],
        })
    );
    // A list with nothing to say is left out rather than printed as
    // "at least 0 item(s)".
    assert!(plan["arrays"].get("relationships").is_none());
    assert_eq!(
        plan["arrays"]["negative_controls"],
        json!({"minItems": 1, "uniqueItems": true})
    );
    assert_eq!(
        plan["enums"]["negative_controls.kind"],
        json!([
            "suppressed-observation",
            "gain-within-noise",
            "stale-evidence",
            "apparatus-edit",
            "selective-reporting",
        ])
    );
    let conditional = &plan["conditionalRequired"];
    assert!(contains(
        conditional,
        &json!({
            "when": "stage = gate and status is not retired",
            "required": ["ground_truth_kind", "negative_controls", "protected_apparatus"],
        })
    ));
    assert!(contains(
        conditional,
        &json!({
            "when": "negative_controls has an item where kind = apparatus-edit",
            "required": ["protected_apparatus"],
        })
    ));
    assert!(contains(
        conditional,
        &json!({
            "when": "negative_controls is present",
            "required": ["negative_controls.kind", "negative_controls.description"],
        })
    ));
    assert_eq!(plan["warnings"], json!([]));
}

#[test]
fn onboard_js_reports_no_warning_for_any_artifact_type() {
    // Trace: FR-024-AC-6, TC-159.
    let node = require_node!();
    let tmp = TempDir::new().unwrap();
    let report = run_onboard_json(&node, tmp.path());
    let checklists = report["artifactChecklists"].as_object().expect("object");
    assert!(checklists.contains_key("MeasurementPlan"));
    for (artifact_type, checklist) in checklists {
        assert!(checklist.get("unreadable").is_none(), "{artifact_type}");
        assert_eq!(
            checklist["warnings"],
            json!([]),
            "{artifact_type}: {}",
            checklist["warnings"]
        );
        for (path, shape) in checklist["arrays"].as_object().expect("arrays") {
            let min_items = shape["minItems"].as_u64().expect("minItems");
            assert!(
                min_items > 0
                    || shape["uniqueItems"].as_bool().expect("uniqueItems")
                    || shape.get("itemPattern").is_some(),
                "{artifact_type} {path}"
            );
        }
    }

    let target_repo = tmp.path().join("text-consumer");
    fs::create_dir(&target_repo).unwrap();
    let text = stdout(&run_ok(
        &node,
        &onboard_js(),
        &[Path::new("--repo"), &target_repo],
        &[],
    ));
    assert!(!text.contains("WARNING"));
    assert!(!text.contains("at least 0 item(s)"));
    let protected_line = text
        .lines()
        .find(|line| line.contains("protected_apparatus is a list"))
        .expect("protected_apparatus line");
    assert!(protected_line.contains("directory entry `<directory>/**`"));
    assert!(!protected_line.contains("\\x00") && !protected_line.contains("(?:"));
}

#[test]
fn onboard_js_lists_the_profile_measurement_policy() {
    // Trace: FR-025-AC-7, TC-165.
    let node = require_node!();
    let tmp = TempDir::new().unwrap();
    let report = run_onboard_json(&node, tmp.path());
    let profile = &report["artifactChecklists"]["AssuranceProfile"];
    assert_eq!(
        profile["enums"]["measurement_policy.mode"],
        json!(["recommend", "require"])
    );
    assert_eq!(
        profile["enums"]["measurement_policy.stages"],
        json!([
            "observe",
            "baseline",
            "branch-comparison",
            "trend",
            "ratchet",
            "target",
            "gate",
        ])
    );
    assert_eq!(
        profile["arrays"]["measurement_policy.stages"],
        json!({"minItems": 1, "uniqueItems": true})
    );
    assert!(contains(
        &profile["conditionalRequired"],
        &json!({
            "when": "measurement_policy is present",
            "required": ["measurement_policy.mode", "measurement_policy.stages"],
        })
    ));
    assert_eq!(profile["warnings"], json!([]));

    let module = package_root();
    let skeleton = read(&module.join("skeletons/AssuranceProfile.md"));
    assert!(skeleton.contains("measurement_policy:\n  mode: require\n  stages: [gate]\n"));
    assert!(skeleton.contains("## Measurement Policy"));
    let skill = read(&module.join("skills/assurance-onboarding/SKILL.md"));
    for phrase in [
        "measurement_policy",
        "measurement_verdict",
        "quoin.measurement-verdict.v1",
        "git-first-parent-add",
        "promotion_checker_missing",
        "promotion_checker_mismatch",
        "promotion_checker_not_accepted",
        "promotion_checker_order_unattested",
        "current `exception` item",
    ] {
        assert!(skill.contains(phrase), "{phrase}");
    }
}

fn help_prints_usage_not_a_report(flag: &str) {
    // Trace: FR-001-AC-8, TC-174.
    let node = require_node!();
    let tmp = TempDir::new().unwrap();
    let output = Command::new(&node)
        .arg(onboard_js())
        .arg(flag)
        .current_dir(tmp.path())
        .output()
        .expect("node runs");
    let out = stdout(&output);
    assert_eq!(output.status.code(), Some(0));
    assert!(out.starts_with("Usage: "));
    assert!(out.contains("--repo <path>"));
    assert!(!out.contains("onboarding report"));
}

#[test]
fn onboard_js_help_prints_usage_not_a_report_long_flag() {
    help_prints_usage_not_a_report("--help");
}

#[test]
fn onboard_js_help_prints_usage_not_a_report_short_flag() {
    help_prints_usage_not_a_report("-h");
}

fn refuses_a_usage_error(arguments: &[&str], message: &str) {
    // Trace: FR-001-AC-8, TC-174.
    let node = require_node!();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().to_str().expect("utf-8 tmp path");
    let output = Command::new(&node)
        .arg(onboard_js())
        .args(arguments.iter().map(|arg| arg.replace("{repo}", repo)))
        .current_dir(tmp.path())
        .output()
        .expect("node runs");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains(message), "{}", stderr(&output));
    assert!(stderr(&output).contains("Usage: "));
}

#[test]
fn onboard_js_refuses_a_usage_error_unknown_argument() {
    refuses_a_usage_error(
        &["--repo", "{repo}", "--bogus"],
        "unknown argument: --bogus",
    );
}

#[test]
fn onboard_js_refuses_a_usage_error_repo_without_path() {
    refuses_a_usage_error(&["--repo"], "--repo requires a path argument");
}

#[test]
fn onboard_js_refuses_a_usage_error_repo_followed_by_flag() {
    refuses_a_usage_error(&["--repo", "--json"], "--repo requires a path argument");
}

#[test]
fn onboard_js_refuses_a_usage_error_repo_given_twice() {
    refuses_a_usage_error(
        &["--repo", "{repo}", "--repo", "{repo}"],
        "--repo may be given only once",
    );
}

fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).expect("write script");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod");
}

fn run_summary(node: &Path, tmp: &Path, path_env: &str, extra: &[&str]) -> Output {
    let target_repo = tmp.join("consumer");
    let assurance = target_repo.join("spec/assurance");
    fs::create_dir_all(&assurance).expect("mkdir assurance");
    fs::write(
        assurance.join("MP-001.md"),
        "---\ntype: MeasurementPlan\n---\n",
    )
    .unwrap();
    Command::new(node)
        .arg(onboard_js())
        .arg("--repo")
        .arg(&target_repo)
        .arg("--summary")
        .args(extra)
        .env_clear()
        .env("PATH", path_env)
        .env("HOME", tmp)
        .output()
        .expect("node runs")
}

#[test]
fn onboard_js_summary_reports_validation_and_toolchain() {
    let node = require_node!();
    let tmp = TempDir::new().unwrap();
    let tmp_path = tmp.path();
    // Fake quire/quoin first on PATH make the validation and toolchain answers
    // deterministic: quire fails the one artifact named MP-002.md.
    let bin_dir = tmp_path.join("bin");
    fs::create_dir(&bin_dir).unwrap();
    write_executable(
        &bin_dir.join("quire"),
        r#"#!/bin/sh
if [ "$1" = provenance ]; then echo '{"cli":{"version":"9.9.9"}}'; exit 0; fi
case "$*" in *MP-002.md)
  echo "x/MP-002.md: [MeasurementPlan] frontmatter: \"owner\" is a required property" >&2
  exit 1;;
esac
exit 0
"#,
    );
    write_executable(
        &bin_dir.join("quoin"),
        r#"#!/bin/sh
if [ "$1" = --version ]; then echo "quoin 8.8.8"; exit 0; fi
echo "  measurement record  Record."
"#,
    );

    let target_repo = tmp_path.join("consumer");
    let assurance = target_repo.join("spec/assurance");
    fs::create_dir_all(&assurance).unwrap();
    for name in ["MP-001.md", "MP-002.md"] {
        fs::write(assurance.join(name), "---\ntype: MeasurementPlan\n---\n").unwrap();
    }

    let output = Command::new(&node)
        .arg(onboard_js())
        .arg("--repo")
        .arg(&target_repo)
        .args(["--summary", "--json"])
        .env_clear()
        .env("PATH", format!("{}:/usr/bin:/bin", bin_dir.display()))
        .env("HOME", tmp_path)
        .output()
        .expect("node runs");
    assert_eq!(
        output.status.code(),
        Some(1),
        "an invalid artifact must fail the summary"
    );
    let summary: Value = serde_json::from_str(&stdout(&output)).expect("json summary");

    assert_eq!(
        summary["toolchain"],
        json!({"quire": "9.9.9", "quoin": "8.8.8", "quoinMeasurementVerify": false})
    );
    let version = &summary["installedModuleVersion"];
    assert!(
        !version.is_null() && version != "" && version != false,
        "installedModuleVersion is empty"
    );
    let by_name: std::collections::HashMap<&str, &Value> = summary["artifacts"]
        .as_array()
        .expect("artifacts")
        .iter()
        .map(|item| (item["name"].as_str().expect("name"), item))
        .collect();
    assert_eq!(
        by_name["MP-001.md"]["validation"],
        json!({"status": "valid", "findings": [], "moreFindings": 0})
    );
    assert_eq!(by_name["MP-002.md"]["validation"]["status"], "invalid");
    assert_eq!(
        by_name["MP-002.md"]["validation"]["findings"],
        json!([r#"[MeasurementPlan] frontmatter: "owner" is a required property"#])
    );
    // The compact form must not carry the full report's orientation prose.
    assert!(summary.get("relationship").is_none());
    assert!(summary.get("nextSteps").is_none());
}

#[test]
fn onboard_js_summary_exits_nonzero_when_validation_is_unavailable() {
    let node = require_node!();
    let tmp = TempDir::new().unwrap();
    // No quire on PATH: nothing was validated, so the summary must not pass.
    // Only node itself is reachable, so a real quire beside it cannot be found.
    let only_node = tmp.path().join("only-node");
    fs::create_dir(&only_node).unwrap();
    symlink(&node, only_node.join("node")).unwrap();
    let output = run_summary(&node, tmp.path(), only_node.to_str().unwrap(), &["--json"]);
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    let summary: Value = serde_json::from_str(&stdout(&output)).expect("json summary");
    assert_eq!(
        summary["artifacts"][0]["validation"]["status"],
        "unavailable"
    );
    assert!(stderr(&output).contains("validation unavailable"));
}

#[test]
fn onboard_js_summary_marks_truncated_findings_and_labels_the_module() {
    let node = require_node!();
    let tmp = TempDir::new().unwrap();
    let bin_dir = tmp.path().join("bin");
    fs::create_dir(&bin_dir).unwrap();
    write_executable(
        &bin_dir.join("quire"),
        r#"#!/bin/sh
if [ "$1" = provenance ]; then echo '{"cli":{"version":"9.9.9"}}'; exit 0; fi
i=1
while [ $i -le 12 ]; do
  echo "x/MP-001.md: [MeasurementPlan] finding $i" >&2
  i=$((i+1))
done
exit 1
"#,
    );
    symlink(&node, bin_dir.join("node")).unwrap();
    let output = run_summary(&node, tmp.path(), bin_dir.to_str().unwrap(), &[]);
    let out = stdout(&output);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(out.contains("      +2 more"));
    assert!(out.contains("finding 10"));
    assert!(!out.contains("finding 11"));
    assert!(out.contains("module in this checkout:"));
}

#[test]
fn onboard_js_observation_checklist_notes_the_optional_interval() {
    // Trace: FR-021-AC-6, TC-149.
    let node = require_node!();
    let tmp = TempDir::new().unwrap();
    let report = run_onboard_json(&node, tmp.path());
    let notes: Vec<&str> = report["measurementRecordChecklist"]["observation"]
        .as_array()
        .expect("observation checklist")
        .iter()
        .filter_map(Value::as_str)
        .filter(|item| item.starts_with("interval"))
        .collect();
    assert_eq!(notes.len(), 1, "{notes:?}");
    let interval_note = notes[0];
    assert!(interval_note.contains("OPTIONAL"));
    assert!(interval_note.contains("quoin applies it"));
    assert!(interval_note.contains("tracked in EA-26"));
    assert!(!interval_note.contains("TBD"));
    assert!(interval_note.contains("decision_rule.interval_level"));
}

#[test]
fn onboard_js_reads_dependent_required_and_warns_on_a_malformed_entry() {
    // Trace: FR-021-AC-12, TC-188.
    let node = require_node!();
    let tmp = TempDir::new().unwrap();
    let probe = probe_report(
        &node,
        tmp.path(),
        &json!({
            "type": "object",
            "properties": {"a": {"type": "string"}, "b": {"type": "string"}},
            "dependentRequired": {"a": ["b"], "b": "a"},
        }),
    );
    assert!(contains(
        &probe["conditionalRequired"],
        &json!({"when": "a is present", "required": ["b"]})
    ));
    assert!(any_warning_mentions(&probe, "dependentRequired"), "{probe}");
}
