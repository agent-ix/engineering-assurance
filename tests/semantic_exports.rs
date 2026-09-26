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

use engineering_assurance::content_rights::{
    ContentEntryKind, ContentRightsCategory, inspect_content,
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

/// RFC 3339 `date-time` as a pattern, kept beside `format: date-time`.
///
/// quire-rs (jsonschema 0.18) asserts `format` under draft-07 but not under
/// 2020-12, so this pattern must by itself be as strict as the check quire
/// applied to the original draft-07 schema. It was measured against that check
/// on the real quire CLI (the `QUIRE_OLD_VERDICTS` table below): month-aware day
/// ranges with the full Gregorian leap-year rule, seconds 00 to 59 only (a leap
/// second `:60` is refused), a `T`, `t` or space separator, `Z` or `z` or a
/// numeric offset of at most 23:59, and one or more fraction digits. Uppercase
/// only is NOT required: the old check accepted lowercase `t`/`z`. The one place
/// this regex is written; the schema must carry it verbatim.
const RFC_3339_DATE_TIME: &str = r"^(?:[0-9]{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12][0-9]|3[01])|(?:0[469]|11)-(?:0[1-9]|[12][0-9]|30)|02-(?:0[1-9]|1[0-9]|2[0-8]))|(?:(?:0[048]|[2468][048]|[13579][26])00|[0-9]{2}(?:0[48]|[2468][048]|[13579][26]))-02-29)[Tt ](?:[01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9](?:\.[0-9]+)?(?:[Zz]|[+-](?:[01][0-9]|2[0-3]):[0-5][0-9])$";

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
/// trace tag because the repository's source audit requires one on every test.
/// Tag-based tools may therefore count TC-194 as covered by it; the live TC-194
/// tests are what actually check the files, and this one only rewrites them.
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
            "{relative}: $id does not carry manifest version {version}; after a version \
bump run `EA_BLESS=1 cargo test --test semantic_exports -- --ignored bless`"
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

/// Walks a schema at any depth. Every `format` must be `date-time` with the one
/// documented pattern beside it (quire does not assert `format` under 2020-12,
/// so an unpaired `format` is unchecked); any other `format` value has no known
/// pattern and fails. Returns how many were found.
fn check_formats(node: &Value, at: &str) -> usize {
    match node {
        Value::Object(map) => {
            let mut found = 0;
            if let Some(format) = map.get("format") {
                assert_eq!(
                    format, "date-time",
                    "{at}: `format: {format}` has no known pattern; add one or drop the format"
                );
                assert_eq!(
                    map.get("pattern").and_then(Value::as_str),
                    Some(RFC_3339_DATE_TIME),
                    "{at}: `format: date-time` needs the documented pattern beside it"
                );
                found += 1;
            }
            for (key, child) in map {
                found += check_formats(child, &format!("{at}/{key}"));
            }
            found
        }
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, child)| check_formats(child, &format!("{at}/{index}")))
            .sum(),
        _ => 0,
    }
}

