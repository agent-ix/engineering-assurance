// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Deterministic rendering of inert compatibility fixtures.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    CANONICAL_FIXTURE_BYTES, NON_SUCCESS_STATES_BYTES, SemanticError, SemanticFixture,
    validate_semantic_fixture_bytes,
};

#[derive(Debug, Deserialize)]
struct CompatibilityCorpus {
    cases: Vec<CompatibilityCase>,
}

#[derive(Debug, Deserialize)]
struct CompatibilityCase {
    id: String,
    kind: String,
    family: String,
    retained_path: String,
    retained_sha256: String,
    derivation: Option<Value>,
    expected: CompatibilityExpected,
}

#[derive(Debug, Deserialize)]
struct CompatibilityExpected {
    outcome: String,
}

#[derive(Debug, Serialize)]
struct GeneratedCompatibilityCase {
    constructed: bool,
    expected_outcome: String,
    family: String,
    id: String,
    kind: String,
    retained_path: String,
    retained_sha256: String,
}

/// Render all inert cross-language fixture samples from canonical semantic data.
///
/// `compatibility_corpus_bytes` are supplied explicitly so the reusable
/// library does not discover a workstation checkout or embed qualification
/// corpus bytes in its production artifact.
///
/// # Errors
///
/// Returns [`SemanticError`] when a canonical semantic source is malformed.
pub fn render_generated_fixtures(
    compatibility_corpus_bytes: &[u8],
) -> Result<BTreeMap<String, String>, SemanticError> {
    let states: Vec<String> =
        serde_json::from_slice(NON_SUCCESS_STATES_BYTES).map_err(|error| {
            SemanticError::new(format!("invalid non-success-state source: {error}"))
        })?;
    if states.is_empty() || states.iter().any(String::is_empty) {
        return Err(SemanticError::new(
            "non-success-state source must contain non-empty states",
        ));
    }
    let canonical: SemanticFixture = validate_semantic_fixture_bytes(CANONICAL_FIXTURE_BYTES)?;
    let canonical = serde_json::to_value(canonical).map_err(|error| {
        SemanticError::new(format!("canonical fixture serialization failed: {error}"))
    })?;
    let canonical_json = serde_json::to_string(&canonical).map_err(|error| {
        SemanticError::new(format!("canonical fixture serialization failed: {error}"))
    })?;
    let quoted_canonical = serde_json::to_string(&canonical_json).map_err(|error| {
        SemanticError::new(format!("canonical fixture quoting failed: {error}"))
    })?;

    let corpus: CompatibilityCorpus = serde_json::from_slice(compatibility_corpus_bytes)
        .map_err(|error| SemanticError::new(format!("invalid compatibility corpus: {error}")))?;
    let cases: Vec<GeneratedCompatibilityCase> = corpus
        .cases
        .into_iter()
        .map(|case| GeneratedCompatibilityCase {
            constructed: case.derivation.is_some(),
            expected_outcome: case.expected.outcome,
            family: case.family,
            id: case.id,
            kind: case.kind,
            retained_path: case.retained_path,
            retained_sha256: case.retained_sha256,
        })
        .collect();
    let cases_json = serde_json::to_string(&cases).map_err(|error| {
        SemanticError::new(format!(
            "compatibility fixture serialization failed: {error}"
        ))
    })?;
    let quoted_cases = serde_json::to_string(&cases_json).map_err(|error| {
        SemanticError::new(format!("compatibility fixture quoting failed: {error}"))
    })?;

    let mut generated = BTreeMap::new();
    insert_state_fixtures(&mut generated, &states);
    insert_canonical_fixtures(&mut generated, &canonical_json, &quoted_canonical);
    insert_compatibility_fixtures(&mut generated, &cases_json, &quoted_cases);
    Ok(generated)
}

fn insert_state_fixtures(generated: &mut BTreeMap<String, String>, states: &[String]) {
    generated.insert(
        "non_success_states.py".to_owned(),
        format!(
            "\"\"\"Generated fixture; semantic source is non-success-states.json.\"\"\"\n\nNON_SUCCESS_STATES = (\n{}\n)\n",
            states.iter().map(|state| format!("    \"{state}\",")).collect::<Vec<_>>().join("\n")
        ),
    );
    generated.insert(
        "non_success_states.ts".to_owned(),
        format!(
            "// Generated fixture; semantic source is non-success-states.json.\nexport const NON_SUCCESS_STATES = [\n{}\n] as const;\n",
            states.iter().map(|state| format!("  \"{state}\",")).collect::<Vec<_>>().join("\n")
        ),
    );
    generated.insert(
        "non_success_states.rs".to_owned(),
        format!(
            "// Generated fixture; semantic source is non-success-states.json.\npub const NON_SUCCESS_STATES: &[&str] = &[\n{}\n];\n",
            states.iter().map(|state| format!("    \"{state}\",")).collect::<Vec<_>>().join("\n")
        ),
    );
}

fn insert_canonical_fixtures(
    generated: &mut BTreeMap<String, String>,
    canonical_json: &str,
    quoted_canonical: &str,
) {
    for (name, prefix, suffix) in [
        (
            "canonical_references.py",
            "\"\"\"Generated fixture; semantic source is canonical-references.json.\"\"\"\nCANONICAL_FIXTURE_JSON = ",
            "\n",
        ),
        (
            "canonical_references.ts",
            "// Generated fixture; semantic source is canonical-references.json.\nexport const CANONICAL_FIXTURE_JSON = ",
            ";\n",
        ),
    ] {
        generated.insert(
            name.to_owned(),
            format!("{prefix}{quoted_canonical}{suffix}"),
        );
    }
    generated.insert(
        "canonical_references.rs".to_owned(),
        format!("// Generated fixture; semantic source is canonical-references.json.\npub const CANONICAL_FIXTURE_JSON: &str = r#\"{canonical_json}\"#;\n"),
    );
}

fn insert_compatibility_fixtures(
    generated: &mut BTreeMap<String, String>,
    cases_json: &str,
    quoted_cases: &str,
) {
    for (name, prefix, suffix) in [
        (
            "compatibility_cases.py",
            "\"\"\"Generated fixture; semantic source is compatibility-corpus/corpus.json.\"\"\"\nCOMPATIBILITY_CASES_JSON = ",
            "\n",
        ),
        (
            "compatibility_cases.ts",
            "// Generated fixture; semantic source is compatibility-corpus/corpus.json.\nexport const COMPATIBILITY_CASES_JSON = ",
            ";\n",
        ),
    ] {
        generated.insert(name.to_owned(), format!("{prefix}{quoted_cases}{suffix}"));
    }
    generated.insert(
        "compatibility_cases.rs".to_owned(),
        format!("// Generated fixture; semantic source is compatibility-corpus/corpus.json.\npub const COMPATIBILITY_CASES_JSON: &str = r#\"{cases_json}\"#;\n"),
    );
}
