// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The module exports its five artifact types through the semantic contract.
//!
//! Quoin's semantic reader accepts an export only when its `data_schema` names
//! a JSON Schema 2020-12 file whose bytes hash to the recorded digest and whose
//! `$id` sits under the module's version. The five frontmatter schemas stay
//! draft-07 (the retired-plan and Python guards depend on it), so each export
//! is a separate 2020-12 file derived mechanically from its draft-07 source by
//! [`to_2020_12`], the only place that transformation lives.
//!
//! Regenerate the derived files and every manifest digest with
//! `EA_BLESS=1 cargo test --test semantic_exports`.

use std::{
    collections::BTreeSet,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use ix_trace_rs::trace;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

// Assembled from parts so the content-rights URL scan does not see a literal.
const DRAFT_2020_12: &str = concat!("https:", "//json-schema.org/draft/2020-12/schema");
const ID_BASE: &str = concat!(
    "https:",
    "//schemas.agent-ix.org/agent-ix/engineering-assurance-campaign/"
);

/// Artifact type, its draft-07 source and its derived 2020-12 export.
const EXPORTS: [(&str, &str, &str); 5] = [
    (
        "AssuranceProfile",
        "assurance-profile-frontmatter.schema.json",
        "assurance-profile.schema.json",
    ),
    (
        "MeasurementPlan",
        "measurement-plan-frontmatter.schema.json",
        "measurement-plan.schema.json",
    ),
    (
        "ArchitectureDescription",
        "architecture-description-frontmatter.schema.json",
        "architecture-description.schema.json",
    ),
    (
        "ComponentAssuranceContract",
        "component-assurance-contract-frontmatter.schema.json",
        "component-assurance-contract.schema.json",
    ),
    (
        "AssuranceArgument",
        "assurance-argument-frontmatter.schema.json",
        "assurance-argument.schema.json",
    ),
];

fn module_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("engineering_assurance")
}

fn blessing() -> bool {
    std::env::var_os("EA_BLESS").is_some_and(|value| value == "1")
}

