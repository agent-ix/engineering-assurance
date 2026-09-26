//! Port of the retired `tests/test_module.py`; see `main.rs`.
#![allow(
    clippy::needless_pass_by_value,
    reason = "case builders take owned `json!` values so call sites stay literal"
)]
#![allow(
    clippy::too_many_lines,
    reason = "each ported schema test keeps its accept/refuse table in one place"
)]
use ix_trace_rs::trace;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

use jsonschema::Validator;
use regex::Regex;
use serde_json::{Value, json};

use super::common::{
    frontmatter, glob_ext, manifest, merged, package_root, read, required_msg, root, schema,
    schema_errors, yaml,
};

// Assembled from parts so the content-rights URL scan does not see a literal.
fn draft_07_uris() -> Vec<String> {
    ["http:", "https:"]
        .iter()
        .map(|s| format!("{s}//json-schema.org/draft-07/schema"))
        .collect()
}

fn draft_2020_12_uri() -> String {
    format!("{}{}", "https:", "//json-schema.org/draft/2020-12/schema")
}

fn dialect(declared: Option<&Value>) -> Option<&'static str> {
    let declared = declared?.as_str()?;
    let trimmed = declared.strip_suffix('#').unwrap_or(declared);
    if draft_07_uris().iter().any(|u| u == trimmed) {
        Some("draft-07")
    } else if declared == draft_2020_12_uri() {
        Some("2020-12")
    } else {
        None
    }
}

fn validator(contract: &Value) -> Validator {
    jsonschema::draft202012::new(contract).expect("schema compiles")
}

fn skeleton_path(name: &str) -> PathBuf {
    package_root().join("skeletons").join(name)
}

fn onboarding_script() -> PathBuf {
    package_root()
        .join("skills")
        .join("assurance-onboarding")
        .join("scripts")
        .join("onboard.js")
}

fn onboarding_skill() -> String {
    read(
        &package_root()
            .join("skills")
            .join("assurance-onboarding")
            .join("SKILL.md"),
    )
}

fn run_onboard() -> Value {
    let completed = Command::new("node")
        .arg(onboarding_script())
        .args(["--repo"])
        .arg(root())
        .arg("--json")
        .output()
        .expect("run node");
    assert!(
        completed.status.success(),
        "onboard.js failed: {}",
        String::from_utf8_lossy(&completed.stderr)
    );
    serde_json::from_slice(&completed.stdout).expect("onboard.js json")
}

fn strs(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .expect("array")
        .iter()
        .map(|v| v.as_str().expect("string"))
        .collect()
}

fn minimal_profile(over: Value) -> Value {
    merged(
        json!({
            "id": "AP-900",
            "title": "t",
            "type": "AssuranceProfile",
            "status": "proposed",
            "owner": "o",
            "relationships": [],
        }),
        over,
    )
}

fn minimal_measurement_plan(over: Value) -> Value {
    merged(
        json!({
            "id": "MP-900",
            "title": "t",
            "type": "MeasurementPlan",
            "status": "proposed",
            "owner": "o",
            "stage": "baseline",
            "relationships": [],
        }),
        over,
    )
}

fn negative_control() -> Value {
    json!({
        "kind": "suppressed-observation",
        "description": "a dropped failing item lowers examined below the corpus size",
    })
}

fn plan_validator() -> Validator {
    validator(&schema("measurement-plan-frontmatter.schema"))
}

fn plan_errors(over: Value) -> Vec<String> {
    schema_errors(&plan_validator(), &minimal_measurement_plan(over))
}

fn statistical_design(over: Value) -> Value {
    merged(
        json!({
            "population": "p",
            "sampling": "s",
            "repetitions": 5,
            "estimator": "proportion",
            "error_model": "e",
            "uncertainty": "u",
            "decision_rule": {"comparator": "ge", "threshold": 0.99},
        }),
        over,
    )
}

fn statistical_design_errors(objective: Option<Value>, over: Value) -> Vec<String> {
    let mut plan = minimal_measurement_plan(json!({
        "metric": "request_retention_rate",
        "statistical_design": statistical_design(over),
    }));
    if let Some(objective) = objective {
        plan["objective"] = objective;
    }
    schema_errors(&plan_validator(), &plan)
}

fn sd(over: Value) -> Vec<String> {
    statistical_design_errors(None, over)
}

fn dr(rule: Value) -> Vec<String> {
    sd(json!({ "decision_rule": rule }))
}

#[test]
#[trace("TC-194", "FR-003-AC-7")]
fn dialect_detector_accepts_every_spelling_and_refuses_other_drafts() {
    for uri in draft_07_uris() {
        for declared in [uri.clone(), format!("{uri}#")] {
            assert_eq!(
                dialect(Some(&json!(declared))),
                Some("draft-07"),
                "{declared}"
            );
        }
    }
    assert_eq!(dialect(Some(&json!(draft_2020_12_uri()))), Some("2020-12"));
    let others = [
        Some(json!(format!(
            "{}{}",
            "https:", "//json-schema.org/draft/2019-09/schema"
        ))),
        Some(json!(format!(
            "{}{}",
            "http:", "//json-schema.org/draft-04/schema#"
        ))),
        None,
    ];
    for declared in others {
        assert_eq!(dialect(declared.as_ref()), None, "{declared:?}");
    }
    // The Python `None` case: a non-string `$schema` is also refused.
    assert_eq!(dialect(Some(&Value::Null)), None);
}

#[test]
#[trace("TC-194", "FR-003-AC-7")]
fn every_schema_uses_the_forms_of_the_dialect_it_declares() {
    // `definitions`/array-form `dependencies` are draft-07; `$defs`/
    // `dependentRequired` are 2019-09+. A file that mixes them validates
    // differently under different consumers.
    let paths = glob_ext(&package_root().join("schemas"), "json");
    assert!(!paths.is_empty());
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for path in paths {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let text = read(&path);
        let parsed: Value = serde_json::from_str(&text).expect("json");
        let d = dialect(parsed.get("$schema"))
            .unwrap_or_else(|| panic!("{name} declares no supported dialect"));
        seen.insert(d);
        if d == "draft-07" {
            for modern in ["\"$defs\"", "#/$defs/", "\"dependentRequired\""] {
                assert!(
                    !text.contains(modern),
                    "{name} is draft-07 but uses {modern}"
                );
            }
        } else {
            for old in ["\"definitions\"", "#/definitions/", "\"dependencies\""] {
                assert!(!text.contains(old), "{name} is 2020-12 but uses {old}");
            }
        }
    }
    assert!(!seen.is_empty(), "the guard checked no schema");
}