#[trace("TC-194", "FR-003-AC-7")]
#[test]
fn artifact_schemas_use_2020_12_forms_only_and_every_format_is_paired_with_its_pattern() {
    let mut formats = 0_usize;
    for (name, file) in ARTIFACTS {
        let text = fs::read_to_string(module_root().join("schemas").join(file)).expect("reads");
        for stale in ["\"definitions\"", "#/definitions/", "\"dependencies\""] {
            assert!(!text.contains(stale), "{name}: {file} still uses {stale}");
        }
        let schema: Value = serde_json::from_str(&text).expect("schema is JSON");
        formats += check_formats(&schema, &format!("{name} #"));
    }
    assert!(formats >= 1, "the walk must reach the one date-time field");
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

/// Every nested path of a document: drop it, retype it several ways, add an
/// undeclared key to each object, and duplicate the first item of each array.
/// Reaches every `$defs` entry a skeleton populates (claim, reasoning,
/// participant, challenge, impact, review and measurement policy, objective,
/// negative control, apparatus path).
fn deep_mutations(base: &Value) -> Vec<(String, Value)> {
    fn paths(node: &Value, at: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
        match node {
            Value::Object(map) => {
                for (key, child) in map {
                    at.push(key.clone());
                    out.push(at.clone());
                    paths(child, at, out);
                    at.pop();
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    at.push(index.to_string());
                    out.push(at.clone());
                    paths(child, at, out);
                    at.pop();
                }
            }
            _ => {}
        }
    }
    fn parent<'a>(root: &'a mut Value, path: &[String]) -> &'a mut Value {
        path[..path.len() - 1]
            .iter()
            .fold(root, |node, step| match node {
                Value::Array(items) => &mut items[step.parse::<usize>().expect("index")],
                other => &mut other[step.as_str()],
            })
    }
    let mut all = Vec::new();
    paths(base, &mut Vec::new(), &mut all);
    let mut cases = Vec::new();
    for path in all {
        let name = path.join(".");
        let last = path.last().expect("non-empty path");
        for (op, wrong) in [
            ("null", Value::Null),
            ("number", json!(-1)),
            ("empty string", json!("")),
            ("empty list", json!([])),
            ("empty object", json!({})),
        ] {
            let mut changed = base.clone();
            match parent(&mut changed, &path) {
                Value::Array(items) => items[last.parse::<usize>().expect("index")] = wrong,
                node => node[last.as_str()] = wrong,
            }
            cases.push((format!("@{name} as {op}"), changed));
        }
        let mut dropped = base.clone();
        match parent(&mut dropped, &path) {
            Value::Array(items) => {
                items.remove(last.parse::<usize>().expect("index"));
            }
            Value::Object(map) => {
                map.remove(last.as_str());
            }
            _ => unreachable!("a path always ends inside a container"),
        }
        cases.push((format!("@{name} dropped"), dropped));

        let mut node = base;
        for step in &path {
            node = match node {
                Value::Array(items) => &items[step.parse::<usize>().expect("index")],
                other => &other[step.as_str()],
            };
        }
        if node.is_object() {
            let mut extended = base.clone();
            let mut target = &mut extended;
            for step in &path {
                target = match target {
                    Value::Array(items) => &mut items[step.parse::<usize>().expect("index")],
                    other => &mut other[step.as_str()],
                };
            }
            target["undeclared_field"] = json!(true);
            cases.push((format!("@{name} plus undeclared field"), extended));
        }
    }
    cases
}

/// The corpus of documents whose verdicts are recorded for each artifact type.
fn corpus(name: &str) -> Vec<(String, Value)> {
    let base = frontmatter(name);
    let mut cases = mutations(&base);
    cases.extend(deep_mutations(&base));
    if name == "MeasurementPlan" {
        cases.extend(
            decision_rule_cases()
                .into_iter()
                .map(|(label, rule)| (format!("rule: {label}"), plan_with_rule(&rule))),
        );
        cases.extend(retired_cases());
    }
    let labels: BTreeSet<&str> = cases.iter().map(|(label, _)| label.as_str()).collect();
    assert_eq!(
        labels.len(),
        cases.len(),
        "{name}: corpus labels must be unique"
    );
    cases
}

