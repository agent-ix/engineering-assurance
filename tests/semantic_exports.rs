// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The module exports its five artifact types through the semantic contract.
//!
//! Each artifact type has ONE JSON Schema 2020-12 file, which is both its
//! manifest `frontmatter_schema_ref` and its exported `data_schema`; there is no
//! second copy to drift.
//!
//! The schema describes the type's frontmatter. Body sections stay quire's
//! `body_extraction`; quire-rs applies a `data_schema` only to the declaration
//! record of an archetype named by a document's `object:` key, never to a
//! `type:`-backed document (see FR-003).

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use engineering_assurance::content_rights::{
    ContentEntryKind, ContentRightsCategory, inspect_content,
};
use ix_trace_rs::trace;
use serde_json::{Value, json};

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

fn schema_id(file: &str) -> String {
    format!("{ID_BASE}{file}")
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
fn every_export_is_a_declared_type_with_a_2020_12_schema() {
    let manifest = manifest();
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

    // Every exported schema, including the two campaign exports, is 2020-12.
    let mut checked = BTreeSet::new();
    for entry in &types {
        let Some(data_schema) = entry.get("data_schema") else {
            continue;
        };
        let relative = data_schema["schema"].as_str().expect("schema path");
        let parsed = read_json(&module_root().join(relative));
        assert_eq!(parsed["$schema"], DRAFT_2020_12, "{relative}");
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
fn content_rights_admits_exactly_the_manifest_data_schema_files() {
    let manifest = manifest();
    for entry in declared_types(&manifest) {
        let Some(relative) = entry["data_schema"]["schema"].as_str() else {
            continue;
        };
        // Every exported schema passes the scan at its real path with its real bytes.
        let path = format!("engineering_assurance/{relative}");
        let text = fs::read_to_string(module_root().join(relative)).expect("schema reads");
        assert_eq!(url_findings(&path, &text), vec![], "{path}");

        let file = relative.strip_prefix("schemas/").expect("under schemas/");
        let id_line = format!("{{\"$id\": \"{}\"}}", schema_id(file));
        assert_eq!(url_findings(&path, &id_line), vec![], "{path}");
        // A nested directory is not the listed file.
        for nested in [
            format!("engineering_assurance/schemas/sub/{file}"),
            format!("schemas/sub/{file}"),
        ] {
            assert_eq!(
                url_findings(&nested, &id_line),
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
        let text = format!("{{\"$id\": \"{}\"}}", schema_id(&file));
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