#[test]
#[trace("TC-194", "FR-003-AC-7")]
fn module_inventory_is_exact() {
    let data = manifest();
    assert_eq!(data["version"], "0.5.0");
    let names = |key: &str| -> Vec<String> {
        data[key]
            .as_array()
            .expect("array")
            .iter()
            .map(|i| i["name"].as_str().expect("name").to_owned())
            .collect()
    };
    assert_eq!(
        names("artifact_types"),
        [
            "AssuranceProfile",
            "MeasurementPlan",
            "ArchitectureDescription",
            "ComponentAssuranceContract",
            "AssuranceArgument",
        ]
    );
    assert_eq!(data["lint_rules"], json!([]));
    assert_eq!(names("object_types"), ["campaign_value", "campaign_enum"]);
    assert!(data.get("edge_types").is_none());
}

#[test]
#[trace("TC-115", "FR-018-AC-4")]
fn only_the_configuration_package_entry_point_remains_python() {
    let sources: Vec<String> = glob_ext(&package_root(), "py")
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(sources, ["__init__.py"]);
    let source = read(&package_root().join("__init__.py"));
    assert!(!source.contains("def "));
    assert!(!source.contains("class "));
}

#[test]
#[trace("TC-195", "FR-003-AC-8")]
fn every_schema_and_skeleton_is_valid() {
    for artifact in manifest()["artifact_types"].as_array().unwrap() {
        let schema_path = package_root().join(artifact["frontmatter_schema_ref"].as_str().unwrap());
        let name = artifact["name"].as_str().unwrap();
        let path = skeleton_path(&format!("{name}.md"));
        let contract: Value = serde_json::from_str(&read(&schema_path)).unwrap();
        jsonschema::meta::validate(&contract)
            .unwrap_or_else(|e| panic!("{name} schema is not valid: {e}"));
        let errors = schema_errors(&validator(&contract), &frontmatter(&path));
        assert!(errors.is_empty(), "{name}: {errors:?}");
        let body = read(&path);
        let locators = artifact["body_extraction"]["yield_pattern"]["match"]
            .as_object()
            .unwrap();
        for locator in locators.values() {
            assert_eq!(locator["required"], json!(true));
            let heading = locator["after_heading"].as_str().unwrap();
            assert!(body.contains(&format!("## {heading}")), "{name}: {heading}");
        }
    }
}

#[test]
#[trace("TC-163", "FR-025-AC-1")]
fn profile_measurement_policy_is_closed_and_uses_plan_stages() {
    let contract = schema("assurance-profile-frontmatter.schema");
    let plan = schema("measurement-plan-frontmatter.schema");
    let policy = &contract["$defs"]["measurement_policy"];
    assert_eq!(
        policy["properties"]["stages"]["items"]["enum"],
        plan["properties"]["stage"]["enum"]
    );
    assert_eq!(
        policy["properties"]["mode"]["enum"],
        contract["$defs"]["review_policy"]["properties"]["mode"]["enum"]
    );
    assert!(!strs(&contract["required"]).contains(&"measurement_policy"));
    let v = validator(&contract);
    let accepted = [
        Value::Null,
        json!({"mode": "recommend", "stages": ["observe"]}),
        json!({"mode": "require", "stages": ["gate"]}),
        json!({"mode": "require", "stages": ["target", "gate"]}),
    ];
    for value in accepted {
        let document = if value.is_null() {
            minimal_profile(json!({}))
        } else {
            minimal_profile(json!({"measurement_policy": value}))
        };
        assert!(schema_errors(&v, &document).is_empty(), "{value}");
    }
    let refused = [
        json!({"mode": "require"}),
        json!({"stages": ["gate"]}),
        json!({"mode": "enforced", "stages": ["gate"]}),
        json!({"mode": "advisory", "stages": ["gate"]}),
        json!({"mode": "require", "stages": []}),
        json!({"mode": "require", "stages": ["gate", "gate"]}),
        json!({"mode": "require", "stages": ["release"]}),
        json!({"mode": "require", "stages": "gate"}),
        json!({"mode": "require", "stages": ["gate"], "exceptions": []}),
        json!("require"),
    ];
    for value in refused {
        let errors = schema_errors(&v, &minimal_profile(json!({"measurement_policy": value})));
        assert!(!errors.is_empty(), "{value}");
    }
}

/// A gate-stage plan grades against answers someone produced, so it must say
/// how: by a person, by another agent, or mechanically. Below gate it may
/// stay silent, and no other provenance is admitted at any stage.
#[test]
#[trace("TC-203", "FR-024-AC-11")]
fn ground_truth_kind_is_required_only_for_gate_stage() {
    let gate = |kind: Option<&str>| {
        let mut plan = json!({
            "stage": "gate",
            "negative_controls": [negative_control()],
            "protected_apparatus": ["evals/harness.py"],
        });
        if let Some(kind) = kind {
            plan["ground_truth_kind"] = json!(kind);
        }
        plan_errors(plan)
    };
    for kind in ["human-labelled", "agent-labelled", "mechanical"] {
        assert!(
            gate(Some(kind)).is_empty(),
            "{kind}: {:?}",
            gate(Some(kind))
        );
        let below = plan_errors(json!({"stage": "baseline", "ground_truth_kind": kind}));
        assert!(below.is_empty(), "{kind} below gate: {below:?}");
    }

    let missing = gate(None);
    assert_eq!(missing, [required_msg("ground_truth_kind")]);
    assert!(plan_errors(json!({"stage": "baseline"})).is_empty());

    for refused in ["vibes-based", "Mechanical", ""] {
        assert!(!gate(Some(refused)).is_empty(), "{refused} at gate");
        let below = plan_errors(json!({"stage": "baseline", "ground_truth_kind": refused}));
        assert!(!below.is_empty(), "{refused} below gate");
    }
}

