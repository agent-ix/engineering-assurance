// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-020 `MeasurementPlan` objective types, schema parity, the definition-change
//! check, and the narrow `measurement` feature; FR-021 decision-rule and
//! estimator vocabulary, its schema parity, and rule evaluation; FR-024
//! protected apparatus and negative controls, their schema parity, and
//! protected-list edits in the definition-change check.

use std::{collections::BTreeSet, fs, path::Path, process::Command};

use engineering_assurance::measurement::{
    ApparatusPath, ApparatusPathError, Baseline, Comparator, DecisionRule, DecisionRuleError,
    DefinitionChangedWithoutVersionBump, DefinitionMember, Direction, Estimator,
    MeasurementDefinition, NegativeControl, NegativeControlError, NegativeControlKind,
    NegativeControls, Objective, ObjectiveError, PlanDefinition, ProtectedApparatus,
    ProtectedApparatusError, RuleEvaluationError, RuleReference,
    definition_change_without_version_bump,
};
use ix_trace_rs::trace;

fn objective(direction: Direction, bound: Option<f64>) -> Objective {
    Objective::new(direction, bound).expect("valid objective")
}

fn parse(yaml: &str) -> Result<Objective, String> {
    yaml_serde::from_str::<Objective>(yaml).map_err(|error| error.to_string())
}

fn rule_threshold(comparator: Comparator, threshold: f64) -> DecisionRule {
    DecisionRule::against_threshold(comparator, threshold).expect("valid rule")
}

fn rule_baseline(comparator: Comparator, baseline: Baseline, margin: Option<f64>) -> DecisionRule {
    DecisionRule::against_baseline(comparator, baseline, margin).expect("valid rule")
}

