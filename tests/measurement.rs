// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-020 `MeasurementPlan` objective types, schema parity, the definition-change
//! check, and the narrow `measurement` feature.

use std::{collections::BTreeSet, fs, path::Path, process::Command};

use engineering_assurance::measurement::{
    Direction, Objective, ObjectiveChangedWithoutVersionBump, ObjectiveError, PlanDefinition,
    objective_change_without_version_bump,
};
use ix_trace_rs::trace;

fn objective(direction: Direction, bound: Option<f64>) -> Objective {
    Objective::new(direction, bound).expect("valid objective")
}

fn parse(yaml: &str) -> Result<Objective, String> {
    yaml_serde::from_str::<Objective>(yaml).map_err(|error| error.to_string())
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
    assert!(not_a_number.contains("is not a finite number"), "{not_a_number}");
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
        let decoded: Direction =
            serde_json::from_value(serde_json::Value::String(wire_name.clone()))
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
#[trace("TC-142", "FR-020-AC-4")]
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
        "use engineering_assurance::measurement::{Direction, Objective, objective_change_without_version_bump};\nfn main(){let _ = (Objective::new(Direction::Target, Some(1.0)), objective_change_without_version_bump);}\n",
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