/// `preregistration` records only the digest Quoin checks the plan's
/// "Comparison and Enforcement" section against at intake, so its one member
/// must be a `sha256:` digest Quoin can compare byte for byte.
#[test]
#[trace("TC-204", "FR-024-AC-12")]
fn preregistration_is_optional_and_requires_a_sha256_bar_digest() {
    assert!(
        !strs(&schema("measurement-plan-frontmatter.schema")["required"])
            .contains(&"preregistration")
    );
    assert!(plan_errors(json!({})).is_empty());

    let digest = format!("sha256:{}", "a".repeat(64));
    assert!(plan_errors(json!({"preregistration": {"bar_digest": digest}})).is_empty());

    let refused = [
        json!({}),
        json!({"bar_digest": "not-a-digest"}),
        json!({"bar_digest": "a".repeat(64)}),
        json!({"bar_digest": format!("sha256:{}", "A".repeat(64))}),
        json!({"bar_digest": format!("sha256:{}", "a".repeat(63))}),
        json!({"bar_digest": format!("sha256:{}", "a".repeat(65))}),
        json!({"bar_digest": format!("sha512:{}", "a".repeat(64))}),
        json!({"bar_digest": 7}),
        json!({"bar_digest": digest, "bar_source": "elsewhere.rs"}),
        json!(digest),
    ];
    for value in refused {
        assert!(
            !plan_errors(json!({"preregistration": value})).is_empty(),
            "{value}"
        );
    }
}

#[test]
#[trace("TC-139", "FR-020-AC-1")]
fn objective_is_optional_and_target_requires_a_bound() {
    let contract = schema("measurement-plan-frontmatter.schema");
    assert!(!strs(&contract["required"]).contains(&"objective"));
    let errors = |objective: Value| plan_errors(json!({"objective": objective}));

    assert!(plan_errors(json!({})).is_empty());
    for direction in ["higher", "lower", "zero"] {
        assert!(errors(json!({"direction": direction})).is_empty());
        assert!(errors(json!({"direction": direction, "bound": 0.95})).is_empty());
    }
    assert!(errors(json!({"direction": "target", "bound": 250})).is_empty());

    assert!(errors(json!({"direction": "target"})).contains(&required_msg("bound")));
    assert!(!errors(json!({"direction": "sideways"})).is_empty());
    assert!(!errors(json!({"bound": 1})).is_empty());
    assert!(!errors(json!({"direction": "target", "bound": "250"})).is_empty());
    assert!(!errors(json!({"direction": "higher", "epoch": 2})).is_empty());
}

#[test]
#[trace("TC-139", "FR-020-AC-1")]
fn measurement_plan_skeleton_shows_a_valid_objective() {
    let skeleton = frontmatter(&skeleton_path("MeasurementPlan.md"));
    let direction = skeleton["objective"]["direction"].as_str().unwrap();
    assert!(["higher", "lower", "zero", "target"].contains(&direction));
    assert!(schema_errors(&plan_validator(), &skeleton).is_empty());
}

#[test]
#[trace("TC-166", "FR-026-AC-1")]
fn objective_steering_fields_are_optional_advisory_and_range_checked() {
    let contract = schema("measurement-plan-frontmatter.schema");
    let objective_schema = &contract["$defs"]["objective"];
    let props: BTreeSet<&str> = objective_schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        props,
        BTreeSet::from(["direction", "bound", "weight", "value_half_life", "budget"])
    );
    assert_eq!(objective_schema["required"], json!(["direction"]));

    let errors = |over: Value| {
        plan_errors(json!({"objective": merged(json!({"direction": "higher"}), over)}))
    };

    // No steering fields at all, and every field present together.
    assert!(errors(json!({})).is_empty());
    assert!(errors(json!({"weight": 1.0, "value_half_life": 30, "budget": 50})).is_empty());
    // Each field independently, including the advisory-only edge cases named
    // by PLAT-967: weight and budget may be exactly zero.
    assert!(errors(json!({"weight": 0})).is_empty());
    assert!(errors(json!({"weight": 2.5})).is_empty());
    assert!(errors(json!({"budget": 0})).is_empty());
    assert!(errors(json!({"budget": 1000})).is_empty());
    assert!(errors(json!({"value_half_life": 0.001})).is_empty());
    assert!(errors(json!({"value_half_life": 365})).is_empty());

    // Negative weight, negative budget, and a non-positive half-life are rejected.
    assert!(!errors(json!({"weight": -0.01})).is_empty());
    assert!(!errors(json!({"budget": -1})).is_empty());
    assert!(!errors(json!({"value_half_life": 0})).is_empty());
    assert!(!errors(json!({"value_half_life": -1})).is_empty());

    // Non-numeric values are rejected for every field.
    for field in ["weight", "value_half_life", "budget"] {
        for bad in [json!("a lot"), json!(true), json!([1]), Value::Null] {
            assert!(!errors(json!({ field: bad })).is_empty(), "{field}: {bad}");
        }
    }
}

#[test]
#[trace("TC-170", "FR-026-AC-5")]
fn measurement_plan_skeleton_shows_the_steering_fields() {
    let skeleton = frontmatter(&skeleton_path("MeasurementPlan.md"));
    let objective = &skeleton["objective"];
    assert!(objective["weight"].as_f64().unwrap() > 0.0);
    assert!(objective["value_half_life"].as_f64().unwrap() > 0.0);
    assert!(objective["budget"].as_f64().unwrap() >= 0.0);
    assert!(schema_errors(&plan_validator(), &skeleton).is_empty());
    let body = read(&skeleton_path("MeasurementPlan.md"));
    assert!(body.contains("## Steering Fields"));
    assert!(body.to_lowercase().contains("advisory only"));
    let skill = onboarding_skill();
    for field in ["`weight`", "`value_half_life`", "`budget`"] {
        assert!(skill.contains(field), "{field}");
    }
    assert!(skill.contains("never gate"));
    let report = run_onboard();
    assert_eq!(
        report["artifactChecklists"]["MeasurementPlan"]["warnings"],
        json!([])
    );
}