fn apparatus(paths: &[&str]) -> ProtectedApparatus {
    ProtectedApparatus::new(
        paths
            .iter()
            .map(|path| ApparatusPath::new(*path).expect("valid apparatus path")),
    )
    .expect("valid protected apparatus")
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
        assert_eq!(
            &decoded, expected,
            "schema value {wire:?} decodes to the wrong variant"
        );
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

/// An edit from `before` to `after` changing exactly `changed` is a finding
/// under an equal, absent, or cleared `definition_version`, and no finding
/// under a genuine bump.
fn assert_edit_is_reported_unless_versioned(
    before: &MeasurementDefinition,
    after: &MeasurementDefinition,
    changed: &[DefinitionMember],
) {
    let plan = |version, definition: &MeasurementDefinition| PlanDefinition {
        definition_version: version,
        definition: definition.clone(),
    };
    let finding = |version: Option<&str>| {
        Some(DefinitionChangedWithoutVersionBump {
            definition_version: version.map(str::to_owned),
            changed: changed.to_vec(),
            before: before.clone(),
            after: after.clone(),
        })
    };
    // Unchanged version, absent version, and a cleared version (Some -> None,
    // which is not a bump and carries the version that was cleared).
    for (before_version, after_version, expected) in [
        (
            Some("retention-v1"),
            Some("retention-v1"),
            finding(Some("retention-v1")),
        ),
        (None, None, finding(None)),
        (Some("retention-v1"), None, finding(Some("retention-v1"))),
        (Some("retention-v1"), Some("retention-v2"), None),
        (None, Some("retention-v1"), None),
    ] {
        assert_eq!(
            definition_change_without_version_bump(
                &plan(before_version, before),
                &plan(after_version, after)
            ),
            expected,
            "{before_version:?} -> {after_version:?}: {changed:?}"
        );
    }
}

#[test]
#[trace("TC-141", "FR-020-AC-3", "FR-021-AC-8", "FR-024-AC-5")]
fn tc_141_definition_edit_without_a_version_bump_is_a_typed_finding_naming_the_member() {
    let higher = Some(objective(Direction::Higher, Some(0.995)));
    let lower = Some(objective(Direction::Lower, Some(0.995)));
    let raised = Some(objective(Direction::Higher, Some(0.999)));
    let ge_099 = Some(rule_threshold(Comparator::Ge, 0.99));
    let ge_095 = Some(rule_threshold(Comparator::Ge, 0.95));
    let base = MeasurementDefinition {
        objective: higher,
        estimator: Some(Estimator::Proportion),
        decision_rule: ge_099,
        protected_apparatus: Some(apparatus(&["evals/harness.py", "labels/*.json"])),
    };
    let plan = |version, definition| PlanDefinition {
        definition_version: version,
        definition,
    };

    let edits = [
        (
            MeasurementDefinition {
                objective: lower,
                ..base.clone()
            },
            vec![DefinitionMember::Objective],
        ),
        (
            MeasurementDefinition {
                objective: raised,
                ..base.clone()
            },
            vec![DefinitionMember::Objective],
        ),
        (
            MeasurementDefinition {
                objective: None,
                ..base.clone()
            },
            vec![DefinitionMember::Objective],
        ),
        (
            MeasurementDefinition {
                estimator: Some(Estimator::Mean),
                ..base.clone()
            },
            vec![DefinitionMember::Estimator],
        ),
        (
            MeasurementDefinition {
                estimator: None,
                ..base.clone()
            },
            vec![DefinitionMember::Estimator],
        ),
        (
            MeasurementDefinition {
                decision_rule: ge_095,
                ..base.clone()
            },
            vec![DefinitionMember::DecisionRule],
        ),
        (
            MeasurementDefinition {
                decision_rule: None,
                ..base.clone()
            },
            vec![DefinitionMember::DecisionRule],
        ),
        (
            MeasurementDefinition {
                objective: raised,
                estimator: Some(Estimator::Mean),
                decision_rule: ge_095,
                protected_apparatus: None,
            },
            DefinitionMember::ALL.to_vec(),
        ),
    ];
    for (edited, changed) in edits {
        // Both directions: an edit and its reversal, so additions and
        // removals are both covered.
        assert_edit_is_reported_unless_versioned(&base, &edited, &changed);
        assert_edit_is_reported_unless_versioned(&edited, &base, &changed);
    }
    for unchanged in [MeasurementDefinition::default(), base] {
        assert_eq!(
            definition_change_without_version_bump(
                &plan(Some("retention-v1"), unchanged.clone()),
                &plan(Some("retention-v1"), unchanged)
            ),
            None
        );
    }
    let paths = DefinitionMember::ALL
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        [
            "objective",
            "statistical_design.estimator",
            "statistical_design.decision_rule",
            "protected_apparatus"
        ]
    );
}

