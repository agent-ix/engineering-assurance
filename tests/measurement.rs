// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-020 `MeasurementPlan` objective types, schema parity, the definition-change
//! check, and the narrow `measurement` feature; FR-021 decision-rule and
//! estimator vocabulary, its schema parity, and rule evaluation.

use std::{collections::BTreeSet, fs, path::Path, process::Command};

use engineering_assurance::measurement::{
    Baseline, Comparator, DecisionRule, DecisionRuleError, Direction, Estimator, Objective,
    ObjectiveChangedWithoutVersionBump, ObjectiveError, PlanDefinition, RuleEvaluationError,
    RuleReference, objective_change_without_version_bump,
};
use ix_trace_rs::trace;

fn objective(direction: Direction, bound: Option<f64>) -> Objective {
    Objective::new(direction, bound).expect("valid objective")
}

fn parse(yaml: &str) -> Result<Objective, String> {
    yaml_serde::from_str::<Objective>(yaml).map_err(|error| error.to_string())
}

fn parse_rule(yaml: &str) -> Result<DecisionRule, String> {
    yaml_serde::from_str::<DecisionRule>(yaml).map_err(|error| error.to_string())
}

fn measurement_plan_schema() -> serde_json::Value {
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("engineering_assurance/schemas/measurement-plan-frontmatter.schema.json");
    serde_json::from_slice(&fs::read(&schema_path).expect("schema readable"))
        .expect("schema is JSON")
}

/// The schema enum at `pointer`, as owned strings, in schema order.
fn schema_enum(schema: &serde_json::Value, pointer: &str) -> Vec<String> {
    schema
        .pointer(pointer)
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| panic!("schema enum at {pointer}"))
        .iter()
        .map(|value| value.as_str().expect("enum value is a string").to_owned())
        .collect()
}

