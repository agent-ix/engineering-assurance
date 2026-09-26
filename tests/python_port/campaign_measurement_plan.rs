//! The Campaign candidate's procedure path is an exact, safe plan input.
use super::common::{package_root, read, schema, yaml};
use ix_trace_rs::trace;
use serde_json::{Value, json};

fn validator() -> jsonschema::Validator {
    let schema = schema("measurement-plan-frontmatter.schema");
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(&schema)
        .expect("schema builds")
}

fn plan() -> Value {
    let skeleton = read(&package_root().join("skeletons/MeasurementPlan.md"));
    let front = skeleton
        .split("---")
        .nth(1)
        .expect("frontmatter")
        .to_string();
    yaml(&front)
}

fn errors(value: &Value) -> Vec<String> {
    validator()
        .iter_errors(value)
        .map(|e| e.to_string())
        .collect()
}

#[test]
#[trace("TC-185", "FR-024-AC-10")]
fn procedure_path_is_optional_and_safe_json_files_are_valid() {
    jsonschema::meta::validate(&schema("measurement-plan-frontmatter.schema"))
        .expect("schema is a valid draft 2020-12 schema");
    let mut candidate = plan();
    assert!(errors(&candidate).is_empty());
    for path in [
        "campaign/procedures/probe.json",
        "campaign/.procedures/probe.JSON",
        "campaign/procedures/.probe.json",
    ] {
        candidate["execution_procedure"] = json!(path);
        let apparatus = candidate["protected_apparatus"]
            .as_array_mut()
            .expect("protected_apparatus array");
        apparatus.push(json!(path));
        let found = errors(&candidate);
        assert!(found.is_empty(), "{path}: {found:?}");
        candidate["protected_apparatus"]
            .as_array_mut()
            .expect("array")
            .pop();
    }
}

#[test]
#[trace("TC-185", "FR-024-AC-10")]
fn procedure_path_refuses_unsafe_or_non_json_values() {
    let cases: Vec<Value> = [
        "",
        "../escape.json",
        "/absolute/probe.json",
        "C:\\probe.json",
        "campaign//probe.json",
        "campaign/./probe.json",
        "campaign/../probe.json",
        "campaign/procedures/**",
        "campaign/procedure.yaml",
        "campaign/procedure.json/",
        "campaign\\procedure.json",
        "campaign/procedure.json\n",
        ".json",
        "campaign/.json",
    ]
    .iter()
    .map(|s| json!(s))
    .chain([json!(42)])
    .collect();
    for path in cases {
        let mut candidate = plan();
        candidate["execution_procedure"] = path.clone();
        assert!(!errors(&candidate).is_empty(), "{path}");
    }
}

/// Quoin checks exact path coverage.
#[test]
#[trace("TC-185", "FR-024-AC-10")]
fn procedure_path_requires_protected_apparatus_declaration() {
    let mut candidate = plan();
    candidate["execution_procedure"] = json!("campaign/procedures/probe.json");
    candidate
        .as_object_mut()
        .expect("object")
        .remove("protected_apparatus");
    assert!(!errors(&candidate).is_empty());
}
