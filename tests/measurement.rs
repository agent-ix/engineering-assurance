// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-020 `MeasurementPlan` objective types, schema parity, the definition-change
//! check, and the narrow `measurement` feature; FR-021 decision-rule and
//! estimator vocabulary, its schema parity, and rule evaluation; FR-024
//! protected apparatus and negative controls, their schema parity, and
//! protected-list edits in the definition-change check; FR-026 objective
//! steering fields (`weight`, `value_half_life`, `budget`), their exclusion
//! from the measurement definition, and their absence from any decision-rule
//! verdict.

use std::{collections::BTreeSet, fs, path::Path, process::Command};

use engineering_assurance::measurement::{
    ApparatusPath, ApparatusPathError, Baseline, Comparator, ConfidenceLevel, ConfidenceLevelError,
    DecisionRule, DecisionRuleError, DefinitionChangedWithoutVersionBump, DefinitionMember,
    Direction, Estimator, Interval, IntervalError, MarginMode, MeasurementDefinition,
    NegativeControl, NegativeControlError, NegativeControlKind, NegativeControls, Objective,
    ObjectiveError, PlanDefinition, ProtectedApparatus, ProtectedApparatusError,
    RuleEvaluationError, RuleReference, definition_change_without_version_bump,
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

/// The reference of an absolute-margin baseline rule.
const fn absolute_baseline(baseline: Baseline, margin: f64) -> RuleReference {
    RuleReference::Baseline {
        baseline,
        margin,
        margin_mode: MarginMode::Absolute,
    }
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
        schema["properties"]["objective"]["$ref"], "#/definitions/objective",
        "objective must resolve to the definitions entry this test reads"
    );
    let schema_directions = schema["definitions"]["objective"]["properties"]["direction"]["enum"]
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
        schema["definitions"]["objective"]["allOf"][0]["then"]["required"],
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
        protected_apparatus: Some(apparatus(&["evals/harness.py", "labels/**"])),
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

#[test]
#[trace("TC-167", "FR-026-AC-2")]
fn tc_167_objective_steering_fields_construction_and_deserialization_are_validated() {
    let built = Objective::with_steering(
        Direction::Higher,
        Some(0.9),
        Some(2.0),
        Some(30.0),
        Some(50.0),
    )
    .expect("valid steering fields");
    assert_eq!(built.weight(), Some(2.0));
    assert_eq!(built.value_half_life(), Some(30.0));
    assert_eq!(built.budget(), Some(50.0));

    // Zero weight and zero budget are valid edge cases: an objective may
    // carry no relative value and no attempt budget (PLAT-967).
    let zeroed = Objective::with_steering(
        Direction::Higher,
        Some(0.9),
        Some(0.0),
        Some(1.0),
        Some(0.0),
    )
    .expect("weight and budget may be zero");
    assert_eq!(zeroed.weight(), Some(0.0));
    assert_eq!(zeroed.budget(), Some(0.0));

    assert_eq!(
        Objective::with_steering(Direction::Higher, None, Some(-1.0), None, None),
        Err(ObjectiveError::NegativeWeight { weight: -1.0 })
    );
    assert!(matches!(
        Objective::with_steering(Direction::Higher, None, Some(f64::NAN), None, None),
        Err(ObjectiveError::NonFiniteWeight { weight }) if weight.is_nan()
    ));
    assert_eq!(
        Objective::with_steering(Direction::Higher, None, None, Some(f64::INFINITY), None),
        Err(ObjectiveError::NonFiniteValueHalfLife {
            value_half_life: f64::INFINITY
        })
    );
    assert_eq!(
        Objective::with_steering(Direction::Higher, None, None, Some(0.0), None),
        Err(ObjectiveError::NonPositiveValueHalfLife {
            value_half_life: 0.0
        })
    );
    assert_eq!(
        Objective::with_steering(Direction::Higher, None, None, Some(-5.0), None),
        Err(ObjectiveError::NonPositiveValueHalfLife {
            value_half_life: -5.0
        })
    );
    assert_eq!(
        Objective::with_steering(Direction::Higher, None, None, None, Some(-0.01)),
        Err(ObjectiveError::NegativeBudget { budget: -0.01 })
    );
    assert!(matches!(
        Objective::with_steering(Direction::Higher, None, None, None, Some(f64::NAN)),
        Err(ObjectiveError::NonFiniteBudget { budget }) if budget.is_nan()
    ));

    assert_eq!(
        parse("direction: higher\nbound: 0.9\nweight: 2\nvalue_half_life: 30\nbudget: 50\n"),
        Ok(built)
    );
    let negative_weight =
        parse("direction: higher\nweight: -1\n").expect_err("weight must be non-negative");
    assert!(
        negative_weight.contains(&ObjectiveError::NegativeWeight { weight: -1.0 }.to_string()),
        "{negative_weight}"
    );
    let zero_half_life =
        parse("direction: higher\nvalue_half_life: 0\n").expect_err("half-life must be positive");
    assert!(
        zero_half_life.contains(
            &ObjectiveError::NonPositiveValueHalfLife {
                value_half_life: 0.0
            }
            .to_string()
        ),
        "{zero_half_life}"
    );
    let negative_budget =
        parse("direction: higher\nbudget: -5\n").expect_err("budget must be non-negative");
    assert!(
        negative_budget.contains(&ObjectiveError::NegativeBudget { budget: -5.0 }.to_string()),
        "{negative_budget}"
    );
    assert!(
        parse("direction: higher\nweight: \"heavy\"\n").is_err(),
        "weight must be numeric"
    );
    assert!(
        parse("direction: higher\nbudget: [1]\n").is_err(),
        "budget must be numeric"
    );

    let emitted = yaml_serde::to_string(&built).expect("objective serializes");
    assert_eq!(parse(&emitted), Ok(built));
}

#[test]
#[trace("TC-167", "FR-026-AC-2")]
fn tc_167_non_finite_and_non_positive_steering_fields_are_refused_on_deserialization() {
    // Every non-finite refusal, and a negative half-life, also holds through deserialization, where a
    // YAML `.inf`/`.nan` passes the schema's `type: number` (FR-026).
    for (yaml, refusal) in [
        (
            "direction: higher\nweight: .inf\n",
            ObjectiveError::NonFiniteWeight {
                weight: f64::INFINITY,
            }
            .to_string(),
        ),
        (
            "direction: higher\nvalue_half_life: -.inf\n",
            ObjectiveError::NonFiniteValueHalfLife {
                value_half_life: f64::NEG_INFINITY,
            }
            .to_string(),
        ),
        (
            "direction: higher\nvalue_half_life: -2\n",
            ObjectiveError::NonPositiveValueHalfLife {
                value_half_life: -2.0,
            }
            .to_string(),
        ),
        (
            "direction: higher\nbudget: .inf\n",
            ObjectiveError::NonFiniteBudget {
                budget: f64::INFINITY,
            }
            .to_string(),
        ),
    ] {
        let refused = parse(yaml).expect_err(yaml);
        assert!(refused.contains(&refusal), "{yaml}: {refused}");
    }
    for field in ["weight", "value_half_life", "budget"] {
        let yaml = format!("direction: higher\n{field}: .nan\n");
        let refused = parse(&yaml).expect_err(&yaml);
        assert!(
            refused.contains("is not a finite number"),
            "{yaml}: {refused}"
        );
    }
}

#[test]
#[trace("TC-168", "FR-026-AC-3")]
fn tc_168_objective_steering_fields_are_excluded_from_the_measurement_definition() {
    let base_objective = objective(Direction::Higher, Some(0.995));
    let with_steering = Objective::with_steering(
        Direction::Higher,
        Some(0.995),
        Some(1.0),
        Some(30.0),
        Some(50.0),
    )
    .expect("valid steering fields");
    let base = MeasurementDefinition {
        objective: Some(base_objective),
        estimator: Some(Estimator::Proportion),
        decision_rule: Some(rule_threshold(Comparator::Ge, 0.99)),
        protected_apparatus: None,
    };
    let steered = MeasurementDefinition {
        objective: Some(with_steering),
        ..base.clone()
    };

    // All three steering fields differ between `base` and `steered`;
    // `direction`/`bound` do not. Neither direction of the edit is a
    // definition change.
    assert_eq!(
        base.changed_members(&steered),
        Vec::<DefinitionMember>::new()
    );
    assert_eq!(
        steered.changed_members(&base),
        Vec::<DefinitionMember>::new()
    );

    let plan = |version, definition: &MeasurementDefinition| PlanDefinition {
        definition_version: version,
        definition: definition.clone(),
    };
    assert_eq!(
        definition_change_without_version_bump(
            &plan(Some("retention-v1"), &base),
            &plan(Some("retention-v1"), &steered)
        ),
        None,
        "a steering-only edit must not require a definition_version bump"
    );

    // A `bound` change alongside unchanged steering fields is still
    // reported, so the exclusion is specific to the steering fields.
    let raised = Objective::with_steering(
        Direction::Higher,
        Some(0.999),
        Some(1.0),
        Some(30.0),
        Some(50.0),
    )
    .expect("valid steering fields");
    let raised_definition = MeasurementDefinition {
        objective: Some(raised),
        ..steered.clone()
    };
    assert_eq!(
        steered.changed_members(&raised_definition),
        vec![DefinitionMember::Objective]
    );
}

/// A test-local `MeasurementPlan` frontmatter shape carrying only the
/// `objective` block and `statistical_design.decision_rule`, so TC-169 parses
/// both from one document the way a consumer would, rather than parsing the
/// rule in isolation.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SteeringProbePlan {
    objective: Objective,
    statistical_design: SteeringProbeDesign,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SteeringProbeDesign {
    decision_rule: DecisionRule,
}

fn steering_probe_plan(steering: &str, rule: &str) -> SteeringProbePlan {
    let frontmatter = format!(
        "objective:\n  direction: higher\n  bound: 0.995\n{steering}statistical_design:\n  decision_rule:\n{rule}"
    );
    yaml_serde::from_str(&frontmatter).unwrap_or_else(|error| panic!("{frontmatter}: {error}"))
}

/// One `holds` probe: estimate, baseline value, expected verdict.
type VerdictProbe = (f64, Option<f64>, bool);

/// Assert that `steered` -- the same frontmatter as `plain` plus steering
/// fields -- parses to the same rule and definitional objective as `plain`,
/// agrees on direction/comparator consistency, and reaches every expected
/// `holds` verdict in `probes`.
fn assert_steering_is_inert(
    label: &str,
    plain: &SteeringProbePlan,
    steered: &SteeringProbePlan,
    probes: &[VerdictProbe],
) {
    assert_eq!(
        plain.objective.definitional(),
        steered.objective.definitional(),
        "{label}"
    );
    let plain_rule = &plain.statistical_design.decision_rule;
    let steered_rule = &steered.statistical_design.decision_rule;
    assert_eq!(
        plain_rule, steered_rule,
        "{label}: steering fields must not change the parsed rule"
    );
    assert_eq!(
        plain_rule.check_against(&plain.objective),
        steered_rule.check_against(&steered.objective),
        "{label}: direction/comparator agreement must not depend on steering"
    );
    for &(estimate, baseline_value, expected) in probes {
        let verdict = steered_rule
            .holds(estimate, baseline_value)
            .expect("finite estimate and baseline");
        assert_eq!(
            verdict, expected,
            "{label}: estimate {estimate} against baseline {baseline_value:?}"
        );
    }
}

#[test]
#[trace("TC-169", "FR-026-AC-4")]
fn tc_169_steering_fields_never_change_a_decision_rules_verdict() {
    // Three otherwise-identical frontmatter documents per rule: one with no
    // steering fields, one at the smallest accepted magnitudes (weight 0, the
    // smallest positive f64 half-life, budget 0), and one at the largest
    // finite magnitudes. Each is parsed independently, and every one must
    // yield the same decision rule, the same direction/comparator agreement,
    // and the same `holds` verdict on every probed estimate, including the
    // exact boundary. Values the fields refuse (negative, zero half-life,
    // non-finite) never reach this point: TC-167 asserts their refusal.
    let steering_variants = [
        ("none", ""),
        (
            "smallest",
            "  weight: 0\n  value_half_life: 5e-324\n  budget: 0\n",
        ),
        (
            "largest",
            "  weight: 1.7976931348623157e308\n  value_half_life: 1.7976931348623157e308\n  budget: 1.7976931348623157e308\n",
        ),
    ];
    let rules: [(&str, &[VerdictProbe]); 2] = [
        (
            "    comparator: ge\n    threshold: 0.99\n",
            &[
                (0.999, None, true),
                (0.99, None, true), // exactly at the threshold: `ge` holds
                (0.989_999, None, false),
                (0.0, None, false),
            ],
        ),
        (
            "    comparator: gt\n    baseline: prior-collection\n    margin: 0.5\n",
            &[
                (1.75, Some(1.0), true),
                (1.5, Some(1.0), false), // exactly baseline + margin: `gt` fails
                (0.25, Some(1.0), false),
            ],
        ),
    ];

    for (rule, probes) in rules {
        let plain = steering_probe_plan("", rule);
        assert_eq!(plain.objective.weight(), None);
        for (label, steering) in steering_variants {
            let steered = steering_probe_plan(steering, rule);
            if !steering.is_empty() {
                assert_ne!(
                    plain.objective, steered.objective,
                    "{label}: the fixture must actually carry steering fields"
                );
                assert!(steered.objective.weight().is_some(), "{label}");
                assert!(steered.objective.value_half_life().is_some(), "{label}");
                assert!(steered.objective.budget().is_some(), "{label}");
            }
            assert_steering_is_inert(label, &plain, &steered, probes);
        }
    }
    assert_eq!(
        steering_probe_plan(steering_variants[1].1, rules[0].0)
            .objective
            .value_half_life(),
        Some(5e-324),
        "the smallest positive half-life must survive parsing unrounded"
    );
}

/// Quoin's measurement intake imports these types with
/// `default-features = false, features = ["measurement"]`; like
/// `tc_138_a_minimal_downstream_compiles_only_the_source_audit_feature`, this
/// proves that consumer never resolves `serde_json`, so it cannot inherit the
/// `arbitrary_precision` flip `full` carries.
#[test]
#[trace("TC-142", "FR-020-AC-4", "FR-021-AC-7", "FR-024-AC-7", "FR-026-AC-6")]
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
        "use engineering_assurance::measurement::{ApparatusPath, Baseline, Comparator, DecisionRule, Direction, Estimator, NegativeControl, NegativeControlKind, NegativeControls, Objective, ProtectedApparatus, definition_change_without_version_bump};\nfn main(){let steered = Objective::with_steering(Direction::Target, Some(1.0), Some(1.0), Some(30.0), Some(50.0)).expect(\"valid\"); let _ = (steered.weight(), steered.value_half_life(), steered.budget(), steered.definitional(), definition_change_without_version_bump, DecisionRule::against_baseline(Comparator::Gt, Baseline::ConstantPredictor, None), Estimator::Proportion, ApparatusPath::new(\"evals/**\").map(|path| ProtectedApparatus::new([path])), NegativeControl::new(NegativeControlKind::ApparatusEdit, \"digest\").map(|control| NegativeControls::new([control])));}\n",
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
        absolute_baseline(Baseline::ConstantPredictor, 0.05)
    );
    assert_eq!(
        DecisionRule::against_baseline(Comparator::Ge, Baseline::BestSeen, None)
            .expect("valid rule")
            .reference(),
        absolute_baseline(Baseline::BestSeen, 0.0)
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
        schema["definitions"]["statistical_design"]["properties"]["decision_rule"]["$ref"],
        "#/definitions/decision_rule",
        "decision_rule must resolve to the definitions entry this test reads"
    );
    assert_wire_parity(
        &schema_enum(
            &schema,
            "/definitions/statistical_design/properties/estimator/enum",
        ),
        &Estimator::ALL,
        |estimator| estimator.wire_name(),
    );
    assert_wire_parity(
        &schema_enum(
            &schema,
            "/definitions/decision_rule/properties/comparator/enum",
        ),
        &Comparator::ALL,
        |comparator| comparator.wire_name(),
    );
    assert_wire_parity(
        &schema_enum(
            &schema,
            "/definitions/decision_rule/properties/baseline/enum",
        ),
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
            Baseline::ExternalReference => 3,
        };
        assert_eq!(Baseline::ALL.get(index), Some(&baseline));
    }
    assert_eq!(
        (
            Estimator::ALL.len(),
            Comparator::ALL.len(),
            Baseline::ALL.len()
        ),
        (5, 5, 4)
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

#[test]
#[trace("TC-171", "FR-021-AC-10")]
fn tc_171_external_reference_baseline_has_no_estimator_restriction_and_allows_eq() {
    // Unlike `constant-predictor`, `external-reference` is accepted with
    // every estimator: it is a checker-resolved value from outside the plan
    // (PLAT-1009), not a corpus computation.
    for estimator in Estimator::ALL {
        assert_eq!(
            rule_baseline(Comparator::Ge, Baseline::ExternalReference, None)
                .check_estimator(estimator),
            Ok(())
        );
    }
    // Unlike `best-seen`, `external-reference` is accepted with `eq`: its
    // value does not depend on the comparator's direction.
    let equal = DecisionRule::against_baseline(Comparator::Eq, Baseline::ExternalReference, None)
        .expect("eq against external-reference is valid");
    assert_eq!(equal.holds(30.0, Some(30.0)), Ok(true));
    assert_eq!(equal.holds(31.0, Some(30.0)), Ok(false));
    // `eq` still takes no margin, exactly as for every other baseline.
    assert_eq!(
        DecisionRule::against_baseline(Comparator::Eq, Baseline::ExternalReference, Some(0.1)),
        Err(DecisionRuleError::MarginWithEq)
    );
    assert!(
        parse_rule("comparator: eq\nbaseline: external-reference\nmargin: 0.1\n")
            .expect_err("eq with a margin is refused")
            .contains(&DecisionRuleError::MarginWithEq.to_string())
    );

    // The direction check applies unchanged: a lower-is-better budget takes
    // `le`, and refuses `ge` with a typed error naming both.
    let budget = rule_baseline(Comparator::Le, Baseline::ExternalReference, Some(5.0));
    assert_eq!(
        budget.check_against(&objective(Direction::Lower, None)),
        Ok(())
    );
    assert_eq!(
        rule_baseline(Comparator::Ge, Baseline::ExternalReference, None)
            .check_against(&objective(Direction::Lower, None)),
        Err(DecisionRuleError::DirectionMismatch {
            direction: Direction::Lower,
            comparator: Comparator::Ge,
        })
    );

    // Evaluation: the checker must supply the resolved value, which the
    // margin moves in the direction of improvement (30 - 5 = 25 for `le`).
    assert_eq!(
        budget.holds(25.0, None),
        Err(RuleEvaluationError::MissingBaselineValue {
            baseline: Baseline::ExternalReference
        })
    );
    assert_eq!(budget.holds(25.0, Some(30.0)), Ok(true));
    assert_eq!(budget.holds(26.0, Some(30.0)), Ok(false));

    // Construction, serialization, deserialization, and round trip.
    let rule =
        DecisionRule::against_baseline(Comparator::Le, Baseline::ExternalReference, Some(5.0))
            .expect("valid rule");
    assert_eq!(
        rule.reference(),
        absolute_baseline(Baseline::ExternalReference, 5.0)
    );
    assert_eq!(
        parse_rule("comparator: le\nbaseline: external-reference\nmargin: 5\n"),
        Ok(rule)
    );
    let emitted = yaml_serde::to_string(&rule).expect("rule serializes");
    assert_eq!(parse_rule(&emitted), Ok(rule), "{emitted}");
    assert_eq!(
        Baseline::ExternalReference.to_string(),
        "external-reference"
    );
    assert_eq!(
        yaml_serde::from_str::<Baseline>("external-reference").expect("valid baseline"),
        Baseline::ExternalReference
    );
}

/// One case from `tests/fixtures/apparatus-paths.json`, the shared
/// `protected_apparatus` entry table `tests/test_module.py` also reads.
#[derive(serde::Deserialize)]
struct ApparatusCase {
    path: String,
    /// For an accepted case: the directory a directory entry names.
    #[serde(default)]
    directory: Option<String>,
    /// For a refused case: the `ApparatusPathError` variant name.
    #[serde(default)]
    error: Option<String>,
    /// For a `ForbiddenCharacter` case: the character reported.
    #[serde(default)]
    character: Option<char>,
}

#[derive(serde::Deserialize)]
struct ApparatusCases {
    accepted: Vec<ApparatusCase>,
    refused: Vec<ApparatusCase>,
}

fn apparatus_cases() -> ApparatusCases {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/apparatus-paths.json");
    serde_json::from_slice(&fs::read(&fixture).expect("fixture readable"))
        .expect("fixture is the case-table shape")
}

/// The typed refusal a refused case names.
fn expected_refusal(case: &ApparatusCase) -> ApparatusPathError {
    let path = case.path.clone();
    let variant = case.error.as_deref().expect("refused case names its error");
    match variant {
        "Empty" => ApparatusPathError::Empty,
        "Absolute" => ApparatusPathError::Absolute { path },
        "ForbiddenCharacter" => ApparatusPathError::ForbiddenCharacter {
            path,
            character: case.character.expect("ForbiddenCharacter case names it"),
        },
        "WholeRepository" => ApparatusPathError::WholeRepository,
        "MisplacedWildcard" => ApparatusPathError::MisplacedWildcard { path },
        "EmptySegment" => ApparatusPathError::EmptySegment { path },
        "CurrentDirectorySegment" => ApparatusPathError::CurrentDirectorySegment { path },
        "ParentDirectorySegment" => ApparatusPathError::ParentDirectorySegment { path },
        other => panic!("fixture names unknown ApparatusPathError variant {other:?}"),
    }
}

#[test]
#[trace("TC-156", "FR-024-AC-3")]
fn tc_156_apparatus_path_accepts_and_refuses_the_shared_case_table() {
    let schema = measurement_plan_schema();
    assert_eq!(
        schema["properties"]["protected_apparatus"]["items"]["$ref"],
        "#/definitions/apparatus_path",
        "protected_apparatus items must resolve to the pattern this test reads"
    );
    let pattern = |pointer: &str| {
        let source = schema
            .pointer(pointer)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("schema pattern at {pointer}"));
        regex::Regex::new(source).expect("schema pattern compiles")
    };
    let entry_pattern = pattern("/definitions/apparatus_path/pattern");
    let control_pattern = pattern("/definitions/apparatus_path/not/pattern");
    let schema_accepts =
        |path: &str| entry_pattern.is_match(path) && !control_pattern.is_match(path);

    let cases = apparatus_cases();
    assert!(!cases.accepted.is_empty() && !cases.refused.is_empty());
    let mut variants = BTreeSet::new();
    for case in &cases.accepted {
        let accepted = case.path.as_str();
        let path = ApparatusPath::new(accepted)
            .unwrap_or_else(|error| panic!("{accepted:?} must be accepted: {error}"));
        assert_eq!(path.as_str(), accepted);
        assert_eq!(path.directory(), case.directory.as_deref(), "{accepted:?}");
        assert!(schema_accepts(accepted), "schema refuses {accepted:?}");
        assert_eq!(accepted.parse::<ApparatusPath>(), Ok(path.clone()));
        let decoded: ApparatusPath =
            serde_json::from_value(serde_json::Value::String(accepted.to_owned()))
                .expect("path deserializes");
        assert_eq!(decoded, path);
    }
    for case in &cases.refused {
        let refused = case.path.as_str();
        let error = expected_refusal(case);
        variants.insert(case.error.clone().expect("named"));
        assert_eq!(
            ApparatusPath::new(refused),
            Err(error.clone()),
            "{refused:?}"
        );
        assert!(!schema_accepts(refused), "schema accepts {refused:?}");
        let decoded =
            serde_json::from_value::<ApparatusPath>(serde_json::Value::String(refused.to_owned()))
                .expect_err("deserialization refuses");
        assert!(
            decoded.to_string().contains(&error.to_string()),
            "{decoded}"
        );
    }
    // The table exercises every refusal the type has.
    assert_eq!(
        variants,
        [
            "Absolute",
            "CurrentDirectorySegment",
            "Empty",
            "EmptySegment",
            "ForbiddenCharacter",
            "MisplacedWildcard",
            "ParentDirectorySegment",
            "WholeRepository",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
    );
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

    let built = apparatus(&["labels/**", "evals/harness.py"]);
    assert_eq!(built.len(), 2);
    assert!(!built.is_empty());
    assert!(built.contains(&ApparatusPath::new("labels/**").expect("valid path")));
    assert!(!built.contains(&ApparatusPath::new("labels/a.json").expect("valid path")));
    // A set: order does not matter, and it serializes sorted.
    assert_eq!(built, apparatus(&["evals/harness.py", "labels/**"]));
    assert_eq!(
        built.iter().map(ApparatusPath::as_str).collect::<Vec<_>>(),
        ["evals/harness.py", "labels/**"]
    );
    let emitted = yaml_serde::to_string(&built).expect("apparatus serializes");
    assert_eq!(emitted, "- evals/harness.py\n- labels/**\n");
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
        "#/definitions/negative_control"
    );
    assert_wire_parity(
        &schema_enum(
            &schema,
            "/definitions/negative_control/properties/kind/enum",
        ),
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
        schema["allOf"][1]["then"]["required"],
        serde_json::json!([
            "ground_truth_kind",
            "negative_controls",
            "protected_apparatus"
        ]),
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
        protected_apparatus: Some(apparatus(&["evals/harness.py", "labels/**"])),
    };
    let with = |protected_apparatus| MeasurementDefinition {
        protected_apparatus,
        ..base.clone()
    };
    for edited in [
        // An entry added, one removed, one changed, and the list removed.
        with(Some(apparatus(&[
            "evals/harness.py",
            "labels/**",
            "evals/checker.toml",
        ]))),
        with(Some(apparatus(&["evals/harness.py"]))),
        with(Some(apparatus(&["evals/harness.py", "labels/answers/**"]))),
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
    let reordered = with(Some(apparatus(&["labels/**", "evals/harness.py"])));
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

#[trace("TC-188", "FR-021-AC-12")]
#[test]
fn tc_188_a_relative_margin_is_a_fraction_of_the_baseline_value() {
    // One rule, two benchmarks whose baselines differ by three orders of
    // magnitude: "no worse than 5% slower" is 5% of each baseline.
    let rule = DecisionRule::against_baseline_with_mode(
        Comparator::Le,
        Baseline::PriorCollection,
        Some(-0.05),
        MarginMode::Relative,
    )
    .expect("valid rule");
    for (baseline, tolerated, refused) in [(100.0, 105.0, 105.1), (100_000.0, 105_000.0, 105_100.0)]
    {
        assert_eq!(rule.holds(tolerated, Some(baseline)), Ok(true));
        assert_eq!(rule.holds(refused, Some(baseline)), Ok(false));
    }
    // The scale is the baseline's magnitude, so a negative baseline does not
    // flip the direction of the tolerance.
    assert_eq!(rule.holds(-95.0, Some(-100.0)), Ok(true));
    assert_eq!(rule.holds(-94.9, Some(-100.0)), Ok(false));

    // The same margin stated absolutely is a fixed number of units.
    let absolute =
        DecisionRule::against_baseline(Comparator::Le, Baseline::PriorCollection, Some(-0.05))
            .expect("valid rule");
    assert_eq!(absolute.holds(100.04, Some(100.0)), Ok(true));
    assert_eq!(absolute.holds(105.0, Some(100.0)), Ok(false));
}

#[trace("TC-188", "FR-021-AC-12")]
#[test]
fn tc_188_margin_mode_is_closed_defaults_to_absolute_and_round_trips() {
    let relative = parse_rule(
        "comparator: le\nbaseline: prior-collection\nmargin: -0.05\nmargin_mode: relative\n",
    )
    .expect("a relative margin parses");
    assert_eq!(
        relative.reference(),
        RuleReference::Baseline {
            baseline: Baseline::PriorCollection,
            margin: -0.05,
            margin_mode: MarginMode::Relative
        }
    );
    let encoded = yaml_serde::to_string(&relative).expect("rule serializes");
    assert_eq!(parse_rule(&encoded), Ok(relative), "round trip: {encoded}");

    // Absent means absolute, and an absolute rule never emits the key, so a
    // plan written before this field existed serializes unchanged.
    let legacy = parse_rule("comparator: gt\nbaseline: best-seen\nmargin: 1\n").expect("parses");
    assert_eq!(
        legacy.reference(),
        RuleReference::Baseline {
            baseline: Baseline::BestSeen,
            margin: 1.0,
            margin_mode: MarginMode::Absolute
        }
    );
    assert!(
        !yaml_serde::to_string(&legacy)
            .expect("rule serializes")
            .contains("margin_mode")
    );

    // A mode with no margin, or on a threshold, has nothing to apply to.
    for yaml in [
        "comparator: le\nbaseline: prior-collection\nmargin_mode: relative\n",
        "comparator: le\nthreshold: 3\nmargin_mode: relative\n",
    ] {
        assert!(
            yaml_serde::from_str::<DecisionRule>(yaml)
                .expect_err("a mode without a margin is refused")
                .to_string()
                .contains("margin_mode"),
            "{yaml}"
        );
    }
    assert_eq!(
        DecisionRule::against_baseline_with_mode(
            Comparator::Le,
            Baseline::PriorCollection,
            None,
            MarginMode::Relative
        ),
        Err(DecisionRuleError::MarginModeWithoutMargin)
    );
    assert!(
        parse_rule("comparator: le\nbaseline: prior-collection\nmargin: 1\nmargin_mode: percent\n")
            .is_err()
    );
}

#[trace("TC-188", "FR-021-AC-12")]
#[test]
fn tc_188_schema_margin_mode_enum_equals_the_rust_wire_names() {
    let schema = measurement_plan_schema();
    assert_wire_parity(
        &schema_enum(
            &schema,
            "/definitions/decision_rule/properties/margin_mode/enum",
        ),
        &MarginMode::ALL,
        |mode| mode.wire_name(),
    );
    assert_eq!(
        schema["definitions"]["decision_rule"]["dependencies"]["margin_mode"],
        serde_json::json!(["margin"]),
        "the schema must refuse a margin_mode with no margin, as the Rust rule does"
    );
}

#[trace("TC-188", "FR-021-AC-12")]
#[test]
fn tc_188_a_zero_margin_is_canonical_and_round_trips_in_either_mode() {
    let relative_zero = DecisionRule::against_baseline_with_mode(
        Comparator::Ge,
        Baseline::PriorCollection,
        Some(0.0),
        MarginMode::Relative,
    )
    .expect("valid rule");
    let absolute_zero =
        DecisionRule::against_baseline(Comparator::Ge, Baseline::PriorCollection, Some(0.0))
            .expect("valid rule");
    assert_eq!(relative_zero, absolute_zero);
    let encoded = yaml_serde::to_string(&relative_zero).expect("rule serializes");
    assert_eq!(
        parse_rule(&encoded),
        Ok(relative_zero),
        "round trip: {encoded}"
    );
}

#[trace("TC-188", "FR-021-AC-12")]
#[test]
fn tc_188_a_relative_margin_against_a_zero_baseline_shrinks_to_nothing() {
    let rule = DecisionRule::against_baseline_with_mode(
        Comparator::Ge,
        Baseline::PriorCollection,
        Some(0.05),
        MarginMode::Relative,
    )
    .expect("valid rule");
    // "5% better" than zero is zero: the rule demands no improvement.
    assert_eq!(rule.holds(0.0, Some(0.0)), Ok(true));
    assert_eq!(rule.holds(-0.001, Some(0.0)), Ok(false));
}

fn level(value: f64) -> ConfidenceLevel {
    ConfidenceLevel::new(value).expect("valid confidence level")
}

fn interval(lower: f64, upper: f64, at: f64) -> Interval {
    Interval::new(lower, upper, level(at), "bootstrap, 1000 resamples").expect("valid interval")
}

fn interval_rule(rule: DecisionRule, at: f64) -> DecisionRule {
    rule.with_interval_level(level(at))
        .expect("valid interval rule")
}

#[trace("TC-192", "FR-021-AC-13")]
#[test]
fn tc_192_interval_level_is_validated_optional_and_round_trips() {
    let rule = parse_rule("comparator: ge\nthreshold: 0.9\ninterval_level: 0.95\n")
        .expect("a threshold rule with a level parses");
    assert_eq!(rule.interval_level(), Some(level(0.95)));
    let encoded = yaml_serde::to_string(&rule).expect("rule serializes");
    assert_eq!(parse_rule(&encoded), Ok(rule), "round trip: {encoded}");

    let baseline = parse_rule(
        "comparator: le\nbaseline: prior-collection\nmargin: -0.05\nmargin_mode: relative\ninterval_level: 0.9\n",
    )
    .expect("a baseline rule with a level parses");
    assert_eq!(baseline.interval_level(), Some(level(0.9)));
    let encoded = yaml_serde::to_string(&baseline).expect("rule serializes");
    assert_eq!(parse_rule(&encoded), Ok(baseline), "round trip: {encoded}");

    // Absent means the point estimate, and no key is emitted.
    let point = parse_rule("comparator: gt\nthreshold: 1\n").expect("parses");
    assert_eq!(point.interval_level(), None);
    assert!(
        !yaml_serde::to_string(&point)
            .expect("rule serializes")
            .contains("interval_level")
    );

    // Every comparator but eq takes a level.
    for comparator in [
        Comparator::Gt,
        Comparator::Ge,
        Comparator::Lt,
        Comparator::Le,
    ] {
        assert!(
            rule_threshold(comparator, 1.0)
                .with_interval_level(level(0.5))
                .is_ok()
        );
    }
    assert_eq!(
        rule_threshold(Comparator::Eq, 1.0).with_interval_level(level(0.5)),
        Err(DecisionRuleError::IntervalLevelWithEq)
    );
    assert!(
        parse_rule("comparator: eq\nbaseline: prior-collection\ninterval_level: 0.9\n")
            .expect_err("eq refuses a level")
            .contains("interval_level")
    );

    for bad in ["0", "1", "-0.5", "1.5", ".inf", ".nan"] {
        let yaml = format!("comparator: ge\nthreshold: 1\ninterval_level: {bad}\n");
        assert!(
            parse_rule(&yaml)
                .expect_err("an out-of-range level is refused")
                .contains("interval_level"),
            "{bad}"
        );
    }
    assert!(parse_rule("comparator: ge\nthreshold: 1\ninterval_level: high\n").is_err());
    for bad in [0.0, 1.0, -0.1, 1.1, f64::NAN, f64::INFINITY] {
        assert!(
            matches!(ConfidenceLevel::new(bad), Err(ConfidenceLevelError { .. })),
            "{bad}"
        );
    }
}

#[trace("TC-192", "FR-021-AC-13")]
#[test]
fn tc_192_interval_validates_and_round_trips() {
    let value = interval(0.8, 0.9, 0.95);
    assert_eq!(
        (value.lower(), value.upper(), value.level(), value.method()),
        (0.8, 0.9, level(0.95), "bootstrap, 1000 resamples")
    );
    let encoded = yaml_serde::to_string(&value).expect("interval serializes");
    assert_eq!(
        yaml_serde::from_str::<Interval>(&encoded).expect("round trip"),
        value,
        "{encoded}"
    );
    // A degenerate interval is a valid interval.
    assert!(Interval::new(1.0, 1.0, level(0.5), "exact count").is_ok());

    assert_eq!(
        Interval::new(1.0, 0.0, level(0.5), "m"),
        Err(IntervalError::InvertedBounds {
            lower: 1.0,
            upper: 0.0
        })
    );
    for (lower, upper) in [
        (f64::NAN, 1.0),
        (0.0, f64::INFINITY),
        (f64::NEG_INFINITY, 0.0),
    ] {
        assert!(matches!(
            Interval::new(lower, upper, level(0.5), "m"),
            Err(IntervalError::NonFiniteBound { .. })
        ));
    }
    for method in ["", "  \t"] {
        assert_eq!(
            Interval::new(0.0, 1.0, level(0.5), method),
            Err(IntervalError::EmptyMethod)
        );
    }
    for yaml in [
        "lower: 0\nupper: 1\nlevel: 1\nmethod: m\n",
        "lower: 0\nupper: 1\nlevel: 0\nmethod: m\n",
        "lower: 1\nupper: 0\nlevel: 0.9\nmethod: m\n",
        "lower: 0\nupper: 1\nlevel: 0.9\nmethod: ''\n",
        "lower: 0\nupper: 1\nlevel: 0.9\n",
        "lower: 0\nupper: 1\nlevel: 0.9\nmethod: m\nextra: 1\n",
    ] {
        assert!(yaml_serde::from_str::<Interval>(yaml).is_err(), "{yaml}");
    }
    assert!(matches!(
        yaml_serde::from_str::<Interval>("lower: 0\nupper: 1\nlevel: 1\nmethod: m\n")
            .expect_err("level 1 refused")
            .to_string()
            .as_str(),
        message if message.contains("level")
    ));
}

#[trace("TC-192", "FR-021-AC-13")]
#[test]
fn tc_192_schema_interval_level_is_an_open_unit_interval_forbidden_with_eq() {
    let schema = measurement_plan_schema();
    let rule = &schema["definitions"]["decision_rule"];
    let property = &rule["properties"]["interval_level"];
    assert_eq!(property["type"], "number");
    assert_eq!(property["exclusiveMinimum"], 0);
    assert_eq!(property["exclusiveMaximum"], 1);
    assert_eq!(
        rule["allOf"][0]["then"]["properties"]["interval_level"],
        serde_json::json!(false),
        "the schema must refuse interval_level with eq, as the Rust rule does"
    );
    assert!(
        rule["dependencies"].get("interval_level").is_none(),
        "interval_level stands alone: a threshold or baseline rule may state it"
    );
}

#[trace("TC-192", "FR-021-AC-13", "FR-021-AC-8")]
#[test]
fn tc_192_editing_interval_level_without_a_version_bump_is_a_definition_finding() {
    let definition = |decision_rule| MeasurementDefinition {
        decision_rule: Some(decision_rule),
        ..MeasurementDefinition::default()
    };
    let plan = |version, definition| PlanDefinition {
        definition_version: version,
        definition,
    };
    let point = rule_threshold(Comparator::Ge, 0.9);
    let at_95 = interval_rule(point, 0.95);
    let at_99 = interval_rule(point, 0.99);

    for (before, after) in [(point, at_95), (at_95, at_99), (at_95, point)] {
        let finding = definition_change_without_version_bump(
            &plan(Some("v1"), definition(before)),
            &plan(Some("v1"), definition(after)),
        )
        .expect("an interval_level edit is a finding");
        assert_eq!(finding.changed, vec![DefinitionMember::DecisionRule]);
        assert_eq!(
            definition_change_without_version_bump(
                &plan(Some("v1"), definition(before)),
                &plan(Some("v2"), definition(after)),
            ),
            None
        );
    }
    assert_eq!(
        definition_change_without_version_bump(
            &plan(Some("v1"), definition(at_95)),
            &plan(Some("v1"), definition(at_95)),
        ),
        None
    );
}

#[trace("TC-193", "FR-021-AC-14")]
#[test]
fn tc_193_a_rule_holds_at_the_unfavourable_bound_of_the_interval() {
    // Higher is better: the lower bound decides.
    let ge = interval_rule(rule_threshold(Comparator::Ge, 0.9), 0.95);
    let gt = interval_rule(rule_threshold(Comparator::Gt, 0.9), 0.95);
    let passes = interval(0.91, 0.97, 0.95);
    let straddles = interval(0.85, 0.99, 0.95);
    assert_eq!(ge.holds_on_interval(0.95, Some(&passes), None), Ok(true));
    // The point estimate clears 0.9 but its interval cannot rule out noise.
    assert_eq!(
        ge.holds_on_interval(0.95, Some(&straddles), None),
        Ok(false)
    );
    assert_eq!(
        ge.holds(0.95, None),
        Err(RuleEvaluationError::IntervalRequired {
            required: level(0.95)
        })
    );
    // Boundary: `ge` holds at equality, `gt` does not.
    let edge = interval(0.9, 0.97, 0.95);
    assert_eq!(ge.holds_on_interval(0.95, Some(&edge), None), Ok(true));
    assert_eq!(gt.holds_on_interval(0.95, Some(&edge), None), Ok(false));

    // Lower is better: the upper bound decides.
    let le = interval_rule(rule_threshold(Comparator::Le, 100.0), 0.9);
    let lt = interval_rule(rule_threshold(Comparator::Lt, 100.0), 0.9);
    assert_eq!(
        le.holds_on_interval(90.0, Some(&interval(85.0, 99.0, 0.9)), None),
        Ok(true)
    );
    assert_eq!(
        le.holds_on_interval(90.0, Some(&interval(85.0, 105.0, 0.9)), None),
        Ok(false)
    );
    assert_eq!(
        le.holds_on_interval(90.0, Some(&interval(85.0, 100.0, 0.9)), None),
        Ok(true)
    );
    assert_eq!(
        lt.holds_on_interval(90.0, Some(&interval(85.0, 100.0, 0.9)), None),
        Ok(false)
    );

    // A baseline reference, absolute and relative, resolves as in `holds`.
    let ratchet = interval_rule(
        rule_baseline(Comparator::Ge, Baseline::PriorCollection, Some(0.02)),
        0.9,
    );
    assert_eq!(
        ratchet.holds_on_interval(0.8, Some(&interval(0.73, 0.85, 0.9)), Some(0.7)),
        Ok(true)
    );
    assert_eq!(
        ratchet.holds_on_interval(0.8, Some(&interval(0.71, 0.85, 0.9)), Some(0.7)),
        Ok(false)
    );
    let relative = interval_rule(
        DecisionRule::against_baseline_with_mode(
            Comparator::Le,
            Baseline::PriorCollection,
            Some(-0.05),
            MarginMode::Relative,
        )
        .expect("valid rule"),
        0.9,
    );
    assert_eq!(
        relative.holds_on_interval(100.0, Some(&interval(98.0, 104.0, 0.9)), Some(100.0)),
        Ok(true)
    );
    assert_eq!(
        relative.holds_on_interval(100.0, Some(&interval(98.0, 106.0, 0.9)), Some(100.0)),
        Ok(false)
    );
    assert_eq!(
        ratchet.holds_on_interval(0.8, Some(&interval(0.7, 0.9, 0.9)), None),
        Err(RuleEvaluationError::MissingBaselineValue {
            baseline: Baseline::PriorCollection
        })
    );
    assert_eq!(
        ge.holds_on_interval(0.95, Some(&passes), Some(1.0)),
        Err(RuleEvaluationError::UnexpectedBaselineValue)
    );
}

#[trace("TC-193", "FR-021-AC-14")]
#[test]
fn tc_193_interval_evaluation_refuses_what_it_cannot_judge() {
    let ge = interval_rule(rule_threshold(Comparator::Ge, 0.9), 0.95);
    assert_eq!(
        ge.holds_on_interval(0.95, None, None),
        Err(RuleEvaluationError::MissingInterval {
            required: level(0.95)
        })
    );
    assert_eq!(
        ge.holds_on_interval(0.95, Some(&interval(0.91, 0.97, 0.9)), None),
        Err(RuleEvaluationError::IntervalLevelTooLow {
            required: level(0.95),
            observed: level(0.9)
        })
    );
    // An equal or higher level is accepted.
    for at in [0.95, 0.99] {
        assert_eq!(
            ge.holds_on_interval(0.95, Some(&interval(0.91, 0.97, at)), None),
            Ok(true)
        );
    }
    for outside in [0.9, 0.98] {
        assert_eq!(
            ge.holds_on_interval(outside, Some(&interval(0.91, 0.97, 0.95)), None),
            Err(RuleEvaluationError::EstimateOutsideInterval {
                estimate: outside,
                lower: 0.91,
                upper: 0.97
            })
        );
    }
    for estimate in [f64::NAN, f64::INFINITY] {
        assert!(matches!(
            ge.holds_on_interval(estimate, Some(&interval(0.91, 0.97, 0.95)), None),
            Err(RuleEvaluationError::NonFiniteEstimate { .. })
        ));
    }
    let overflow = interval_rule(
        rule_baseline(Comparator::Ge, Baseline::PriorCollection, Some(f64::MAX)),
        0.9,
    );
    assert!(matches!(
        overflow.holds_on_interval(1.0, Some(&interval(0.0, 2.0, 0.9)), Some(f64::MAX)),
        Err(RuleEvaluationError::NonFiniteReference { .. })
    ));
    assert!(matches!(
        overflow.holds_on_interval(1.0, Some(&interval(0.0, 2.0, 0.9)), Some(f64::NAN)),
        Err(RuleEvaluationError::NonFiniteBaselineValue { .. })
    ));
    // A point rule takes no interval, and an eq rule is a point rule.
    let point = rule_threshold(Comparator::Ge, 0.9);
    assert_eq!(
        point.holds_on_interval(0.95, Some(&interval(0.91, 0.97, 0.95)), None),
        Err(RuleEvaluationError::IntervalNotRequested)
    );
    assert_eq!(point.holds(0.95, None), Ok(true));
}