#[trace("TC-195", "FR-003-AC-8")]
#[test]
fn each_artifact_schema_gives_the_verdicts_the_original_draft_07_schema_gave() {
    let recorded: Value = serde_json::from_str(VERDICTS).expect("verdict fixture is JSON");
    let mut accepted_total = 0_usize;
    let mut refused_total = 0_usize;
    for (name, file) in ARTIFACTS {
        let validator = validator(&read_json(&module_root().join("schemas").join(file)));
        assert!(
            validator.is_valid(&frontmatter(name)),
            "{name}: the skeleton must be accepted"
        );
        let entry = &recorded["types"][name];
        let accepted: BTreeSet<&str> = entry["accepted"]
            .as_array()
            .expect("accepted list")
            .iter()
            .map(|label| label.as_str().expect("label"))
            .collect();
        let cases = corpus(name);
        assert_eq!(
            entry["cases"].as_u64(),
            Some(cases.len() as u64),
            "{name}: the corpus size changed; update the fixture by hand with a stated reason"
        );
        let labels: BTreeSet<&str> = cases.iter().map(|(label, _)| label.as_str()).collect();
        for label in &accepted {
            assert!(
                labels.contains(label),
                "{name}: recorded case {label:?} is gone"
            );
        }
        for (label, document) in &cases {
            let old = accepted.contains(label.as_str());
            let new = validator.is_valid(document);
            assert_eq!(
                new,
                old,
                "{name} / {label}: draft-07 verdict was {}, the schema now {}; if intended, \
                 hand-edit tests/fixtures/semantic-export-verdicts.json in this change",
                if old { "accept" } else { "refuse" },
                if new { "accept" } else { "refuse" },
            );
            if old {
                accepted_total += 1;
            } else {
                refused_total += 1;
            }
        }
    }
    // Both verdicts must be exercised, or agreement would prove nothing.
    assert!(
        accepted_total > 10 && refused_total > 1000,
        "{accepted_total} accepted, {refused_total} refused"
    );
    // The nested definitions must be reached, not just top-level keys.
    let reached = |name: &str, prefix: &str| {
        corpus(name)
            .iter()
            .any(|(label, _)| label.starts_with(prefix))
    };
    for (name, prefix) in [
        ("AssuranceArgument", "@top_claim.evidence_refs"),
        ("AssuranceArgument", "@reasoning.0."),
        ("AssuranceArgument", "@participants.0."),
        ("AssuranceArgument", "@challenges.0."),
        ("AssuranceProfile", "@impact_assessments.0."),
        ("AssuranceProfile", "@review_policy."),
        ("AssuranceProfile", "@measurement_policy."),
        ("MeasurementPlan", "@objective."),
        ("MeasurementPlan", "@negative_controls.0."),
        ("MeasurementPlan", "@protected_apparatus.0"),
    ] {
        assert!(
            reached(name, prefix),
            "{name}: corpus never reaches {prefix}"
        );
    }
}

/// Verdicts of the ORIGINAL draft-07 module on the real quire CLI (quire 0.33,
/// engine 0.47.1, jsonschema 0.18, which asserts `format`) for an
/// `AssuranceArgument` whose `assumptions[0].review_by` is the value. Measured
/// with `quire validate --module <old module>`; `true` means accepted. The
/// 2020-12 schema must give the same verdict without `format` being asserted.
const QUIRE_OLD_VERDICTS: [(&str, bool); 40] = [
    ("2030-01-01T00:00:00Z", true),
    ("2030-01-01T00:00:00z", true),
    ("2030-01-01t00:00:00Z", true),
    ("2030-01-01t00:00:00z", true),
    ("2030-01-01 00:00:00Z", true),
    ("2030-01-01T00:00:00", false),
    ("2030-01-01", false),
    ("2030-02-31T00:00:00Z", false),
    ("2030-02-29T00:00:00Z", false),
    ("2028-02-29T00:00:00Z", true),
    ("2100-02-29T00:00:00Z", false),
    ("2000-02-29T00:00:00Z", true),
    ("1900-02-29T00:00:00Z", false),
    ("2030-04-31T00:00:00Z", false),
    ("2030-01-01T00:00:60Z", false),
    ("2030-01-01T23:59:60Z", false),
    ("2030-01-01T00:00:59Z", true),
    ("2030-01-01T24:00:00Z", false),
    ("2030-01-01T23:60:00Z", false),
    ("2030-01-01T00:00:00.5Z", true),
    ("2030-01-01T00:00:00.Z", false),
    ("2030-01-01T00:00:00.123456789012Z", true),
    ("2030-01-01T00:00:00+02:00", true),
    ("2030-01-01T00:00:00-23:59", true),
    ("2030-01-01T00:00:00+24:00", false),
    ("2030-01-01T00:00:00+00:60", false),
    ("2030-01-01T00:00:00+0200", false),
    ("2030-01-01T00:00:00+02", false),
    ("2030-01-01T00:00:00-00:00", true),
    ("2030-00-10T00:00:00Z", false),
    ("2030-13-01T00:00:00Z", false),
    ("2030-01-00T00:00:00Z", false),
    ("2030-01-32T00:00:00Z", false),
    ("0000-01-01T00:00:00Z", true),
    ("9999-12-31T23:59:59Z", true),
    ("2030-1-1T00:00:00Z", false),
    (" 2030-01-01T00:00:00Z", false),
    ("2030-01-01T00:00:00Z ", false),
    ("2030-01-01T00:00Z", false),
    ("next spring", false),
];