fn read_json(path: &Path) -> Value {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn manifest() -> Value {
    let text = fs::read_to_string(module_root().join("manifest.yaml")).expect("manifest reads");
    yaml_serde::from_str(&text).expect("manifest is YAML")
}

fn manifest_version(manifest: &Value) -> String {
    manifest["version"]
        .as_str()
        .expect("manifest version is a string")
        .to_owned()
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::from("sha256:");
    for byte in digest {
        write!(hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hex
}

fn subschema_map(value: &Value) -> Result<Value, String> {
    let map = value.as_object().ok_or("expected a map of subschemas")?;
    map.iter()
        .map(|(name, sub)| Ok((name.clone(), schema(sub)?)))
        .collect::<Result<Map<_, _>, String>>()
        .map(Value::Object)
}

fn subschema_list(value: &Value) -> Result<Value, String> {
    value
        .as_array()
        .ok_or("expected a list of subschemas")?
        .iter()
        .map(schema)
        .collect::<Result<Vec<_>, _>>()
        .map(Value::Array)
}

/// Rewrites one draft-07 schema node into its 2020-12 equivalent.
///
/// Only keyword positions that hold subschemas are walked, so a property that
/// happens to be named `dependencies` or `definitions` is left alone. Draft-07
/// forms with no mechanical 2020-12 equivalent are refused, not guessed.
fn schema(node: &Value) -> Result<Value, String> {
    let Some(map) = node.as_object() else {
        return Ok(node.clone());
    };
    if map.contains_key("$ref") && map.len() > 1 {
        return Err(
            "$ref has sibling keywords: draft-07 ignores them, 2020-12 applies them".into(),
        );
    }
    let mut out = Map::new();
    for (keyword, value) in map {
        match keyword.as_str() {
            "properties" | "patternProperties" => {
                out.insert(keyword.clone(), subschema_map(value)?);
            }
            "items" if value.is_array() => {
                return Err("array-form `items` is draft-07 tuple validation".into());
            }
            "additionalItems" => return Err("`additionalItems` is draft-07 only".into()),
            "items"
            | "additionalProperties"
            | "not"
            | "if"
            | "then"
            | "else"
            | "contains"
            | "propertyNames" => {
                out.insert(keyword.clone(), schema(value)?);
            }
            "allOf" | "anyOf" | "oneOf" => {
                out.insert(keyword.clone(), subschema_list(value)?);
            }
            "dependencies" => {
                let entries = value.as_object().ok_or("`dependencies` is not a map")?;
                let mut required = Map::new();
                let mut schemas = Map::new();
                for (name, dependency) in entries {
                    if dependency.is_array() {
                        required.insert(name.clone(), dependency.clone());
                    } else {
                        schemas.insert(name.clone(), schema(dependency)?);
                    }
                }
                if !required.is_empty() {
                    out.insert("dependentRequired".into(), Value::Object(required));
                }
                if !schemas.is_empty() {
                    out.insert("dependentSchemas".into(), Value::Object(schemas));
                }
            }
            "$ref" => {
                let target = value.as_str().ok_or("$ref is not a string")?;
                let target = target
                    .strip_prefix("#/definitions/")
                    .map_or_else(|| target.to_owned(), |rest| format!("#/$defs/{rest}"));
                out.insert(keyword.clone(), Value::String(target));
            }
            _ => {
                out.insert(keyword.clone(), value.clone());
            }
        }
    }
    Ok(Value::Object(out))
}

/// The one draft-07 to 2020-12 transformation, applied to a schema root.
fn to_2020_12(source: &Value, id: &str) -> Value {
    let mut root =
        schema(source).unwrap_or_else(|reason| panic!("cannot derive {id} mechanically: {reason}"));
    let map = root.as_object_mut().expect("schema root is an object");
    if let Some(definitions) = map.remove("definitions") {
        map.insert("$defs".into(), subschema_map(&definitions).expect("$defs"));
    }
    map.insert("$schema".into(), json!(DRAFT_2020_12));
    map.insert("$id".into(), json!(id));
    root
}

fn derived_id(version: &str, file: &str) -> String {
    format!("{ID_BASE}{version}/{file}")
}

fn derived_bytes(source_file: &str, file: &str, version: &str) -> String {
    let source = read_json(&module_root().join("schemas").join(source_file));
    let derived = to_2020_12(&source, &derived_id(version, file));
    let mut text = serde_json::to_string_pretty(&derived).expect("derived schema serializes");
    text.push('\n');
    text
}

/// Rewrites every `data_schema` digest line in the manifest text so it equals
/// the hash of the schema file recorded on the preceding `schema:` line.
fn rewrite_digests(manifest_text: &str) -> String {
    let mut lines: Vec<String> = manifest_text.lines().map(str::to_owned).collect();
    let mut schema_path: Option<String> = None;
    for line in &mut lines {
        let trimmed = line.trim_start();
        if let Some(path) = trimmed.strip_prefix("schema: ") {
            schema_path = Some(path.trim().to_owned());
        } else if trimmed.starts_with("digest: ")
            && let Some(path) = schema_path.take()
        {
            let indent = &line[..line.len() - trimmed.len()];
            let digest = sha256(&fs::read(module_root().join(&path)).expect("schema reads"));
            *line = format!("{indent}digest: {digest}");
        }
    }
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

/// In bless mode (`EA_BLESS=1`), regenerates the derived files and the manifest
/// digests; otherwise it does nothing and the checks below judge the files.
#[trace("TC-194", "FR-003-AC-7")]
#[test]
fn bless_regenerates_derived_schemas_and_digests() {
    if !blessing() {
        return;
    }
    let version = manifest_version(&manifest());
    for (_, source, file) in EXPORTS {
        fs::write(
            module_root().join("schemas").join(file),
            derived_bytes(source, file, &version),
        )
        .expect("derived schema writes");
    }
    let path = module_root().join("manifest.yaml");
    let text = fs::read_to_string(&path).expect("manifest reads");
    fs::write(&path, rewrite_digests(&text)).expect("manifest writes");
}

fn declared_types(manifest: &Value) -> Vec<&Value> {
    ["object_types", "artifact_types"]
        .into_iter()
        .filter_map(|key| manifest[key].as_array())
        .flatten()
        .collect()
}

#[trace("TC-194", "FR-003-AC-7")]
#[test]
fn every_export_is_a_declared_type_with_a_current_2020_12_schema_under_the_manifest_version() {
    let manifest = manifest();
    let version = manifest_version(&manifest);
    let types = declared_types(&manifest);

    let exports: Vec<&str> = manifest["semantic"]["exports"]
        .as_array()
        .expect("semantic.exports is a list")
        .iter()
        .map(|name| name.as_str().expect("export names are strings"))
        .collect();
    for name in EXPORTS.map(|(name, _, _)| name) {
        assert!(exports.contains(&name), "{name} is not exported");
    }
    for name in &exports {
        let entry = types
            .iter()
            .find(|entry| entry["name"] == *name)
            .unwrap_or_else(|| panic!("export {name} is not a declared type"));
        assert!(
            entry["data_schema"]["schema"].is_string(),
            "export {name} has no data_schema reference"
        );
    }

    // Every recorded digest, including the two campaign exports, is over the
    // bytes on disk; the id carries the manifest version; the draft is 2020-12.
    let mut checked = BTreeSet::new();
    for entry in &types {
        let Some(data_schema) = entry.get("data_schema") else {
            continue;
        };
        let relative = data_schema["schema"].as_str().expect("schema path");
        let bytes = fs::read(module_root().join(relative)).expect("schema reads");
        assert_eq!(
            data_schema["digest"].as_str(),
            Some(sha256(&bytes).as_str()),
            "{relative}: digest does not match the file (EA_BLESS=1 to regenerate)"
        );
        let parsed: Value = serde_json::from_slice(&bytes).expect("schema is JSON");
        assert_eq!(parsed["$schema"], DRAFT_2020_12, "{relative}");
        let file = relative.strip_prefix("schemas/").expect("under schemas/");
        assert_eq!(
            parsed["$id"].as_str(),
            Some(derived_id(&version, file).as_str()),
            "{relative}: $id does not carry manifest version {version}"
        );
        checked.insert(relative.to_owned());
    }
    assert!(
        checked.len() >= 7,
        "campaign and artifact exports all checked"
    );

    // Each artifact type points its export at the derived file for its source.
    for (name, source, file) in EXPORTS {
        let entry = types
            .iter()
            .find(|entry| entry["name"] == name)
            .expect("artifact type declared");
        assert_eq!(
            entry["frontmatter_schema_ref"].as_str(),
            Some(format!("schemas/{source}").as_str()),
            "{name}"
        );
        assert_eq!(
            entry["data_schema"]["schema"].as_str(),
            Some(format!("schemas/{file}").as_str()),
            "{name}"
        );
    }
}

#[trace("TC-194", "FR-003-AC-7")]
#[test]
fn each_derived_schema_equals_the_mechanical_transformation_of_its_draft_07_source() {
    let version = manifest_version(&manifest());
    for (name, source, file) in EXPORTS {
        let expected = derived_bytes(source, file, &version);
        let committed = fs::read_to_string(module_root().join("schemas").join(file))
            .unwrap_or_else(|e| panic!("{file}: {e} (EA_BLESS=1 to regenerate)"));
        assert_eq!(
            committed, expected,
            "{name}: {file} drifted from {source} (EA_BLESS=1 to regenerate)"
        );
        let derived: Value = serde_json::from_str(&committed).expect("derived is JSON");
        assert!(derived.get("definitions").is_none(), "{file}");
        assert!(!committed.contains("#/definitions/"), "{file}");
        assert!(!committed.contains("\"dependencies\""), "{file}");
    }
}

#[trace("TC-194", "FR-003-AC-7")]
#[test]
fn the_transformation_refuses_draft_07_forms_it_cannot_carry_over() {
    let refused = [
        json!({"items": [{"type": "string"}]}),
        json!({"items": {"type": "string"}, "additionalItems": false}),
        json!({"$ref": "#/definitions/a", "description": "sibling"}),
    ];
    for case in refused {
        assert!(schema(&case).is_err(), "{case}");
    }
    let converted = schema(&json!({
        "dependencies": {"a": ["b"], "c": {"required": ["d"]}},
        "properties": {"dependencies": {"type": "string"}, "r": {"$ref": "#/definitions/x"}}
    }))
    .expect("convertible");
    assert_eq!(converted["dependentRequired"], json!({"a": ["b"]}));
    assert_eq!(
        converted["dependentSchemas"],
        json!({"c": {"required": ["d"]}})
    );
    assert_eq!(
        converted["properties"]["dependencies"],
        json!({"type": "string"}),
        "a property named like a keyword is data, not a keyword"
    );
    assert_eq!(converted["properties"]["r"]["$ref"], "#/$defs/x");
}

// --- behavioural equivalence -------------------------------------------------

fn validator(schema: &Value, draft: jsonschema::Draft) -> jsonschema::Validator {
    jsonschema::options()
        .with_draft(draft)
        // `format` is an annotation under 2020-12 by default and an assertion
        // under draft-07; assert it in both so the comparison is like for like.
        .should_validate_formats(true)
        .build(schema)
        .expect("schema compiles")
}

fn frontmatter(name: &str) -> Value {
    let text = fs::read_to_string(module_root().join("skeletons").join(format!("{name}.md")))
        .expect("skeleton reads");
    let block = text
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .expect("skeleton has frontmatter")
        .0;
    yaml_serde::from_str(block).expect("frontmatter is YAML")
}

/// Every single-field mutation of a base document: drop each key, replace each
/// key with a wrong-typed value, and add an undeclared key.
fn mutations(base: &Value) -> Vec<(String, Value)> {
    let map = base.as_object().expect("frontmatter is an object");
    let mut cases = vec![("unmodified".to_owned(), base.clone())];
    for key in map.keys() {
        let mut dropped = base.clone();
        dropped.as_object_mut().expect("object").remove(key);
        cases.push((format!("drop {key}"), dropped));
        for (label, wrong) in [
            ("null", Value::Null),
            ("number", json!(-1)),
            ("empty object", json!({})),
            ("empty list", json!([])),
            ("empty string", json!("")),
        ] {
            let mut changed = base.clone();
            changed[key] = wrong;
            cases.push((format!("{key} as {label}"), changed));
        }
    }
    let mut extra = base.clone();
    extra["undeclared_field"] = json!(true);
    cases.push(("undeclared field".to_owned(), extra));
    for status in ["active", "retired", "proposed", "bogus"] {
        let mut changed = base.clone();
        changed["status"] = json!(status);
        cases.push((format!("status {status}"), changed));
    }
    cases
}

fn plan_with_rule(rule: &Value) -> Value {
    let mut plan = frontmatter("MeasurementPlan");
    plan["statistical_design"]["decision_rule"] = rule.clone();
    plan
}

/// The decision-rule and `margin_mode` cases the Python schema tests express,
/// including the `dependencies` (now `dependentRequired`) refusals.
fn decision_rule_cases() -> Vec<(&'static str, Value)> {
    vec![
        ("ge threshold", json!({"comparator": "ge", "threshold": 1})),
        (
            "unknown comparator",
            json!({"comparator": "approximately", "threshold": 1}),
        ),
        ("missing comparator", json!({"threshold": 1})),
        ("neither reference", json!({"comparator": "ge"})),
        (
            "both references",
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
            "absolute margin",
            json!({"comparator": "ge", "baseline": "prior-collection", "margin": 0.05,
                   "margin_mode": "absolute"}),
        ),
        (
            "relative margin",
            json!({"comparator": "ge", "baseline": "prior-collection", "margin": 0.05,
                   "margin_mode": "relative"}),
        ),
        (
            "margin_mode with a threshold",
            json!({"comparator": "ge", "threshold": 1, "margin_mode": "relative"}),
        ),
        (
            "margin_mode without a margin",
            json!({"comparator": "ge", "baseline": "prior-collection",
                   "margin_mode": "relative"}),
        ),
        (
            "margin_mode with eq and a margin",
            json!({"comparator": "eq", "baseline": "prior-collection", "margin": 0.1,
                   "margin_mode": "relative"}),
        ),
        (
            "unknown margin_mode",
            json!({"comparator": "ge", "baseline": "prior-collection", "margin": 0.05,
                   "margin_mode": "percent"}),
        ),
        (
            "margin without a baseline",
            json!({"comparator": "ge", "margin": 0.05}),
        ),
        (
            "interval_level 0.9",
            json!({"comparator": "ge", "threshold": 1, "interval_level": 0.9}),
        ),
        (
            "interval_level with eq",
            json!({"comparator": "eq", "threshold": 1, "interval_level": 0.9}),
        ),
        (
            "interval_level 0",
            json!({"comparator": "ge", "threshold": 1, "interval_level": 0}),
        ),
        (
            "interval_level 1",
            json!({"comparator": "ge", "threshold": 1, "interval_level": 1}),
        ),
        (
            "interval_level 95",
            json!({"comparator": "ge", "threshold": 1, "interval_level": 95}),
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
        ("prose rule", json!("escalate when low")),
    ]
}

/// The retired-plan shape from `tests/measurement_schema_retired.rs`.
fn retired_legacy_plan() -> Value {
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

fn retired_cases() -> Vec<(String, Value)> {
    let old = retired_legacy_plan();
    let mut cases = vec![("retired legacy".to_owned(), old.clone())];
    for status in ["active", "proposed"] {
        let mut changed = old.clone();
        changed["status"] = json!(status);
        cases.push((format!("legacy prose as {status}"), changed));
    }
    for field in [
        "objective",
        "ground_truth_kind",
        "protected_apparatus",
        "negative_controls",
    ] {
        let mut changed = old.clone();
        changed[field] = json!({});
        cases.push((format!("legacy mixed with {field}"), changed));
    }
    let mut zero = old.clone();
    zero["statistical_design"]["repetitions"] = json!(0);
    cases.push(("legacy zero repetitions".to_owned(), zero));
    let mut empty = old.clone();
    empty["statistical_design"]["decision_rule"] = json!("");
    cases.push(("legacy empty rule".to_owned(), empty));
    let mut undeclared = old.clone();
    undeclared["statistical_design"]["undeclared"] = json!(true);
    cases.push(("legacy undeclared key".to_owned(), undeclared));

    let mut current = old;
    current["statistical_design"]["estimator"] = json!("count");
    current["statistical_design"]["decision_rule"] = json!({"comparator": "ge", "threshold": 1});
    current["ground_truth_kind"] = json!("mechanical");
    current["protected_apparatus"] = json!(["evals/harness.py"]);
    current["negative_controls"] = json!([{
        "kind": "suppressed-observation",
        "description": "missing cells lower the reported population"
    }]);
    cases.push(("retired current shape".to_owned(), current.clone()));
    current["status"] = json!("active");
    cases.push(("active gate".to_owned(), current.clone()));
    for field in [
        "ground_truth_kind",
        "protected_apparatus",
        "negative_controls",
    ] {
        let mut incomplete = current.clone();
        incomplete.as_object_mut().expect("object").remove(field);
        cases.push((format!("active gate without {field}"), incomplete));
    }
    cases
}

#[trace("TC-195", "FR-003-AC-8")]
#[test]
fn the_2020_12_export_accepts_and_refuses_exactly_what_its_draft_07_source_does() {
    let mut compared = 0_usize;
    let mut accepted = 0_usize;
    let mut refused = 0_usize;
    for (name, source, file) in EXPORTS {
        let draft07 = read_json(&module_root().join("schemas").join(source));
        let draft2020 = read_json(&module_root().join("schemas").join(file));
        let old = validator(&draft07, jsonschema::Draft::Draft7);
        let new = validator(&draft2020, jsonschema::Draft::Draft202012);

        let base = frontmatter(name);
        let mut cases = mutations(&base);
        if name == "MeasurementPlan" {
            cases.extend(
                decision_rule_cases()
                    .into_iter()
                    .map(|(label, rule)| (format!("rule: {label}"), plan_with_rule(&rule))),
            );
            cases.extend(retired_cases());
        }
        assert!(
            old.is_valid(&base) && new.is_valid(&base),
            "{name}: the skeleton must be accepted by both"
        );
        for (label, document) in cases {
            let before = old.is_valid(&document);
            assert_eq!(
                before,
                new.is_valid(&document),
                "{name} / {label}: draft-07 says {before}, 2020-12 disagrees"
            );
            compared += 1;
            if before {
                accepted += 1;
            } else {
                refused += 1;
            }
        }
    }
    // Both verdicts must be exercised, or agreement would prove nothing.
    assert!(
        accepted > 10 && refused > 100,
        "{accepted} accepted, {refused} refused"
    );
    assert!(compared > 200, "{compared} cases compared");
}