/// Assert a schema enum equals a Rust enum's wire names in order, and that
/// every schema value round-trips through serde to the same variant.
fn assert_wire_parity<T>(schema_values: &[String], all: &[T], wire_name: fn(&T) -> &'static str)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let rust_values = all
        .iter()
        .map(|variant| {
            assert_eq!(
                serde_json::to_value(variant).expect("variant serializes"),
                wire_name(variant),
                "serde and wire_name agree for {variant:?}"
            );
            wire_name(variant).to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(schema_values, rust_values.as_slice());
    for (wire, expected) in schema_values.iter().zip(all) {
        let decoded: T = serde_json::from_value(serde_json::Value::String(wire.clone()))
            .unwrap_or_else(|error| panic!("schema value {wire:?} must deserialize: {error}"));
        assert_eq!(&decoded, expected, "schema value {wire:?} decodes to the wrong variant");
    }
}

#[test]
#[trace("TC-140", "FR-020-AC-2")]
fn tc_140_objective_construction_refuses_target_without_bound_and_non_finite_bounds() {
    for direction in Direction::ALL {
        let bound = direction.requires_bound().then_some(1.0);
        let built = objective(direction, bound);
        assert_eq!(built.direction(), direction);
        assert_eq!(built.bound(), bound);
        assert_eq!(objective(direction, Some(0.5)).bound(), Some(0.5));
    }
    for direction in [Direction::Higher, Direction::Lower, Direction::Zero] {
        assert_eq!(objective(direction, None).bound(), None);
    }

    assert_eq!(
        Objective::new(Direction::Target, None),
        Err(ObjectiveError::TargetWithoutBound)
    );
    assert_eq!(
        Objective::new(Direction::Higher, Some(f64::INFINITY)),
        Err(ObjectiveError::NonFiniteBound {
            bound: f64::INFINITY
        })
    );
    assert!(matches!(
        Objective::new(Direction::Target, Some(f64::NAN)),
        Err(ObjectiveError::NonFiniteBound { bound }) if bound.is_nan()
    ));
}

#[test]
#[trace("TC-140", "FR-020-AC-2")]
fn tc_140_objective_deserialization_is_closed_and_validated() {
    assert_eq!(
        parse("direction: target\nbound: 250\n"),
        Ok(objective(Direction::Target, Some(250.0)))
    );
    assert_eq!(
        parse("direction: lower\n"),
        Ok(objective(Direction::Lower, None))
    );

    let target_without_bound = parse("direction: target\n").expect_err("target needs a bound");
    assert!(
        target_without_bound.contains(&ObjectiveError::TargetWithoutBound.to_string()),
        "{target_without_bound}"
    );
    let infinite = parse("direction: higher\nbound: .inf\n").expect_err("bound must be finite");
    assert!(infinite.contains("is not a finite number"), "{infinite}");
    let not_a_number = parse("direction: higher\nbound: .nan\n").expect_err("bound must be finite");
    assert!(
        not_a_number.contains("is not a finite number"),
        "{not_a_number}"
    );
    for refused in [
        "direction: sideways\n",
        "bound: 1\n",
        "direction: target\nbound: \"250\"\n",
        "direction: higher\nepoch: 2\n",
    ] {
        assert!(parse(refused).is_err(), "must refuse {refused:?}");
    }

    let round_trip = objective(Direction::Zero, None);
    let emitted = yaml_serde::to_string(&round_trip).expect("objective serializes");
    assert_eq!(emitted, "direction: zero\n");
    assert_eq!(parse(&emitted), Ok(round_trip));
}

#[test]
#[trace("TC-140", "FR-020-AC-2")]
fn tc_140_schema_direction_enum_equals_the_rust_wire_names() {
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("engineering_assurance/schemas/measurement-plan-frontmatter.schema.json");
    let schema: serde_json::Value =
        serde_json::from_slice(&fs::read(&schema_path).expect("schema readable"))
            .expect("schema is JSON");
    assert_eq!(
        schema["properties"]["objective"]["$ref"], "#/$defs/objective",
        "objective must resolve to the $defs entry this test reads"
    );
    let schema_directions = schema["$defs"]["objective"]["properties"]["direction"]["enum"]
        .as_array()
        .expect("direction enum")
        .iter()
        .map(|value| value.as_str().expect("direction is a string").to_owned())
        .collect::<Vec<_>>();

    let rust_directions = Direction::ALL
        .iter()
        .map(|direction| {
            let wire = serde_json::to_value(direction).expect("direction serializes");
            assert_eq!(wire, direction.wire_name(), "serde and wire_name agree");
            direction.to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(schema_directions, rust_directions);
    assert_eq!(
        schema["$defs"]["objective"]["allOf"][0]["then"]["required"],
        serde_json::json!(["bound"])
    );

    // Parity is only proven if every schema enum value also round-trips
    // through serde back to the same `Direction` variant it named.
    for (index, wire_name) in schema_directions.iter().enumerate() {
        let decoded: Direction = serde_json::from_value(serde_json::Value::String(
            wire_name.clone(),
        ))
        .unwrap_or_else(|error| {
            panic!("schema direction {wire_name:?} must deserialize as a Direction: {error}")
        });
        assert_eq!(
            decoded,
            Direction::ALL[index],
            "schema direction {wire_name:?} round-trips to the wrong Direction variant"
        );
        let re_encoded = serde_json::to_value(decoded).expect("direction serializes");
        assert_eq!(re_encoded, serde_json::Value::String(wire_name.clone()));
    }
}

#[test]
#[trace("TC-140", "FR-020-AC-2")]
fn tc_140_direction_all_covers_every_variant() {
    // Exhaustive match: if `Direction` gains a variant, this fails to
    // compile until an arm naming that variant's expected `Direction::ALL`
    // index is added below. Each variant is then checked against
    // `Direction::ALL` at that position, so a variant added to the enum but
    // left out of (or misordered in) `ALL` fails this test instead of
    // silently dropping out of everywhere `ALL` is iterated.
    for direction in [
        Direction::Higher,
        Direction::Lower,
        Direction::Zero,
        Direction::Target,
    ] {
        let expected_index = match direction {
            Direction::Higher => 0,
            Direction::Lower => 1,
            Direction::Zero => 2,
            Direction::Target => 3,
        };
        assert_eq!(
            Direction::ALL.get(expected_index),
            Some(&direction),
            "Direction::ALL is missing {direction:?} at index {expected_index}"
        );
    }
    assert_eq!(
        Direction::ALL.len(),
        4,
        "Direction::ALL must list exactly one entry per Direction variant"
    );
}

#[test]
#[trace("TC-141", "FR-020-AC-3")]
fn tc_141_objective_edit_without_a_version_bump_is_a_typed_finding() {
    let higher = Some(objective(Direction::Higher, Some(0.99)));
    let lower = Some(objective(Direction::Lower, Some(0.99)));
    let raised = Some(objective(Direction::Higher, Some(0.995)));
    let plan = |version, objective| PlanDefinition {
        definition_version: version,
        objective,
    };

    for (before, after) in [
        (higher, lower),
        (higher, raised),
        (None, higher),
        (higher, None),
    ] {
        assert_eq!(
            objective_change_without_version_bump(
                &plan(Some("retention-v1"), before),
                &plan(Some("retention-v1"), after)
            ),
            Some(ObjectiveChangedWithoutVersionBump {
                definition_version: Some("retention-v1".to_owned()),
                before,
                after,
            })
        );
        assert_eq!(
            objective_change_without_version_bump(&plan(None, before), &plan(None, after)),
            Some(ObjectiveChangedWithoutVersionBump {
                definition_version: None,
                before,
                after,
            })
        );
        assert_eq!(
            objective_change_without_version_bump(
                &plan(Some("retention-v1"), before),
                &plan(Some("retention-v2"), after)
            ),
            None
        );
        assert_eq!(
            objective_change_without_version_bump(
                &plan(None, before),
                &plan(Some("retention-v1"), after)
            ),
            None
        );
        // Clearing `definition_version` (Some -> None) is not a bump: it
        // still yields a finding, carrying the version that was cleared.
        assert_eq!(
            objective_change_without_version_bump(
                &plan(Some("retention-v1"), before),
                &plan(None, after)
            ),
            Some(ObjectiveChangedWithoutVersionBump {
                definition_version: Some("retention-v1".to_owned()),
                before,
                after,
            })
        );
    }
    for unchanged in [None, higher] {
        assert_eq!(
            objective_change_without_version_bump(
                &plan(Some("retention-v1"), unchanged),
                &plan(Some("retention-v1"), unchanged)
            ),
            None
        );
    }
}

/// Quoin's measurement intake imports these types with
/// `default-features = false, features = ["measurement"]`; like
/// `tc_138_a_minimal_downstream_compiles_only_the_source_audit_feature`, this
/// proves that consumer never resolves `serde_json`, so it cannot inherit the
/// `arbitrary_precision` flip `full` carries.
#[test]
#[trace("TC-142", "FR-020-AC-4", "FR-021-AC-7")]
fn tc_142_a_minimal_downstream_compiles_only_the_measurement_feature() {
    let consumer = tempfile::tempdir().expect("consumer root");
    fs::create_dir(consumer.path().join("src")).expect("consumer source directory");
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    fs::write(
        consumer.path().join("Cargo.toml"),
        format!(
            "[package]\nname='measurement-consumer-fixture'\nversion='0.0.0'\nedition='2024'\nrust-version='1.98.1'\n[dependencies]\nengineering-assurance={{path={manifest_dir:?},default-features=false,features=['measurement']}}\n"
        ),
    )
    .expect("consumer manifest");
    fs::write(
        consumer.path().join("src/main.rs"),
        "use engineering_assurance::measurement::{Baseline, Comparator, DecisionRule, Direction, Estimator, Objective, objective_change_without_version_bump};\nfn main(){let _ = (Objective::new(Direction::Target, Some(1.0)), objective_change_without_version_bump, DecisionRule::against_baseline(Comparator::Gt, Baseline::ConstantPredictor, None), Estimator::Proportion);}\n",
    )
    .expect("consumer source");
    let status = Command::new(env!("CARGO"))
        .args(["check", "--offline", "--manifest-path"])
        .arg(consumer.path().join("Cargo.toml"))
        .env("CARGO_BUILD_JOBS", "2")
        .env("CARGO_TARGET_DIR", consumer.path().join("target"))
        .status()
        .expect("consumer cargo check must launch");
    assert!(
        status.success(),
        "minimal measurement consumer must compile"
    );

    let metadata = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--offline",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(consumer.path().join("Cargo.toml"))
        .output()
        .expect("consumer metadata must launch");
    assert!(metadata.status.success());
    let graph: serde_json::Value = serde_json::from_slice(&metadata.stdout).expect("metadata JSON");
    let resolved_packages = graph["packages"]
        .as_array()
        .expect("metadata packages")
        .iter()
        .filter_map(|package| package["name"].as_str())
        .collect::<BTreeSet<_>>();
    assert!(
        resolved_packages.contains("engineering-assurance"),
        "{resolved_packages:?}"
    );
    assert!(
        !resolved_packages.contains("serde_json"),
        "a measurement-only consumer must not resolve serde_json anywhere \
         in its dependency graph: {resolved_packages:?}"
    );
}

#[test]
#[trace("TC-146", "FR-021-AC-3")]
fn tc_146_decision_rule_construction_and_deserialization_are_closed_and_validated() {
    let threshold = DecisionRule::against_threshold(Comparator::Ge, 0.99).expect("valid rule");
    assert_eq!(threshold.comparator(), Comparator::Ge);
    assert_eq!(threshold.reference(), RuleReference::Threshold(0.99));
    let baseline = DecisionRule::against_baseline(
        Comparator::Gt,
        Baseline::ConstantPredictor,
        Some(0.05),
    )
    .expect("valid rule");
    assert_eq!(
        baseline.reference(),
        RuleReference::Baseline {
            baseline: Baseline::ConstantPredictor,
            margin: 0.05
        }
    );
    assert_eq!(
        DecisionRule::against_baseline(Comparator::Ge, Baseline::BestSeen, None)
            .expect("valid rule")
            .reference(),
        RuleReference::Baseline {
            baseline: Baseline::BestSeen,
            margin: 0.0
        }
    );

    assert_eq!(
        DecisionRule::against_threshold(Comparator::Ge, f64::INFINITY),
        Err(DecisionRuleError::NonFiniteThreshold {
            threshold: f64::INFINITY
        })
    );
    assert!(matches!(
        DecisionRule::against_baseline(Comparator::Ge, Baseline::BestSeen, Some(f64::NAN)),
        Err(DecisionRuleError::NonFiniteMargin { margin }) if margin.is_nan()
    ));

    assert_eq!(parse_rule("comparator: ge\nthreshold: 0.99\n"), Ok(threshold));
    assert_eq!(
        parse_rule("comparator: gt\nbaseline: constant-predictor\nmargin: 0.05\n"),
        Ok(baseline)
    );
    for (yaml, error) in [
        ("comparator: ge\n", DecisionRuleError::MissingReference),
        (
            "comparator: ge\nthreshold: 1\nbaseline: best-seen\n",
            DecisionRuleError::ThresholdAndBaseline,
        ),
        (
            "comparator: ge\nthreshold: 1\nmargin: 0.1\n",
            DecisionRuleError::MarginWithoutBaseline,
        ),
    ] {
        let refused = parse_rule(yaml).expect_err(yaml);
        assert!(refused.contains(&error.to_string()), "{yaml:?}: {refused}");
    }
    let infinite = parse_rule("comparator: ge\nthreshold: .inf\n").expect_err("finite threshold");
    assert!(infinite.contains("is not a finite number"), "{infinite}");
    let nan_margin = parse_rule("comparator: ge\nbaseline: best-seen\nmargin: .nan\n")
        .expect_err("finite margin");
    assert!(nan_margin.contains("is not a finite number"), "{nan_margin}");
    for refused in [
        "comparator: approximately\nthreshold: 1\n",
        "threshold: 1\n",
        "comparator: ge\nbaseline: vibes\n",
        "comparator: ge\nthreshold: \"0.99\"\n",
        "comparator: ge\nthreshold: 1\nrepetitions: 5\n",
        "comparator: ge\nthreshold: 1\nminimum_n: 20\n",
        "comparator: ge\nthreshold: 1\nmetric: retention\n",
    ] {
        assert!(parse_rule(refused).is_err(), "must refuse {refused:?}");
    }
    for estimator in ["retained-result proportion", "p90", ""] {
        assert!(
            yaml_serde::from_str::<Estimator>(estimator).is_err(),
            "must refuse estimator {estimator:?}"
        );
    }

    for rule in [threshold, baseline] {
        let emitted = yaml_serde::to_string(&rule).expect("rule serializes");
        assert_eq!(parse_rule(&emitted), Ok(rule), "{emitted}");
    }
    assert_eq!(
        yaml_serde::to_string(
            &DecisionRule::against_baseline(Comparator::Eq, Baseline::PriorCollection, None)
                .expect("valid rule")
        )
        .expect("rule serializes"),
        "comparator: eq\nbaseline: prior-collection\n"
    );
}

#[test]
#[trace("TC-147", "FR-021-AC-4")]
fn tc_147_schema_estimator_comparator_and_baseline_enums_equal_the_rust_wire_names() {
    let schema = measurement_plan_schema();
    assert_eq!(
        schema["$defs"]["statistical_design"]["properties"]["decision_rule"]["$ref"],
        "#/$defs/decision_rule",
        "decision_rule must resolve to the $defs entry this test reads"
    );
    assert_wire_parity(
        &schema_enum(&schema, "/$defs/statistical_design/properties/estimator/enum"),
        &Estimator::ALL,
        |estimator| estimator.wire_name(),
    );
    assert_wire_parity(
        &schema_enum(&schema, "/$defs/decision_rule/properties/comparator/enum"),
        &Comparator::ALL,
        |comparator| comparator.wire_name(),
    );
    assert_wire_parity(
        &schema_enum(&schema, "/$defs/decision_rule/properties/baseline/enum"),
        &Baseline::ALL,
        |baseline| baseline.wire_name(),
    );
    for (value, name) in [
        (Estimator::Ratio.to_string(), "ratio"),
        (Comparator::Le.to_string(), "le"),
        (Baseline::BestSeen.to_string(), "best-seen"),
    ] {
        assert_eq!(value, name);
    }
}

#[test]
#[trace("TC-147", "FR-021-AC-4")]
fn tc_147_all_constants_cover_every_variant() {
    // Exhaustive matches: a new variant fails to compile here until its
    // expected `ALL` index is named, so it cannot silently drop out of the
    // schema parity check above.
    for estimator in Estimator::ALL {
        let index = match estimator {
            Estimator::Proportion => 0,
            Estimator::Count => 1,
            Estimator::Mean => 2,
            Estimator::Median => 3,
            Estimator::Ratio => 4,
        };
        assert_eq!(Estimator::ALL.get(index), Some(&estimator));
    }
    for comparator in Comparator::ALL {
        let index = match comparator {
            Comparator::Gt => 0,
            Comparator::Ge => 1,
            Comparator::Lt => 2,
            Comparator::Le => 3,
            Comparator::Eq => 4,
        };
        assert_eq!(Comparator::ALL.get(index), Some(&comparator));
    }
    for baseline in Baseline::ALL {
        let index = match baseline {
            Baseline::ConstantPredictor => 0,
            Baseline::PriorCollection => 1,
            Baseline::BestSeen => 2,
        };
        assert_eq!(Baseline::ALL.get(index), Some(&baseline));
    }
    assert_eq!(
        (Estimator::ALL.len(), Comparator::ALL.len(), Baseline::ALL.len()),
        (5, 5, 3)
    );
}

#[test]
#[trace("TC-148", "FR-021-AC-5")]
fn tc_148_decision_rule_evaluation_compares_the_estimate_with_its_reference() {
    let below_at_above = [0.5, 1.0, 1.5];
    for (comparator, expected) in [
        (Comparator::Gt, [false, false, true]),
        (Comparator::Ge, [false, true, true]),
        (Comparator::Lt, [true, false, false]),
        (Comparator::Le, [true, true, false]),
        (Comparator::Eq, [false, true, false]),
    ] {
        let rule = DecisionRule::against_threshold(comparator, 1.0).expect("valid rule");
        for (estimate, holds) in below_at_above.into_iter().zip(expected) {
            assert_eq!(comparator.holds(estimate, 1.0), holds, "{comparator} {estimate}");
            assert_eq!(rule.holds(estimate, None), Ok(holds), "{comparator} {estimate}");
        }
    }

    // reference = baseline value + margin: beating a 0.80 constant predictor
    // by five points needs more than 0.85.
    let margin_rule =
        DecisionRule::against_baseline(Comparator::Gt, Baseline::ConstantPredictor, Some(0.05))
            .expect("valid rule");
    assert_eq!(margin_rule.holds(0.86, Some(0.80)), Ok(true));
    assert_eq!(margin_rule.holds(0.84, Some(0.80)), Ok(false));
    // A negative margin tolerates a bounded regression against best-seen.
    let tolerant = DecisionRule::against_baseline(Comparator::Ge, Baseline::BestSeen, Some(-0.25))
        .expect("valid rule");
    assert_eq!(tolerant.holds(0.75, Some(1.0)), Ok(true));
    assert_eq!(tolerant.holds(0.5, Some(1.0)), Ok(false));

    assert_eq!(
        margin_rule.holds(0.9, None),
        Err(RuleEvaluationError::MissingBaselineValue {
            baseline: Baseline::ConstantPredictor
        })
    );
    assert_eq!(
        DecisionRule::against_threshold(Comparator::Eq, 0.0)
            .expect("valid rule")
            .holds(0.0, Some(0.0)),
        Err(RuleEvaluationError::UnexpectedBaselineValue)
    );
    assert!(matches!(
        margin_rule.holds(f64::NAN, Some(0.8)),
        Err(RuleEvaluationError::NonFiniteEstimate { estimate }) if estimate.is_nan()
    ));
    assert_eq!(
        margin_rule.holds(0.9, Some(f64::NEG_INFINITY)),
        Err(RuleEvaluationError::NonFiniteBaselineValue {
            value: f64::NEG_INFINITY
        })
    );
}