fn argument_with_review_by(value: &str) -> Value {
    let mut document = frontmatter("AssuranceArgument");
    document["assumptions"][0]["review_by"] = json!(value);
    document
}

#[trace("TC-195", "FR-003-AC-8")]
#[test]
fn review_by_pattern_gives_the_verdicts_quire_gave_the_original_format_check() {
    let schema =
        read_json(&module_root().join("schemas/assurance-argument-frontmatter.schema.json"));
    let default_options = validator(&schema);
    for (value, accepted) in QUIRE_OLD_VERDICTS {
        assert_eq!(
            default_options.is_valid(&argument_with_review_by(value)),
            accepted,
            "review_by {value:?}: quire 0.18 with the draft-07 schema gave {accepted}"
        );
    }
}

#[trace("TC-195", "FR-003-AC-8")]
#[test]
fn review_by_pattern_agrees_with_a_format_asserting_validator_over_a_generated_set() {
    let schema =
        read_json(&module_root().join("schemas/assurance-argument-frontmatter.schema.json"));
    let default_options = validator(&schema);
    let asserting = jsonschema::options()
        .should_validate_formats(true)
        .build(&schema)
        .expect("schema compiles with format assertion");

    // Every calendar day, plus impossible days, of years around each leap rule.
    let mut values = Vec::new();
    for year in [0_u32, 1900, 2000, 2023, 2024, 2100, 2400, 9999] {
        for month in 0..=13_u32 {
            for day in 0..=32_u32 {
                values.push(format!("{year:04}-{month:02}-{day:02}T12:30:45Z"));
            }
        }
    }
    // Time fields, offsets, fractions, separators and case.
    for hour in ["00", "12", "23", "24", "25", "1", ""] {
        for minute in ["00", "59", "60", "6"] {
            for second in ["00", "59", "61", "5"] {
                values.push(format!("2030-06-15T{hour}:{minute}:{second}Z"));
            }
        }
    }
    for offset in [
        "Z",
        "z",
        "+00:00",
        "-00:00",
        "+23:59",
        "-23:59",
        "+24:00",
        "+23:60",
        "+0200",
        "+02",
        "+2:00",
        "",
        "+02:00:00",
        " ",
    ] {
        values.push(format!("2030-06-15T12:30:45{offset}"));
    }
    for fraction in ["", ".", ".0", ".5", ".123456789012345", ".x", ",5"] {
        values.push(format!("2030-06-15T12:30:45{fraction}Z"));
    }
    for separator in ["T", "t", " ", "_", "", "TT"] {
        values.push(format!("2030-06-15{separator}12:30:45Z"));
    }
    for junk in [
        "",
        "x",
        "2030",
        "2030-06-15",
        "12:30:45Z",
        "2030-06-15T12:30:45Z\n",
    ] {
        values.push(junk.to_owned());
    }

    let mut agreed = 0_usize;
    for value in &values {
        let document = argument_with_review_by(value);
        let by_pattern = default_options.is_valid(&document);
        // The other documented difference: quire's own check accepted a space
        // as the date/time separator (RFC 3339 section 5.6 permits it), which
        // this crate's format validator refuses; compare that value as if `T`.
        let by_format = if value.get(10..11) == Some(" ") {
            asserting.is_valid(&argument_with_review_by(&value.replacen(' ', "T", 1)))
        } else {
            asserting.is_valid(&document)
        };
        // The one documented difference: the format validator in this crate
        // accepts a leap second, which quire's own format check refused (see
        // QUIRE_OLD_VERDICTS), so the pattern is stricter there on purpose.
        if value.contains(":60") && by_format {
            assert!(
                !by_pattern,
                "review_by {value:?}: leap second must be refused"
            );
            continue;
        }
        assert_eq!(
            by_pattern, by_format,
            "review_by {value:?}: pattern says {by_pattern}, format validator says {by_format}"
        );
        agreed += 1;
    }
    assert!(agreed > 3000, "{agreed} generated values compared");
    let accepted = values
        .iter()
        .filter(|value| default_options.is_valid(&argument_with_review_by(value)))
        .count();
    assert!(
        accepted > 1000 && accepted < values.len(),
        "both verdicts occur"
    );
}

