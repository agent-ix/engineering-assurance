// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Deterministic rendering of inert compatibility fixtures.

use std::collections::BTreeMap;

use super::{
    CANONICAL_FIXTURE_BYTES, NON_SUCCESS_STATES_BYTES, SemanticError, SemanticErrorKind,
    SemanticFixture, validate_semantic_fixture_bytes,
};
use crate::claim_strength::ClaimStrength;

/// Render all inert cross-language fixture samples from canonical semantic data.
///
/// # Errors
///
/// Returns [`SemanticError`] when a canonical semantic source is malformed.
pub fn render_generated_fixtures() -> Result<BTreeMap<String, String>, SemanticError> {
    let states: Vec<String> =
        serde_json::from_slice(NON_SUCCESS_STATES_BYTES).map_err(|error| {
            SemanticError::new(
                SemanticErrorKind::InvalidInputEncoding,
                format!("invalid non-success-state source: {error}"),
            )
        })?;
    if states.is_empty() || states.iter().any(String::is_empty) {
        return Err(SemanticError::new(
            SemanticErrorKind::EmptyFixture,
            "non-success-state source must contain non-empty states",
        ));
    }
    let canonical: SemanticFixture = validate_semantic_fixture_bytes(CANONICAL_FIXTURE_BYTES)?;
    let canonical = serde_json::to_value(canonical).map_err(|error| {
        SemanticError::new(
            SemanticErrorKind::Serialization,
            format!("canonical fixture serialization failed: {error}"),
        )
    })?;
    let canonical_json = serde_json::to_string(&canonical).map_err(|error| {
        SemanticError::new(
            SemanticErrorKind::Serialization,
            format!("canonical fixture serialization failed: {error}"),
        )
    })?;
    let quoted_canonical = serde_json::to_string(&canonical_json).map_err(|error| {
        SemanticError::new(
            SemanticErrorKind::Serialization,
            format!("canonical fixture quoting failed: {error}"),
        )
    })?;

    let mut generated = BTreeMap::new();
    insert_state_fixtures(&mut generated, &states);
    insert_claim_strength_fixtures(&mut generated);
    insert_canonical_fixtures(&mut generated, &canonical_json, &quoted_canonical);
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

/// Project the FR-022 vocabulary from [`ClaimStrength::ALL`] itself, so the
/// foreign-language copies cannot drift from the type every Rust consumer uses.
/// The listing order is carried over for stable bytes; it is not a rank.
fn insert_claim_strength_fixtures(generated: &mut BTreeMap<String, String>) {
    let listed = |indent: &str| {
        ClaimStrength::ALL
            .iter()
            .map(|strength| format!("{indent}\"{strength}\","))
            .collect::<Vec<_>>()
            .join("\n")
    };
    generated.insert(
        "claim_strengths.py".to_owned(),
        format!(
            "\"\"\"Generated fixture; semantic source is engineering_assurance::claim_strength::ClaimStrength. Unordered.\"\"\"\n\nCLAIM_STRENGTHS = (\n{}\n)\n",
            listed("    ")
        ),
    );
    generated.insert(
        "claim_strengths.ts".to_owned(),
        format!(
            "// Generated fixture; semantic source is engineering_assurance::claim_strength::ClaimStrength. Unordered.\nexport const CLAIM_STRENGTHS = [\n{}\n] as const;\n",
            listed("  ")
        ),
    );
    generated.insert(
        "claim_strengths.rs".to_owned(),
        format!(
            "// Generated fixture; semantic source is engineering_assurance::claim_strength::ClaimStrength. Unordered.\npub const CLAIM_STRENGTHS: &[&str] = &[\n{}\n];\n",
            listed("    ")
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