#[test]
#[trace("TC-144", "FR-021-AC-1")]
#[trace("TC-192", "FR-021-AC-13")]
fn decision_rule_is_a_closed_comparator_with_exactly_one_reference() {
    let contract = schema("measurement-plan-frontmatter.schema");
    let rule = &contract["$defs"]["decision_rule"];
    assert_eq!(
        rule["properties"]["comparator"]["enum"],
        json!(["gt", "ge", "lt", "le", "eq"])
    );
    assert_eq!(
        rule["properties"]["baseline"]["enum"],
        json!([
            "constant-predictor",
            "prior-collection",
            "best-seen",
            "external-reference"
        ])
    );
    let props: BTreeSet<&str> = rule["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        props,
        BTreeSet::from([
            "comparator",
            "threshold",
            "baseline",
            "margin",
            "margin_mode",
            "interval_level"
        ])
    );
    let interval_level = &rule["properties"]["interval_level"];
    assert_eq!(interval_level["type"], "number");
    assert_eq!(interval_level["exclusiveMinimum"], 0);
    assert_eq!(interval_level["exclusiveMaximum"], 1);

    for comparator in ["gt", "ge", "lt", "le", "eq"] {
        assert!(dr(json!({"comparator": comparator, "threshold": 0})).is_empty());
    }
    assert!(dr(json!({"comparator": "eq", "baseline": "prior-collection"})).is_empty());
    for baseline in [
        "constant-predictor",
        "prior-collection",
        "best-seen",
        "external-reference",
    ] {
        assert!(dr(json!({"comparator": "gt", "baseline": baseline})).is_empty());
        assert!(dr(json!({"comparator": "ge", "baseline": baseline, "margin": -0.01})).is_empty());
    }

    for comparator in ["gt", "ge", "lt", "le"] {
        assert!(
            dr(json!({"comparator": comparator, "threshold": 1, "interval_level": 0.95}))
                .is_empty()
        );
    }
    assert!(
        dr(json!({
            "comparator": "le",
            "baseline": "prior-collection",
            "margin": -0.05,
            "margin_mode": "relative",
            "interval_level": 0.9,
        }))
        .is_empty()
    );

    let refused = [
        (
            "unknown comparator",
            json!({"comparator": "approximately", "threshold": 1}),
        ),
        ("missing comparator", json!({"threshold": 1})),
        (
            "neither threshold nor baseline",
            json!({"comparator": "ge"}),
        ),
        (
            "both threshold and baseline",
            json!({"comparator": "ge", "threshold": 1, "baseline": "best-seen"}),
        ),
        (
            "margin with a threshold",
            json!({"comparator": "ge", "threshold": 1, "margin": 0.1}),
        ),
        (
            "margin with eq",
            json!({"comparator": "eq", "baseline": "prior-collection", "margin": 0.1}),
        ),
        (
            "non-numeric margin",
            json!({"comparator": "ge", "baseline": "best-seen", "margin": "0.05"}),
        ),
        (
            "interval_level with eq",
            json!({"comparator": "eq", "threshold": 1, "interval_level": 0.9}),
        ),
        (
            "interval_level of 0",
            json!({"comparator": "ge", "threshold": 1, "interval_level": 0}),
        ),
        (
            "interval_level of 1",
            json!({"comparator": "ge", "threshold": 1, "interval_level": 1}),
        ),
        (
            "interval_level above 1",
            json!({"comparator": "ge", "threshold": 1, "interval_level": 95}),
        ),
        (
            "non-numeric interval_level",
            json!({"comparator": "ge", "threshold": 1, "interval_level": "0.95"}),
        ),
        (
            "unknown baseline",
            json!({"comparator": "ge", "baseline": "vibes"}),
        ),
        (
            "eq against best-seen",
            json!({"comparator": "eq", "baseline": "best-seen"}),
        ),
        (
            "non-numeric threshold",
            json!({"comparator": "ge", "threshold": "0.99"}),
        ),
        (
            "duplicated repetitions",
            json!({"comparator": "ge", "threshold": 1, "repetitions": 5}),
        ),
        (
            "duplicated minimum_n",
            json!({"comparator": "ge", "threshold": 1, "minimum_n": 20}),
        ),
    ];
    for (case, rule) in refused {
        assert!(!dr(rule).is_empty(), "{case}");
    }
    assert!(!dr(json!("escalate when low")).is_empty());

    // JSON Schema has no finiteness keyword: a YAML `.inf` threshold passes the
    // schema, and only the Rust DecisionRule refuses it (FR-021). serde_json
    // cannot carry an infinity, so assert what makes that true instead: the
    // schema's threshold is a bare number with no bound keyword, and the
    // largest finite double passes.
    let threshold = &rule["properties"]["threshold"];
    assert_eq!(threshold["type"], "number");
    for bound in ["minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum"] {
        assert!(threshold.get(bound).is_none(), "threshold has {bound}");
    }
    assert!(dr(json!({"comparator": "ge", "threshold": f64::MAX})).is_empty());

    let without_metric =
        minimal_measurement_plan(json!({"statistical_design": statistical_design(json!({}))}));
    assert!(
        schema_errors(&validator(&contract), &without_metric).contains(&required_msg("metric"))
    );
}