/// Quoin's measurement intake imports these types with
/// `default-features = false, features = ["measurement"]`; like
/// `tc_138_a_minimal_downstream_compiles_only_the_source_audit_feature`, this
/// proves that consumer never resolves `serde_json`, so it cannot inherit the
/// `arbitrary_precision` flip `full` carries.
#[test]
#[trace("TC-142", "FR-020-AC-4", "FR-021-AC-7", "FR-024-AC-7")]
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
        "use engineering_assurance::measurement::{ApparatusPath, Baseline, Comparator, DecisionRule, Direction, Estimator, NegativeControl, NegativeControlKind, NegativeControls, Objective, ProtectedApparatus, definition_change_without_version_bump};\nfn main(){let _ = (Objective::new(Direction::Target, Some(1.0)), definition_change_without_version_bump, DecisionRule::against_baseline(Comparator::Gt, Baseline::ConstantPredictor, None), Estimator::Proportion, ApparatusPath::new(\"evals/**/*.json\").map(|path| ProtectedApparatus::new([path])), NegativeControl::new(NegativeControlKind::ApparatusEdit, \"digest\").map(|control| NegativeControls::new([control])));}\n",
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
    let baseline =
        DecisionRule::against_baseline(Comparator::Gt, Baseline::ConstantPredictor, Some(0.05))
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
    assert_eq!(
        DecisionRule::against_baseline(Comparator::Eq, Baseline::BestSeen, None),
        Err(DecisionRuleError::EqAgainstBestSeen)
    );
    assert!(matches!(
        DecisionRule::against_baseline(Comparator::Ge, Baseline::BestSeen, Some(f64::NAN)),
        Err(DecisionRuleError::NonFiniteMargin { margin }) if margin.is_nan()
    ));

    assert_eq!(
        parse_rule("comparator: ge\nthreshold: 0.99\n"),
        Ok(threshold)
    );
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
        (
            "comparator: eq\nbaseline: best-seen\n",
            DecisionRuleError::EqAgainstBestSeen,
        ),
    ] {
        let refused = parse_rule(yaml).expect_err(yaml);
        assert!(refused.contains(&error.to_string()), "{yaml:?}: {refused}");
    }
    let infinite = parse_rule("comparator: ge\nthreshold: .inf\n").expect_err("finite threshold");
    assert!(infinite.contains("is not a finite number"), "{infinite}");
    let nan_margin = parse_rule("comparator: ge\nbaseline: best-seen\nmargin: .nan\n")
        .expect_err("finite margin");
    assert!(
        nan_margin.contains("is not a finite number"),
        "{nan_margin}"
    );
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
#[trace("TC-146", "FR-021-AC-3")]
fn tc_146_decision_rule_refuses_margin_misuse_and_non_numeric_margin() {
    assert_eq!(
        DecisionRule::against_baseline(Comparator::Eq, Baseline::PriorCollection, Some(0.1)),
        Err(DecisionRuleError::MarginWithEq)
    );
    let eq_margin = parse_rule("comparator: eq\nbaseline: prior-collection\nmargin: 0.1\n")
        .expect_err("eq takes no margin");
    assert!(
        eq_margin.contains(&DecisionRuleError::MarginWithEq.to_string()),
        "{eq_margin}"
    );
    for refused in [
        "comparator: ge\nbaseline: best-seen\nmargin: \"0.05\"\n",
        "comparator: ge\nbaseline: best-seen\nmargin: [0.05]\n",
    ] {
        assert!(parse_rule(refused).is_err(), "must refuse {refused:?}");
    }
}

