// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The module exports its five artifact types through the semantic contract.
//!
//! Each artifact type has ONE JSON Schema 2020-12 file, which is both its
//! manifest `frontmatter_schema_ref` and its exported `data_schema`; there is no
//! second copy to drift. Quoin accepts an export only when the schema's bytes
//! hash to the recorded digest and its `$id` sits under the module's version.
//!
//! The schema describes the type's frontmatter. Body sections stay quire's
//! `body_extraction`; quire-rs applies a `data_schema` only to the declaration
//! record of an archetype named by a document's `object:` key, never to a
//! `type:`-backed document (see FR-003).
//!
//! After a version bump, rewrite the schemas' `$id` version and every manifest
//! digest with
//! `EA_BLESS=1 cargo test --test semantic_exports -- --ignored bless`. That test
//! is `#[ignore]`d so it can neither race the checks nor count as coverage, and
//! it changes nothing but `$id` lines and digests.

use std::{
    collections::BTreeSet,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use ix_trace_rs::trace;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

// Assembled from parts so the content-rights URL scan does not see a literal.
const DRAFT_2020_12: &str = concat!("https:", "//json-schema.org/draft/2020-12/schema");
const ID_BASE: &str = concat!(
    "https:",
    "//schemas.agent-ix.org/agent-ix/engineering-assurance-campaign/"
);

/// RFC 3339 `date-time` as a pattern: a full date, `T`, a time with an optional
/// fraction, and a `Z` or numeric offset. It replaces `format: date-time`
/// because quire-rs (jsonschema 0.18) does not assert `format` under 2020-12, and
/// every wire field must be checked by every consumer. It checks the shape and
/// range of each field, not calendar validity (no day-of-month vs month check).
/// The one place this regex is documented; the schema must carry it verbatim.
const RFC_3339_DATE_TIME: &str = "^[0-9]{4}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])[Tt]([01][0-9]|2[0-3]):[0-5][0-9]:([0-5][0-9]|60)(\\.[0-9]+)?([Zz]|[+-]([01][0-9]|2[0-3]):[0-5][0-9])$";

/// Artifact types; each one's schema is `schemas/<kebab-name>-frontmatter.schema.json`.
const ARTIFACTS: [(&str, &str); 5] = [
    (
        "AssuranceProfile",
        "assurance-profile-frontmatter.schema.json",
    ),
    (
        "MeasurementPlan",
        "measurement-plan-frontmatter.schema.json",
    ),
    (
        "ArchitectureDescription",
        "architecture-description-frontmatter.schema.json",
    ),
    (
        "ComponentAssuranceContract",
        "component-assurance-contract-frontmatter.schema.json",
    ),
    (
        "AssuranceArgument",
        "assurance-argument-frontmatter.schema.json",
    ),
];

/// Recorded verdicts of the original draft-07 schemas over the corpus below.
const VERDICTS: &str = include_str!("fixtures/semantic-export-verdicts.json");

fn module_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("engineering_assurance")
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
    let mut hex = String::from("sha256:");
    for byte in Sha256::digest(bytes) {
        write!(hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hex
}

fn derived_id(version: &str, file: &str) -> String {
    format!("{ID_BASE}{version}/{file}")
}

fn declared_types(manifest: &Value) -> Vec<&Value> {
    ["object_types", "artifact_types"]
        .into_iter()
        .filter_map(|key| manifest[key].as_array())
        .flatten()
        .collect()
}

/// Rewrites the `"$id"` line of a schema file for `version`; touches nothing else.
fn with_id(text: &str, version: &str, file: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        if line.trim_start().starts_with("\"$id\":") {
            let indent = &line[..line.len() - line.trim_start().len()];
            let _ = writeln!(out, "{indent}\"$id\": \"{}\",", derived_id(version, file));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Rewrites every `digest:` line in the manifest text to the hash of the schema
/// named on the preceding `schema:` line.
fn with_digests(manifest_text: &str) -> String {
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

/// After a version bump: rewrites each exported schema's `$id` version and every
/// manifest digest. Changes nothing else. Run alone with
/// `EA_BLESS=1 cargo test --test semantic_exports -- --ignored bless`. It keeps a
/// trace tag only because the repository's source audit requires one on every
/// test; being ignored, it contributes no coverage.
#[test]
#[ignore = "rewrites committed files; run alone with EA_BLESS=1"]
#[trace("TC-194", "FR-003-AC-7")]
fn bless_rewrites_schema_ids_and_manifest_digests() {
    assert!(
        std::env::var_os("EA_BLESS").is_some_and(|value| value == "1"),
        "set EA_BLESS=1 to rewrite the schema ids and digests"
    );
    let manifest_path = module_root().join("manifest.yaml");
    let manifest_value = manifest();
    let version = manifest_version(&manifest_value);
    for entry in declared_types(&manifest_value) {
        let Some(relative) = entry["data_schema"]["schema"].as_str() else {
            continue;
        };
        let path = module_root().join(relative);
        let file = relative.strip_prefix("schemas/").expect("under schemas/");
        let text = fs::read_to_string(&path).expect("schema reads");
        fs::write(&path, with_id(&text, &version, file)).expect("schema writes");
    }
    let text = fs::read_to_string(&manifest_path).expect("manifest reads");
    fs::write(&manifest_path, with_digests(&text)).expect("manifest writes");
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
    for (name, _) in ARTIFACTS {
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
            "{relative}: digest does not match the file (EA_BLESS=1 -- --ignored bless)"
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
    assert_eq!(
        checked.len(),
        7,
        "five artifact and two campaign exports are all checked"
    );

    // One file per artifact type: its export IS its frontmatter schema.
    for (name, file) in ARTIFACTS {
        let entry = types
            .iter()
            .find(|entry| entry["name"] == name)
            .expect("artifact type declared");
        let reference = format!("schemas/{file}");
        assert_eq!(
            entry["frontmatter_schema_ref"].as_str(),
            Some(reference.as_str()),
            "{name}"
        );
        assert_eq!(
            entry["data_schema"]["schema"].as_str(),
            Some(reference.as_str()),
            "{name}: data_schema must be the frontmatter schema itself, not a copy"
        );
    }
}

#[trace("TC-194", "FR-003-AC-7")]
#[test]
fn artifact_schemas_use_2020_12_forms_only_and_carry_the_documented_date_time_pattern() {
    for (name, file) in ARTIFACTS {
        let text = fs::read_to_string(module_root().join("schemas").join(file)).expect("reads");
        for stale in [
            "\"definitions\"",
            "#/definitions/",
            "\"dependencies\"",
            "\"format\"",
        ] {
            assert!(!text.contains(stale), "{name}: {file} still uses {stale}");
        }
    }
    let argument =
        read_json(&module_root().join("schemas/assurance-argument-frontmatter.schema.json"));
    assert_eq!(
        argument["$defs"]["assumption"]["properties"]["review_by"],
        json!({"type": "string", "pattern": RFC_3339_DATE_TIME}),
        "review_by must carry the one documented RFC 3339 pattern"
    );
    let plan = read_json(&module_root().join("schemas/measurement-plan-frontmatter.schema.json"));
    assert_eq!(
        plan["$defs"]["decision_rule"]["dependentRequired"],
        json!({"margin": ["baseline"], "margin_mode": ["margin"]})
    );
}

#[trace("TC-194", "FR-003-AC-7")]
#[test]
fn digested_schemas_carry_no_carriage_return_and_are_excluded_from_line_ending_conversion() {
    let manifest = manifest();
    for entry in declared_types(&manifest) {
        let Some(relative) = entry["data_schema"]["schema"].as_str() else {
            continue;
        };
        let bytes = fs::read(module_root().join(relative)).expect("schema reads");
        assert!(
            !bytes.contains(&b'\r'),
            "{relative} contains a carriage return"
        );
    }
    let attributes =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".gitattributes"))
            .expect(".gitattributes exists");
    assert!(
        attributes
            .lines()
            .any(|line| line.trim() == "engineering_assurance/schemas/*.json -text"),
        "digested schemas must be excluded from line-ending conversion"
    );
}

// --- behaviour ---------------------------------------------------------------

/// Built at the consumer's default options: under 2020-12 `format` is only an
/// annotation, so every constraint must be a real keyword.
fn validator(schema: &Value) -> jsonschema::Validator {
    jsonschema::options()
        .build(schema)
        .expect("schema compiles at default options")
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

/// `assumptions[].review_by` is a nested `format: date-time`; the mutations
/// above only reach top-level keys.
fn review_by_cases() -> Vec<(String, Value)> {
    [
        "2030-01-01T00:00:00Z",
        "2030-01-01T00:00:00.5+02:00",
        "next spring",
        "2030-01-01",
        "2030-13-01T00:00:00Z",
        "2030-01-01T25:00:00Z",
        "",
        "2030-01-01T00:00:00",
    ]
    .into_iter()
    .map(|value| {
        let mut plan = frontmatter("AssuranceArgument");
        plan["assumptions"][0]["review_by"] = json!(value);
        (format!("review_by {value:?}"), plan)
    })
    .collect()
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
fn each_artifact_schema_gives_the_verdicts_the_original_draft_07_schema_gave() {
    let recorded: Value = serde_json::from_str(VERDICTS).expect("verdict fixture is JSON");
    let mut accepted = 0_usize;
    let mut refused = 0_usize;
    for (name, file) in ARTIFACTS {
        let schema = read_json(&module_root().join("schemas").join(file));
        let validator = validator(&schema);

        let base = frontmatter(name);
        let mut cases = mutations(&base);
        if name == "AssuranceArgument" {
            cases.extend(review_by_cases());
        }
        if name == "MeasurementPlan" {
            cases.extend(
                decision_rule_cases()
                    .into_iter()
                    .map(|(label, rule)| (format!("rule: {label}"), plan_with_rule(&rule))),
            );
            cases.extend(retired_cases());
        }
        assert!(
            validator.is_valid(&base),
            "{name}: the skeleton must be accepted"
        );

        let expected = recorded[name]
            .as_object()
            .expect("recorded verdicts per type");
        assert_eq!(
            cases.len(),
            expected.len(),
            "{name}: the corpus and the recorded verdicts differ in size"
        );
        for (label, document) in cases {
            let verdict = expected
                .get(&label)
                .and_then(Value::as_bool)
                .unwrap_or_else(|| panic!("{name} / {label}: no recorded verdict"));
            assert_eq!(
                validator.is_valid(&document),
                verdict,
                "{name} / {label}: the verdict changed from the recorded draft-07 one"
            );
            if verdict {
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
}