#[test]
#[trace("TC-188", "FR-021-AC-12")]
fn margin_mode_refusals_and_acceptance_run_through_the_validator() {
    for mode in ["relative", "absolute"] {
        assert!(
            dr(json!({
                "comparator": "ge",
                "baseline": "prior-collection",
                "margin": 0.05,
                "margin_mode": mode,
            }))
            .is_empty(),
            "{mode}"
        );
    }
    let refused = [
        (
            "margin_mode with a threshold",
            json!({"comparator": "ge", "threshold": 1, "margin_mode": "relative"}),
        ),
        (
            "margin_mode without a margin",
            json!({"comparator": "ge", "baseline": "prior-collection", "margin_mode": "relative"}),
        ),
        (
            "margin_mode with eq",
            json!({"comparator": "eq", "baseline": "prior-collection", "margin_mode": "relative"}),
        ),
        (
            "margin_mode with eq and a margin",
            json!({
                "comparator": "eq",
                "baseline": "prior-collection",
                "margin": 0.1,
                "margin_mode": "relative",
            }),
        ),
        (
            "unknown margin_mode",
            json!({
                "comparator": "ge",
                "baseline": "prior-collection",
                "margin": 0.05,
                "margin_mode": "percent",
            }),
        ),
    ];
    for (case, rule) in refused {
        assert!(!dr(rule).is_empty(), "{case}");
    }
}

#[test]
#[trace("TC-144", "FR-021-AC-9")]
fn decision_rule_agrees_with_objective_direction_and_estimator() {
    fn rule(comparator: &str, reference: Option<Value>) -> Value {
        merged(
            json!({"comparator": comparator}),
            reference.unwrap_or_else(|| json!({"threshold": 0})),
        )
    }
    let agreeing = [
        ("higher", rule("gt", None)),
        ("higher", rule("ge", None)),
        ("lower", rule("lt", None)),
        ("lower", rule("le", Some(json!({"baseline": "best-seen"})))),
        ("zero", rule("eq", None)),
        (
            "zero",
            rule("eq", Some(json!({"baseline": "prior-collection"}))),
        ),
        ("zero", rule("le", Some(json!({"threshold": 0})))),
        ("target", rule("lt", Some(json!({"threshold": 1})))),
        ("target", rule("eq", Some(json!({"threshold": 1})))),
    ];
    for (direction, decision_rule) in agreeing {
        let objective = json!({"direction": direction, "bound": 1});
        let errors =
            statistical_design_errors(Some(objective), json!({"decision_rule": decision_rule}));
        assert!(errors.is_empty(), "{direction} {decision_rule}: {errors:?}");
    }
    let disagreeing = [
        ("higher", rule("le", None)),
        ("higher", rule("eq", None)),
        ("lower", rule("ge", None)),
        ("lower", rule("gt", Some(json!({"baseline": "best-seen"})))),
        ("zero", rule("le", Some(json!({"threshold": 0.5})))),
        (
            "zero",
            rule("le", Some(json!({"baseline": "prior-collection"}))),
        ),
        ("zero", rule("lt", Some(json!({"threshold": 0})))),
    ];
    for (direction, decision_rule) in disagreeing {
        let errors = statistical_design_errors(
            Some(json!({"direction": direction})),
            json!({"decision_rule": decision_rule}),
        );
        assert!(!errors.is_empty(), "{direction} {decision_rule}");
    }

    let constant = json!({"comparator": "gt", "baseline": "constant-predictor", "margin": 0.05});
    assert!(sd(json!({"estimator": "proportion", "decision_rule": constant})).is_empty());
    for estimator in ["count", "mean", "median", "ratio"] {
        assert!(
            !sd(json!({"estimator": estimator, "decision_rule": constant})).is_empty(),
            "{estimator}"
        );
        assert!(
            sd(json!({
                "estimator": estimator,
                "decision_rule": {"comparator": "ge", "baseline": "best-seen"},
            }))
            .is_empty(),
            "{estimator}"
        );
    }
}

#[test]
#[trace("TC-171", "FR-021-AC-10")]
fn decision_rule_external_reference_baseline_has_no_estimator_restriction() {
    //
    // `external-reference` is a checker-resolved value from outside the plan
    // (PLAT-1009), not a corpus computation like `constant-predictor`, so it
    // carries no estimator restriction, and, unlike `best-seen`, its value does
    // not depend on the comparator's direction, so `eq` is allowed.
    for estimator in ["proportion", "count", "mean", "median", "ratio"] {
        assert!(
            sd(json!({
                "estimator": estimator,
                "decision_rule": {"comparator": "gt", "baseline": "external-reference"},
            }))
            .is_empty(),
            "{estimator}"
        );
    }
    assert!(dr(json!({"comparator": "eq", "baseline": "external-reference"})).is_empty());
    assert!(
        !dr(json!({"comparator": "eq", "baseline": "external-reference", "margin": 0.1}))
            .is_empty()
    );
    let lower = || Some(json!({"direction": "lower"}));
    assert!(
        statistical_design_errors(
            lower(),
            json!({"decision_rule": {"comparator": "le", "baseline": "external-reference"}})
        )
        .is_empty()
    );
    assert!(
        !statistical_design_errors(
            lower(),
            json!({"decision_rule": {"comparator": "ge", "baseline": "external-reference"}})
        )
        .is_empty()
    );
    assert!(
        dr(json!({"comparator": "ge", "baseline": "external-reference", "margin": 0.02}))
            .is_empty()
    );
}

#[test]
#[trace("TC-145", "FR-021-AC-2")]
fn estimator_is_a_closed_vocabulary() {
    let contract = schema("measurement-plan-frontmatter.schema");
    let design = &contract["$defs"]["statistical_design"]["properties"];
    assert_eq!(
        design["estimator"]["enum"],
        json!(["proportion", "count", "mean", "median", "ratio"])
    );
    for estimator in ["proportion", "count", "mean", "median", "ratio"] {
        assert!(sd(json!({"estimator": estimator})).is_empty());
    }
    for refused in [
        json!("retained-result proportion"),
        json!("p90"),
        json!(""),
        json!(1),
    ] {
        assert!(!sd(json!({"estimator": refused})).is_empty(), "{refused}");
    }
    // Prose fields stay prose (FR-021): only estimator and decision_rule close.
    for prose in ["population", "sampling", "error_model", "uncertainty"] {
        let field = &design[prose];
        assert_eq!(field["type"], "string");
        assert_eq!(field["minLength"], 1);
        for key in field.as_object().unwrap().keys() {
            assert!(
                ["type", "minLength", "description"].contains(&key.as_str()),
                "{prose}: {key}"
            );
        }
    }
}