#[test]
#[trace("TC-146", "FR-021-AC-9")]
fn tc_146_decision_rule_agrees_with_the_objective_direction_and_estimator() {
    let agreeing = [
        (Direction::Higher, rule_threshold(Comparator::Gt, 0.9)),
        (Direction::Higher, rule_threshold(Comparator::Ge, 0.9)),
        (Direction::Lower, rule_threshold(Comparator::Lt, 250.0)),
        (
            Direction::Lower,
            rule_baseline(Comparator::Le, Baseline::BestSeen, None),
        ),
        (Direction::Zero, rule_threshold(Comparator::Eq, 0.0)),
        (Direction::Zero, rule_threshold(Comparator::Le, 0.0)),
        (Direction::Target, rule_threshold(Comparator::Lt, 1.0)),
        (Direction::Target, rule_threshold(Comparator::Eq, 1.0)),
    ];
    for (direction, rule) in agreeing {
        let objective = objective(direction, direction.requires_bound().then_some(1.0));
        assert_eq!(
            rule.check_against(&objective),
            Ok(()),
            "{direction} {rule:?}"
        );
    }
    let disagreeing = [
        (Direction::Higher, rule_threshold(Comparator::Le, 0.9)),
        (Direction::Higher, rule_threshold(Comparator::Eq, 0.9)),
        (Direction::Lower, rule_threshold(Comparator::Ge, 250.0)),
        (
            Direction::Lower,
            rule_baseline(Comparator::Gt, Baseline::BestSeen, None),
        ),
        (Direction::Zero, rule_threshold(Comparator::Le, 0.5)),
        (
            Direction::Zero,
            rule_baseline(Comparator::Le, Baseline::PriorCollection, None),
        ),
        (Direction::Zero, rule_threshold(Comparator::Lt, 0.0)),
    ];
    for (direction, rule) in disagreeing {
        let objective = objective(direction, None);
        assert_eq!(
            rule.check_against(&objective),
            Err(DecisionRuleError::DirectionMismatch {
                direction,
                comparator: rule.comparator(),
            }),
            "{direction} {rule:?}"
        );
    }

    let constant = rule_baseline(Comparator::Gt, Baseline::ConstantPredictor, Some(0.05));
    assert_eq!(constant.check_estimator(Estimator::Proportion), Ok(()));
    for estimator in [
        Estimator::Count,
        Estimator::Mean,
        Estimator::Median,
        Estimator::Ratio,
    ] {
        assert_eq!(
            constant.check_estimator(estimator),
            Err(DecisionRuleError::ConstantPredictorRequiresProportion { estimator })
        );
    }
    for estimator in Estimator::ALL {
        assert_eq!(
            rule_baseline(Comparator::Ge, Baseline::BestSeen, None).check_estimator(estimator),
            Ok(())
        );
        assert_eq!(
            rule_threshold(Comparator::Ge, 1.0).check_estimator(estimator),
            Ok(())
        );
    }
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
        &schema_enum(
            &schema,
            "/$defs/statistical_design/properties/estimator/enum",
        ),
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
        (
            Estimator::ALL.len(),
            Comparator::ALL.len(),
            Baseline::ALL.len()
        ),
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
            assert_eq!(
                comparator.holds(estimate, 1.0),
                holds,
                "{comparator} {estimate}"
            );
            assert_eq!(
                rule.holds(estimate, None),
                Ok(holds),
                "{comparator} {estimate}"
            );
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
    let tolerant = rule_baseline(Comparator::Ge, Baseline::BestSeen, Some(-0.25));
    assert_eq!(tolerant.holds(0.75, Some(1.0)), Ok(true));
    assert_eq!(tolerant.holds(0.5, Some(1.0)), Ok(false));
    // Lower is better: reference = baseline value - margin. Cutting a 200 ms
    // prior latency by at least 20 ms needs 180 ms or less.
    let faster = rule_baseline(Comparator::Le, Baseline::PriorCollection, Some(20.0));
    assert_eq!(faster.holds(180.0, Some(200.0)), Ok(true));
    assert_eq!(faster.holds(190.0, Some(200.0)), Ok(false));
    // ...and a negative margin lets it regress by up to 10 ms.
    let slack = rule_baseline(Comparator::Lt, Baseline::BestSeen, Some(-10.0));
    assert_eq!(slack.holds(205.0, Some(200.0)), Ok(true));
    assert_eq!(slack.holds(210.0, Some(200.0)), Ok(false));
    // eq has no margin: the reference is the baseline value itself.
    let unchanged = rule_baseline(Comparator::Eq, Baseline::PriorCollection, None);
    assert_eq!(unchanged.holds(3.0, Some(3.0)), Ok(true));
    assert_eq!(unchanged.holds(4.0, Some(3.0)), Ok(false));

    // A finite baseline value and margin can still overflow the reference.
    assert_eq!(
        rule_baseline(Comparator::Ge, Baseline::BestSeen, Some(f64::MAX))
            .holds(1.0, Some(f64::MAX)),
        Err(RuleEvaluationError::NonFiniteReference {
            reference: f64::INFINITY
        })
    );
    assert_eq!(
        rule_baseline(Comparator::Le, Baseline::BestSeen, Some(f64::MAX))
            .holds(1.0, Some(f64::MIN)),
        Err(RuleEvaluationError::NonFiniteReference {
            reference: f64::NEG_INFINITY
        })
    );
    // `Comparator::holds` itself does not guard NaN: every comparator is false.
    for comparator in Comparator::ALL {
        assert!(!comparator.holds(f64::NAN, 1.0), "{comparator}");
        assert!(!comparator.holds(1.0, f64::NAN), "{comparator}");
    }

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

/// One case table for the `protected_apparatus` item syntax (FR-024), the
/// same entries `tests/test_module.py` runs through the schema.
const APPARATUS_PATHS_ACCEPTED: [&str; 12] = [
    "tests/fixtures/labels.json",
    "evals/harness.py",
    "corpus/*.json",
    "corpus/**/*.yaml",
    "**/checker.toml",
    "**",
    "*",
    ".github/checker.toml",
    "labels/.answers",
    "a/...",
    "src/a*b*c.rs",
    "population select.sql",
];

/// Refused entries, each with the typed refusal the Rust type gives.
fn apparatus_paths_refused() -> Vec<(&'static str, ApparatusPathError)> {
    let path = |refused: &str| refused.to_owned();
    vec![
        ("", ApparatusPathError::Empty),
        (
            "/etc/passwd",
            ApparatusPathError::Absolute {
                path: path("/etc/passwd"),
            },
        ),
        (
            "a//b",
            ApparatusPathError::EmptySegment { path: path("a//b") },
        ),
        ("a/", ApparatusPathError::EmptySegment { path: path("a/") }),
        (
            "./a",
            ApparatusPathError::CurrentDirectorySegment { path: path("./a") },
        ),
        (
            "a/./b",
            ApparatusPathError::CurrentDirectorySegment {
                path: path("a/./b"),
            },
        ),
        (
            "../outside",
            ApparatusPathError::ParentDirectorySegment {
                path: path("../outside"),
            },
        ),
        (
            "a/../b",
            ApparatusPathError::ParentDirectorySegment {
                path: path("a/../b"),
            },
        ),
        (
            "a/..",
            ApparatusPathError::ParentDirectorySegment { path: path("a/..") },
        ),
        (
            "a\\b",
            ApparatusPathError::ForbiddenCharacter {
                path: path("a\\b"),
                character: '\\',
            },
        ),
        (
            "a/b?.json",
            ApparatusPathError::ForbiddenCharacter {
                path: path("a/b?.json"),
                character: '?',
            },
        ),
        (
            "a/[ab].json",
            ApparatusPathError::ForbiddenCharacter {
                path: path("a/[ab].json"),
                character: '[',
            },
        ),
        (
            "a/{x,y}.json",
            ApparatusPathError::ForbiddenCharacter {
                path: path("a/{x,y}.json"),
                character: '{',
            },
        ),
        (
            "C:/labels.json",
            ApparatusPathError::ForbiddenCharacter {
                path: path("C:/labels.json"),
                character: ':',
            },
        ),
        (
            "a**",
            ApparatusPathError::PartialDoubleStar { path: path("a**") },
        ),
        (
            "a/***/b",
            ApparatusPathError::PartialDoubleStar {
                path: path("a/***/b"),
            },
        ),
        (
            "**.json",
            ApparatusPathError::PartialDoubleStar {
                path: path("**.json"),
            },
        ),
        (
            "a\u{1}b",
            ApparatusPathError::ForbiddenCharacter {
                path: path("a\u{1}b"),
                character: '\u{1}',
            },
        ),
    ]
}

#[test]
#[trace("TC-156", "FR-024-AC-3")]
fn tc_156_apparatus_path_accepts_safe_relative_globs_and_refuses_the_rest() {
    let schema = measurement_plan_schema();
    let pattern = schema
        .pointer("/$defs/apparatus_path/pattern")
        .and_then(serde_json::Value::as_str)
        .expect("apparatus_path pattern");
    assert_eq!(
        schema["properties"]["protected_apparatus"]["items"]["$ref"], "#/$defs/apparatus_path",
        "protected_apparatus items must resolve to the pattern this test reads"
    );
    let schema_pattern = regex::Regex::new(pattern).expect("schema pattern compiles");

    for accepted in APPARATUS_PATHS_ACCEPTED {
        let path = ApparatusPath::new(accepted)
            .unwrap_or_else(|error| panic!("{accepted:?} must be accepted: {error}"));
        assert_eq!(path.as_str(), accepted);
        assert_eq!(path.is_glob(), accepted.contains('*'), "{accepted:?}");
        assert!(
            schema_pattern.is_match(accepted),
            "schema refuses {accepted:?}"
        );
        assert_eq!(accepted.parse::<ApparatusPath>(), Ok(path.clone()));
        let decoded: ApparatusPath =
            yaml_serde::from_str(&format!("{accepted:?}")).expect("path deserializes");
        assert_eq!(decoded, path);
    }
    for (refused, error) in apparatus_paths_refused() {
        assert_eq!(
            ApparatusPath::new(refused),
            Err(error.clone()),
            "{refused:?}"
        );
        assert!(
            !schema_pattern.is_match(refused),
            "schema accepts {refused:?}"
        );
        let decoded =
            serde_json::from_value::<ApparatusPath>(serde_json::Value::String(refused.to_owned()))
                .expect_err("deserialization refuses");
        assert!(
            decoded.to_string().contains(&error.to_string()),
            "{decoded}"
        );
    }
}

#[test]
#[trace("TC-156", "FR-024-AC-3")]
fn tc_156_protected_apparatus_is_a_non_empty_set_of_distinct_entries() {
    let paths = |entries: &[&str]| {
        entries
            .iter()
            .map(|entry| ApparatusPath::new(*entry).expect("valid path"))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ProtectedApparatus::new(Vec::new()),
        Err(ProtectedApparatusError::Empty)
    );
    assert_eq!(
        ProtectedApparatus::new(paths(&["b.json", "a.json", "b.json"])),
        Err(ProtectedApparatusError::Duplicate {
            path: ApparatusPath::new("b.json").expect("valid path")
        })
    );

    let built = apparatus(&["labels/*.json", "evals/harness.py"]);
    assert_eq!(built.len(), 2);
    assert!(!built.is_empty());
    assert!(built.contains(&ApparatusPath::new("labels/*.json").expect("valid path")));
    assert!(!built.contains(&ApparatusPath::new("labels/a.json").expect("valid path")));
    // A set: order does not matter, and it serializes sorted.
    assert_eq!(built, apparatus(&["evals/harness.py", "labels/*.json"]));
    assert_eq!(
        built.iter().map(ApparatusPath::as_str).collect::<Vec<_>>(),
        ["evals/harness.py", "labels/*.json"]
    );
    let emitted = yaml_serde::to_string(&built).expect("apparatus serializes");
    assert_eq!(emitted, "- evals/harness.py\n- labels/*.json\n");
    assert_eq!(
        yaml_serde::from_str::<ProtectedApparatus>(&emitted).expect("round trip"),
        built
    );

    for (refused, message) in [
        ("[]\n", ProtectedApparatusError::Empty.to_string()),
        (
            "- a.json\n- a.json\n",
            "protected_apparatus names `a.json` more than once".to_owned(),
        ),
        (
            "- ../a.json\n",
            "protected apparatus path `../a.json` has a `..` segment".to_owned(),
        ),
        ("evals/harness.py\n", "invalid type".to_owned()),
    ] {
        let error = yaml_serde::from_str::<ProtectedApparatus>(refused)
            .expect_err("must refuse")
            .to_string();
        assert!(error.contains(&message), "{refused:?}: {error}");
    }
    // YAML reads an unquoted `7` as a string where one is expected, so the
    // non-string refusal is checked on a JSON number.
    assert!(serde_json::from_value::<ProtectedApparatus>(serde_json::json!([7])).is_err());
}

#[test]
#[trace("TC-157", "FR-024-AC-4")]
fn tc_157_negative_controls_are_closed_non_empty_and_distinct() {
    let schema = measurement_plan_schema();
    assert_eq!(
        schema["properties"]["negative_controls"]["items"]["$ref"],
        "#/$defs/negative_control"
    );
    assert_wire_parity(
        &schema_enum(&schema, "/$defs/negative_control/properties/kind/enum"),
        &NegativeControlKind::ALL,
        |kind| kind.wire_name(),
    );
    for kind in NegativeControlKind::ALL {
        let expected_index = match kind {
            NegativeControlKind::SuppressedObservation => 0,
            NegativeControlKind::GainWithinNoise => 1,
            NegativeControlKind::StaleEvidence => 2,
            NegativeControlKind::ApparatusEdit => 3,
            NegativeControlKind::SelectiveReporting => 4,
        };
        assert_eq!(NegativeControlKind::ALL.get(expected_index), Some(&kind));
        assert_eq!(kind.to_string(), kind.wire_name());
    }
    assert_eq!(NegativeControlKind::ALL.len(), 5);
    assert_eq!(
        schema["allOf"][0]["then"]["required"],
        serde_json::json!(["ground_truth_kind", "negative_controls"]),
        "a gate-stage plan requires negative_controls"
    );

    let control = NegativeControl::new(NegativeControlKind::StaleEvidence, "subject version")
        .expect("valid control");
    assert_eq!(control.kind(), NegativeControlKind::StaleEvidence);
    assert_eq!(control.description(), "subject version");
    assert_eq!(
        NegativeControl::new(NegativeControlKind::GainWithinNoise, ""),
        Err(NegativeControlError::EmptyDescription {
            kind: NegativeControlKind::GainWithinNoise
        })
    );
    assert_eq!(
        NegativeControls::new(Vec::new()),
        Err(NegativeControlError::Empty)
    );
    assert_eq!(
        NegativeControls::new([control.clone(), control.clone()]),
        Err(NegativeControlError::Duplicate {
            kind: NegativeControlKind::StaleEvidence,
            description: "subject version".to_owned()
        })
    );
    let edit =
        NegativeControl::new(NegativeControlKind::ApparatusEdit, "digest").expect("valid control");
    let controls = NegativeControls::new([edit.clone(), control.clone()]).expect("valid list");
    assert_eq!(controls.as_slice(), [edit, control]);
    assert!(controls.covers(NegativeControlKind::ApparatusEdit));
    assert!(!controls.covers(NegativeControlKind::SelectiveReporting));

    let yaml = "- kind: apparatus-edit\n  description: digest\n- kind: stale-evidence\n  description: subject version\n";
    let decoded: NegativeControls = yaml_serde::from_str(yaml).expect("controls deserialize");
    assert_eq!(decoded, controls);
    assert_eq!(yaml_serde::to_string(&decoded).expect("serializes"), yaml);
    for refused in [
        "[]\n",
        "- kind: vibes\n  description: d\n",
        "- description: d\n",
        "- kind: stale-evidence\n",
        "- kind: stale-evidence\n  description: ''\n",
        "- kind: stale-evidence\n  description: d\n  severity: high\n",
        "- kind: stale-evidence\n  description: d\n- kind: stale-evidence\n  description: d\n",
        "- stale-evidence\n",
    ] {
        assert!(
            yaml_serde::from_str::<NegativeControls>(refused).is_err(),
            "must refuse {refused:?}"
        );
    }
}

#[test]
#[trace("TC-158", "FR-024-AC-5")]
fn tc_158_a_protected_apparatus_edit_without_a_version_bump_is_a_finding() {
    let base = MeasurementDefinition {
        objective: Some(objective(Direction::Higher, None)),
        estimator: Some(Estimator::Proportion),
        decision_rule: Some(rule_threshold(Comparator::Ge, 0.99)),
        protected_apparatus: Some(apparatus(&["evals/harness.py", "labels/*.json"])),
    };
    let with = |protected_apparatus| MeasurementDefinition {
        protected_apparatus,
        ..base.clone()
    };
    for edited in [
        // An entry added, one removed, one changed, and the list removed.
        with(Some(apparatus(&[
            "evals/harness.py",
            "labels/*.json",
            "evals/checker.toml",
        ]))),
        with(Some(apparatus(&["evals/harness.py"]))),
        with(Some(apparatus(&["evals/harness.py", "labels/**/*.json"]))),
        with(None),
    ] {
        assert_edit_is_reported_unless_versioned(
            &base,
            &edited,
            &[DefinitionMember::ProtectedApparatus],
        );
        assert_edit_is_reported_unless_versioned(
            &edited,
            &base,
            &[DefinitionMember::ProtectedApparatus],
        );
    }
    // Reordering the list is not a change.
    let reordered = with(Some(apparatus(&["labels/*.json", "evals/harness.py"])));
    let plan = |definition: &MeasurementDefinition| PlanDefinition {
        definition_version: Some("retention-v1"),
        definition: definition.clone(),
    };
    assert_eq!(
        definition_change_without_version_bump(&plan(&base), &plan(&reordered)),
        None
    );
    assert_eq!(
        DefinitionMember::ProtectedApparatus.to_string(),
        "protected_apparatus"
    );
}
