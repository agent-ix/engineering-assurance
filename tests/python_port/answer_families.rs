//! `statistical_design.answer_families`: the plan declares the answer families
//! a constant-predictor baseline is computed over (FR-021-AC-16, AC-18).
use super::common::{package_root, read, schema, schema_errors};
use ix_trace_rs::trace;
use serde_json::{Value, json};
use std::process::Command;

fn plan(rule: &Value, answer_families: Option<Value>) -> Value {
    let mut design = json!({
        "population": "p",
        "sampling": "s",
        "repetitions": 5,
        "estimator": "proportion",
        "error_model": "e",
        "uncertainty": "u",
        "decision_rule": rule,
    });
    if let Some(families) = answer_families {
        design["answer_families"] = families;
    }
    json!({
        "id": "MP-900",
        "title": "t",
        "type": "MeasurementPlan",
        "status": "proposed",
        "owner": "o",
        "stage": "baseline",
        "metric": "agreement_rate",
        "statistical_design": design,
        "relationships": [],
    })
}

fn errors(rule: &Value, answer_families: Option<Value>) -> Vec<String> {
    let validator = jsonschema::draft202012::new(&schema("measurement-plan-frontmatter.schema"))
        .expect("schema compiles");
    schema_errors(&validator, &plan(rule, answer_families))
}

fn constant() -> Value {
    json!({"comparator": "gt", "baseline": "constant-predictor", "margin": 0.05})
}

#[test]
#[trace("TC-240", "FR-021-AC-16")]
fn answer_families_are_required_with_a_constant_predictor_and_a_valid_list() {
    assert!(errors(&constant(), Some(json!(["weakness_kind", "coverage"]))).is_empty());
    assert!(errors(&constant(), Some(json!(["only"]))).is_empty());
    assert!(!errors(&constant(), None).is_empty(), "absent list");
    for refused in [
        json!([]),
        json!(["a", "a"]),
        json!(["a", ""]),
        json!(["a", 7]),
        json!("a"),
    ] {
        assert!(
            !errors(&constant(), Some(refused.clone())).is_empty(),
            "{refused}"
        );
    }
}

#[test]
#[trace("TC-240", "FR-021-AC-16")]
fn answer_families_are_refused_with_any_other_baseline_or_a_threshold_rule() {
    let families = Some(json!(["a", "b"]));
    for baseline in ["prior-collection", "best-seen", "external-reference"] {
        let rule = json!({"comparator": "ge", "baseline": baseline});
        assert!(errors(&rule, None).is_empty(), "{baseline} without");
        assert!(
            !errors(&rule, families.clone()).is_empty(),
            "{baseline} with"
        );
    }
    let threshold = json!({"comparator": "ge", "threshold": 0.9});
    assert!(errors(&threshold, None).is_empty());
    assert!(!errors(&threshold, families).is_empty());
}

#[test]
#[trace("TC-242", "FR-021-AC-18")]
fn skeleton_and_onboarding_state_the_answer_families_rule() {
    let skeleton = read(&package_root().join("skeletons/MeasurementPlan.md"));
    assert!(skeleton.contains("answer_families: [weakness_kind, coverage]"));
    assert!(skeleton.contains("do not take it from the Population prose"));
    assert!(
        skeleton.contains("must not"),
        "refused with another reference"
    );

    let output = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .arg("onboard-report")
        .arg("--module-root")
        .arg(package_root())
        .arg("--repo")
        .arg(super::common::root())
        .arg("--json")
        .output()
        .expect("native report runs");
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("report json");
    let checklist = &report["artifactChecklists"]["MeasurementPlan"];
    let text = checklist.to_string();
    assert!(
        text.contains("statistical_design.answer_families"),
        "{text}"
    );
    assert!(
        checklist["conditionalRequired"]
            .as_array()
            .expect("conditionalRequired array")
            .contains(&json!({
                "when": "statistical_design.decision_rule.baseline = constant-predictor",
                "required": ["statistical_design.answer_families"],
            })),
        "{}",
        checklist["conditionalRequired"]
    );
    assert!(
        checklist["conditionalConstraints"]
            .as_array()
            .expect("conditionalConstraints array")
            .iter()
            .any(|entry| entry["constraint"]
                .as_str()
                .is_some_and(|constraint| constraint
                    .contains("statistical_design.answer_families must be absent"))),
        "{}",
        checklist["conditionalConstraints"]
    );

    let skill = read(&package_root().join("skills/assurance-onboarding/SKILL.md"));
    assert!(skill.contains("required with `constant-predictor`"));
    assert!(skill.contains("refused with any other reference"));
    assert!(skill.contains("must equal it exactly"));
}