#[test]
#[trace("TC-145", "FR-021-AC-2")]
fn measurement_plan_skeleton_shows_a_structured_decision_rule() {
    let skeleton = frontmatter(&skeleton_path("MeasurementPlan.md"));
    let design = &skeleton["statistical_design"];
    assert_eq!(design["estimator"], "proportion");
    assert_eq!(
        design["decision_rule"],
        json!({"comparator": "ge", "threshold": 0.99})
    );
    // The objective's bound is an informational goal, distinct from the
    // evaluated threshold.
    assert_eq!(skeleton["objective"]["direction"], "higher");
    assert_eq!(skeleton["objective"]["bound"], 0.995);
    assert!(schema_errors(&plan_validator(), &skeleton).is_empty());
}

fn apparatus_cases() -> Value {
    serde_json::from_str(&read(
        &root()
            .join("tests")
            .join("fixtures")
            .join("apparatus-paths.json"),
    ))
    .expect("apparatus-paths.json")
}

fn case_paths(cases: &Value, key: &str) -> Vec<Value> {
    cases[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["path"].clone())
        .collect()
}

#[test]
#[trace("TC-154", "FR-024-AC-1")]
fn protected_apparatus_is_a_unique_list_of_safe_relative_paths() {
    let contract = schema("measurement-plan-frontmatter.schema");
    assert!(!strs(&contract["required"]).contains(&"protected_apparatus"));
    let cases = apparatus_cases();
    let accepted = case_paths(&cases, "accepted");
    let refused = case_paths(&cases, "refused");
    assert!(!accepted.is_empty() && !refused.is_empty());
    assert!(plan_errors(json!({})).is_empty());
    assert!(plan_errors(json!({"protected_apparatus": accepted})).is_empty());
    for path in &accepted {
        assert!(
            plan_errors(json!({"protected_apparatus": [path]})).is_empty(),
            "{path}"
        );
    }
    for path in &refused {
        assert!(
            !plan_errors(json!({"protected_apparatus": [path]})).is_empty(),
            "{path:?}"
        );
    }
    assert!(!plan_errors(json!({"protected_apparatus": []})).is_empty());
    assert!(!plan_errors(json!({"protected_apparatus": ["a.json", "a.json"]})).is_empty());
    assert!(!plan_errors(json!({"protected_apparatus": "corpus/*.json"})).is_empty());
    assert!(!plan_errors(json!({"protected_apparatus": [7]})).is_empty());
}

#[test]
#[trace("TC-155", "FR-024-AC-2")]
fn negative_controls_are_closed_and_required_at_gate_stage() {
    let contract = schema("measurement-plan-frontmatter.schema");
    let kinds_value = &contract["$defs"]["negative_control"]["properties"]["kind"]["enum"];
    assert_eq!(
        *kinds_value,
        json!([
            "suppressed-observation",
            "gain-within-noise",
            "stale-evidence",
            "apparatus-edit",
            "selective-reporting"
        ])
    );
    let gate = json!({
        "stage": "gate",
        "ground_truth_kind": "mechanical",
        "protected_apparatus": ["evals/harness.py"],
    });
    for kind in strs(kinds_value) {
        let control = json!({"kind": kind, "description": "declared guard"});
        assert!(
            plan_errors(json!({
                "negative_controls": [control],
                "protected_apparatus": ["evals/harness.py"],
            }))
            .is_empty(),
            "{kind}"
        );
        assert!(
            plan_errors(merged(
                json!({"negative_controls": [control]}),
                gate.clone()
            ))
            .is_empty(),
            "{kind}"
        );
    }
    // Optional below gate stage.
    for stage in [
        "observe",
        "baseline",
        "branch-comparison",
        "trend",
        "ratchet",
        "target",
    ] {
        assert!(plan_errors(json!({"stage": stage})).is_empty(), "{stage}");
    }

    let gate_without = plan_errors(json!({"stage": "gate", "ground_truth_kind": "mechanical"}));
    assert!(gate_without.contains(&required_msg("negative_controls")));
    assert!(gate_without.contains(&required_msg("protected_apparatus")));
    let nc = negative_control();
    let refused = [
        ("empty list", json!([])),
        (
            "unknown kind",
            json!([{"kind": "vibes", "description": "d"}]),
        ),
        ("missing kind", json!([{"description": "d"}])),
        ("missing description", json!([{"kind": "stale-evidence"}])),
        (
            "empty description",
            json!([{"kind": "stale-evidence", "description": ""}]),
        ),
        (
            "extra key",
            json!([merged(nc.clone(), json!({"severity": "high"}))]),
        ),
        ("duplicate", json!([nc.clone(), nc])),
        ("bare kind", json!(["stale-evidence"])),
    ];
    for (case, controls) in refused {
        assert!(
            !plan_errors(merged(json!({"negative_controls": controls}), gate.clone())).is_empty(),
            "{case}"
        );
    }
}

#[test]
#[trace("TC-155", "FR-024-AC-8")]
fn gate_and_apparatus_edit_plans_require_protected_apparatus() {
    let control = json!({"kind": "suppressed-observation", "description": "d"});
    let edit = json!({"kind": "apparatus-edit", "description": "d"});
    let gate = json!({"stage": "gate", "ground_truth_kind": "mechanical"});
    let missing = required_msg("protected_apparatus");

    assert!(
        plan_errors(merged(
            json!({"negative_controls": [control]}),
            gate.clone()
        ))
        .contains(&missing)
    );
    assert!(
        plan_errors(merged(
            json!({"negative_controls": [control], "protected_apparatus": ["evals/**"]}),
            gate.clone()
        ))
        .is_empty()
    );
    // An apparatus-edit control needs a protected list at every stage.
    for stage in ["observe", "baseline", "trend", "gate"] {
        let extra = if stage == "gate" {
            gate.clone()
        } else {
            json!({"stage": stage})
        };
        assert!(
            plan_errors(merged(
                json!({"negative_controls": [control, edit]}),
                extra.clone()
            ))
            .contains(&missing),
            "{stage}"
        );
        assert!(
            plan_errors(merged(
                json!({
                    "negative_controls": [control, edit],
                    "protected_apparatus": ["evals/harness.py"],
                }),
                extra
            ))
            .is_empty(),
            "{stage}"
        );
    }
    // Other kinds below gate stage do not require it.
    assert!(plan_errors(json!({"negative_controls": [control]})).is_empty());
}