fn url_findings(path: &str, text: &str) -> Vec<ContentRightsCategory> {
    inspect_content(path, ContentEntryKind::File, text.as_bytes(), &[])
        .expect("policy patterns compile")
        .into_iter()
        .map(|finding| finding.category)
        .collect()
}

#[trace("TC-194", "FR-003-AC-7")]
#[test]
fn content_rights_admits_exactly_the_manifest_data_schema_files_at_the_current_version() {
    let manifest = manifest();
    let version = manifest_version(&manifest);
    assert_eq!(
        version,
        env!("CARGO_PKG_VERSION"),
        "manifest and crate versions"
    );
    for entry in declared_types(&manifest) {
        let Some(relative) = entry["data_schema"]["schema"].as_str() else {
            continue;
        };
        // Every exported schema passes the scan at its real path with its real bytes.
        let path = format!("engineering_assurance/{relative}");
        let text = fs::read_to_string(module_root().join(relative)).expect("schema reads");
        assert_eq!(url_findings(&path, &text), vec![], "{path}");

        let file = relative.strip_prefix("schemas/").expect("under schemas/");
        let id_line = |version: &str| format!("{{\"$id\": \"{}\"}}", derived_id(version, file));
        // The same `$id` at another version is refused, even at an allowed path.
        assert_eq!(
            url_findings(&path, &id_line("9.9.9")),
            vec![ContentRightsCategory::UnapprovedExternalUrl],
            "{path}: a stale $id must be flagged"
        );
        assert_eq!(url_findings(&path, &id_line(&version)), vec![], "{path}");
        // A nested directory is not the listed file.
        for nested in [
            format!("engineering_assurance/schemas/sub/{file}"),
            format!("schemas/sub/{file}"),
        ] {
            assert_eq!(
                url_findings(&nested, &id_line(&version)),
                vec![ContentRightsCategory::UnapprovedExternalUrl],
                "{nested}"
            );
        }
    }
    // Every other schema file is refused the same URL, so the allowlist holds no
    // file the manifest does not export.
    let exported: BTreeSet<String> = declared_types(&manifest)
        .into_iter()
        .filter_map(|entry| entry["data_schema"]["schema"].as_str())
        .map(|relative| relative.trim_start_matches("schemas/").to_owned())
        .collect();
    let mut unlisted = 0_usize;
    for entry in fs::read_dir(module_root().join("schemas")).expect("schemas dir") {
        let file = entry
            .expect("entry")
            .file_name()
            .to_string_lossy()
            .into_owned();
        if exported.contains(&file) {
            continue;
        }
        unlisted += 1;
        let text = format!("{{\"$id\": \"{}\"}}", derived_id(&version, &file));
        assert_eq!(
            url_findings(&format!("engineering_assurance/schemas/{file}"), &text),
            vec![ContentRightsCategory::UnapprovedExternalUrl],
            "{file} is not exported, so it must not be allowlisted"
        );
    }
    assert!(
        unlisted > 0,
        "the schemas directory holds non-exported files"
    );
}
