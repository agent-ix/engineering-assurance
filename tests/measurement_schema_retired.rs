// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Retired `MeasurementPlan` compatibility stays separate from active plans.

use std::{fs, path::PathBuf};

use ix_trace_rs::trace;
use serde_json::{Value, json};

fn validator() -> jsonschema::Validator {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("engineering_assurance/schemas/measurement-plan-frontmatter.schema.json");
    let schema: Value = serde_json::from_slice(&fs::read(path).expect("packaged schema exists"))
        .expect("packaged schema is JSON");
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .build(&schema)
        .expect("packaged schema compiles")
}

fn legacy() -> Value {
    json!({
        "id": "MP-002",
        "title": "Historical corpus census",
        "type": "MeasurementPlan",
        "status": "retired",
        "owner": "corpus-owner",
        "metric": "tl-mltl.corpus-coverage",
        "definition_version": "tl-mltl.corpus-coverage/v1",
        "stage": "gate",
        "statistical_design": {
            "population": "every cell in the closed catalog",
            "sampling": "complete deterministic enumeration",
            "repetitions": 1,
            "estimator": "covered applicable cells divided by all applicable cells",
            "error_model": "omitted or duplicated cells",
            "uncertainty": "no sampling interval",
            "decision_rule": "fail when an applicable cell lacks a canonical fixture"
        },
        "relationships": []
    })
}

#[trace("TC-172", "FR-021-AC-11", "FR-024-AC-9")]
#[test]
fn retired_prose_is_readable_but_cannot_become_an_active_gate() {
    let validator = validator();
    let old = legacy();
    assert!(validator.is_valid(&old), "unaltered retired v0.2.1 shape");

    for status in ["active", "proposed"] {
        let mut changed = old.clone();
        changed["status"] = json!(status);
        assert!(!validator.is_valid(&changed), "{status} cannot use prose");
    }

    for field in [
        "objective",
        "ground_truth_kind",
        "protected_apparatus",
        "negative_controls",
    ] {
        let mut changed = old.clone();
        changed[field] = json!({});
        assert!(
            !validator.is_valid(&changed),
            "legacy cannot mix in {field}"
        );
    }

    let mut malformed = old.clone();
    malformed["statistical_design"]["repetitions"] = json!(0);
    assert!(!validator.is_valid(&malformed));
    malformed = old.clone();
    malformed["statistical_design"]["decision_rule"] = json!("");
    assert!(!validator.is_valid(&malformed));
    malformed = old.clone();
    malformed["statistical_design"]["undeclared"] = json!(true);
    assert!(!validator.is_valid(&malformed));

    let mut current = old;
    current["statistical_design"]["estimator"] = json!("count");
    current["statistical_design"]["decision_rule"] = json!({"comparator": "ge", "threshold": 1});
    current["ground_truth_kind"] = json!("mechanical");
    current["protected_apparatus"] = json!(["evals/harness.py"]);
    current["negative_controls"] = json!([{
        "kind": "suppressed-observation",
        "description": "missing cells lower the reported population"
    }]);
    assert!(validator.is_valid(&current), "retired current-shape plan");
    current["status"] = json!("active");
    assert!(validator.is_valid(&current), "complete active gate");
    for field in [
        "ground_truth_kind",
        "protected_apparatus",
        "negative_controls",
    ] {
        let mut incomplete = current.clone();
        incomplete
            .as_object_mut()
            .expect("plan is object")
            .remove(field);
        assert!(
            !validator.is_valid(&incomplete),
            "active gate requires {field}"
        );
    }
}