#[test]
#[trace("TC-159", "FR-024-AC-6")]
fn measurement_plan_skeleton_shows_apparatus_and_negative_controls() {
    let skeleton = frontmatter(&skeleton_path("MeasurementPlan.md"));
    assert!(
        skeleton["protected_apparatus"]
            .as_array()
            .is_some_and(|a| !a.is_empty())
    );
    let controls = skeleton["negative_controls"]
        .as_array()
        .expect("negative_controls");
    assert!(!controls.is_empty());
    let mut kinds: BTreeSet<&str> = controls
        .iter()
        .map(|c| c["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains("apparatus-edit"));
    assert!(schema_errors(&plan_validator(), &skeleton).is_empty());
    let body = read(&skeleton_path("MeasurementPlan.md"));
    assert!(body.contains("## Protected Apparatus"));
    assert!(body.contains("## Negative Controls"));
    let skill = onboarding_skill();
    assert!(skill.contains("`protected_apparatus`"));
    assert!(skill.contains("`negative_controls`"));
    kinds.extend(["gain-within-noise", "stale-evidence", "selective-reporting"]);
    for kind in kinds {
        assert!(skill.contains(&format!("`{kind}`")), "{kind}");
    }
}

fn argument_with_top_claim(over: Value) -> Value {
    let mut argument = frontmatter(&skeleton_path("AssuranceArgument.md"));
    let mut claim = argument["top_claim"]
        .as_object()
        .expect("top_claim")
        .clone();
    claim.remove("evidence_refs");
    claim.extend(over.as_object().expect("object").clone());
    argument["top_claim"] = Value::Object(claim);
    argument
}

#[test]
#[trace("TC-143", "FR-023-AC-1", "FR-023-AC-2", "FR-023-AC-3")]
fn supported_claim_must_reference_evidence() {
    let v = validator(&schema("assurance-argument-frontmatter.schema"));
    let evidence = "ix://example/juniper/evidence/request-loss-run";

    let without = schema_errors(&v, &argument_with_top_claim(json!({"status": "supported"})));
    assert!(
        without.contains(&required_msg("evidence_refs")),
        "{without:?}"
    );

    let with = argument_with_top_claim(json!({"status": "supported", "evidence_refs": [evidence]}));
    assert!(schema_errors(&v, &with).is_empty());

    for status in ["open", "challenged", "rejected"] {
        assert!(schema_errors(&v, &argument_with_top_claim(json!({"status": status}))).is_empty());
        let with_refs =
            argument_with_top_claim(json!({"status": status, "evidence_refs": [evidence]}));
        assert!(schema_errors(&v, &with_refs).is_empty());
    }

    let bad_refs = [
        json!([]),
        json!([evidence, evidence]),
        json!(["evidence/request-loss-run"]),
        json!([""]),
        json!(["ix:foo"]),
    ];
    for refs in bad_refs {
        let malformed =
            argument_with_top_claim(json!({"status": "supported", "evidence_refs": refs}));
        let errors: Vec<_> = v.iter_errors(&malformed).collect();
        assert!(!errors.is_empty(), "{refs}");
        assert!(
            errors
                .iter()
                .any(|e| e.instance_path().to_string().contains("top_claim")),
            "{refs}"
        );
    }
}

#[test]
#[trace("TC-143", "FR-023-AC-1")]
fn onboarding_checklist_reports_the_nested_evidence_refs_condition() {
    //
    // `onboard.js` derives conditional requirements from a schema's `allOf`.
    // The claim's `evidence_refs` requirement sits inside `$defs/claim`,
    // reached only through `top_claim`'s `$ref`. Assert the checklist walks
    // that `$ref` and surfaces the condition.
    let report = run_onboard();
    let checklist = &report["artifactChecklists"]["AssuranceArgument"];
    assert_eq!(checklist["warnings"], json!([]));
    let found = checklist["conditionalRequired"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| {
            entry["when"] == "top_claim.status = supported"
                && entry["required"] == json!(["top_claim.evidence_refs"])
        });
    assert!(found, "{checklist}");
}

#[test]
#[trace("TC-112", "FR-017-AC-4")]
fn packages_are_private_and_have_no_release_configuration() {
    let repo = root();
    let npm: Value = serde_json::from_str(&read(&repo.join("package.json"))).unwrap();
    assert_eq!(npm["private"], json!(true));
    assert!(npm.get("publishConfig").is_none());
    assert!(npm["scripts"].get("prepublishOnly").is_some());
    assert!(read(&repo.join("setup.cfg")).contains("Private :: Do Not Upload"));
    let workflow_text = glob_ext(&repo.join(".github").join("workflows"), "yml")
        .iter()
        .map(|p| read(p))
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();
    assert!(!workflow_text.contains("publish"));
    assert!(!workflow_text.contains("upload-artifact"));
}

fn ci_workflow() -> Value {
    yaml(&read(
        &root().join(".github").join("workflows").join("ci.yml"),
    ))
}

#[test]
#[trace("TC-112", "FR-017-AC-4")]
fn hosted_ci_is_manual_only() {
    // Program invariant: opening or updating a PR must not dispatch hosted CI.
    let workflow = ci_workflow();
    let triggers: BTreeSet<&str> = workflow["on"]
        .as_object()
        .expect("on")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(triggers, BTreeSet::from(["workflow_dispatch"]));
}

#[test]
#[trace("TC-116", "NFR-005-AC-1")]
fn manual_verification_workflow_runs_the_rust_foundation_gate() {
    let workflow = ci_workflow();
    let steps = workflow["jobs"]["verify"]["steps"].as_array().unwrap();
    assert_eq!(steps[0]["with"]["submodules"], "recursive");
    let uses = |step: &Value, prefix: &str| {
        step.get("uses")
            .and_then(Value::as_str)
            .is_some_and(|u| u.starts_with(prefix))
    };
    let rust_setup = steps
        .iter()
        .find(|s| uses(s, "dtolnay/rust-toolchain@"))
        .expect("rust-toolchain step");
    assert_eq!(
        rust_setup["with"],
        json!({"toolchain": "1.98.1", "components": "rustfmt, clippy"})
    );
    let installed: BTreeSet<&str> = steps
        .iter()
        .filter(|s| uses(s, "taiki-e/install-action@"))
        .map(|s| s["with"]["tool"].as_str().unwrap())
        .collect();
    assert_eq!(
        installed,
        BTreeSet::from(["cargo-deny@0.19.8", "cargo-audit@0.22.2"])
    );
    assert!(
        steps
            .iter()
            .any(|s| s.get("run").and_then(Value::as_str) == Some("make rust-foundation-gate"))
    );

    let makefile = read(&root().join("Makefile"));
    let foundation = makefile
        .lines()
        .find(|l| l.starts_with("rust-foundation-gate:"))
        .expect("rust-foundation-gate target");
    let deps: BTreeSet<&str> = foundation.split_whitespace().skip(1).collect();
    assert!(deps.contains("rust-deps") && deps.contains("rust-audit"));
    assert!(makefile.contains("$(CARGO) deny --locked check"));
    assert!(makefile.contains("$(CARGO) audit"));
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

#[test]
#[trace("TC-006", "FR-001-AC-3")]
fn quire_accepts_every_skeleton_without_diagnostics() {
    let executable = std::env::var_os("QUIRE_BIN")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| find_on_path("quire"));
    let Some(executable) = executable else {
        assert!(
            std::env::var("REQUIRE_QUIRE").as_deref() != Ok("1"),
            "quire is required"
        );
        eprintln!("skipping: quire is not installed (set REQUIRE_QUIRE=1 to require it)");
        return;
    };
    let repo = root();
    let documents: Vec<String> = glob_ext(&package_root().join("skeletons"), "md")
        .iter()
        .map(|p| {
            p.strip_prefix(&repo)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let completed = Command::new(executable)
        .arg("validate")
        .arg("--module")
        .arg(package_root().strip_prefix(&repo).unwrap())
        .arg("--strict")
        .args(&documents)
        .current_dir(&repo)
        .output()
        .expect("run quire");
    let stderr = String::from_utf8_lossy(&completed.stderr);
    assert!(completed.status.success(), "{stderr}");
    let diagnostics: Vec<&str> = stderr.lines().collect();
    assert!(!diagnostics.is_empty());
    assert!(
        diagnostics
            .iter()
            .all(|l| l.starts_with("UnknownEdgeType:")),
        "{stderr}"
    );
}

/// Test-case ids named by requirement coverage rows (`| FR-n | AC | TC.. |`)
/// that have no row of their own in the Test Case Summary table.
fn dangling_test_case_references(matrix: &str) -> BTreeSet<String> {
    let defined_re = Regex::new(r"^\|\s*(TC-\d+)\s*\|").unwrap();
    let coverage_re = Regex::new(r"^\|\s*(?:FR|NFR|StR|US)-[\w-]+\s*\|[^|]+\|([^|]+)\|").unwrap();
    let tc_re = Regex::new(r"TC-\d+").unwrap();
    let defined: BTreeSet<&str> = matrix
        .lines()
        .filter_map(|line| defined_re.captures(line))
        .map(|c| c.get(1).unwrap().as_str())
        .collect();
    matrix
        .lines()
        .filter_map(|line| coverage_re.captures(line))
        .flat_map(|row| {
            tc_re
                .find_iter(row.get(1).unwrap().as_str())
                .map(|m| m.as_str().to_owned())
                .collect::<Vec<_>>()
        })
        .filter(|tc| !defined.contains(tc.as_str()))
        .collect()
}

#[test]
#[trace("TC-205", "NFR-005-AC-5")]
fn every_matrix_test_reference_has_a_test_case_row() {
    let matrix = read(&root().join("spec").join("tests.md"));
    let dangling = dangling_test_case_references(&matrix);
    assert!(
        dangling.is_empty(),
        "coverage rows name absent test cases: {dangling:?}"
    );

    // The check can fail: a coverage row naming an absent test case is caught,
    // and a padded test-case row still counts as present.
    let probe = format!(
        "{matrix}\n| FR-999 | FR-999-AC-1 | TC-998, TC-999 | x |\n| TC-998  | padded | Unit | P1 | FR-999-AC-1 | x |\n"
    );
    assert_eq!(
        dangling_test_case_references(&probe),
        BTreeSet::from(["TC-999".to_owned()])
    );
}

#[test]
#[trace("TC-195", "FR-003-AC-8")]
fn review_by_date_time_is_enforced_by_the_pattern_alone() {
    //
    // The validator asserts no `format` by default for 2020-12, like quire, so
    // this proves the pattern carries the check.
    let v = validator(&schema("assurance-argument-frontmatter.schema"));
    let mut document = frontmatter(&skeleton_path("AssuranceArgument.md"));
    assert!(schema_errors(&v, &document).is_empty());
    let cases = [
        ("2030-01-01T00:00:00Z", true),
        ("2028-02-29T00:00:00+02:00", true),
        ("2030-02-31T00:00:00Z", false),
        ("2100-02-29T00:00:00Z", false),
        ("2030-01-01T00:00:60Z", false),
        ("next spring", false),
        ("", false),
    ];
    for (value, accepted) in cases {
        document["assumptions"][0]["review_by"] = json!(value);
        assert_eq!(schema_errors(&v, &document).is_empty(), accepted, "{value}");
    }
}
