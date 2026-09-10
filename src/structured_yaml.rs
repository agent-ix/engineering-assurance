// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Pure parsing for unambiguous YAML documents used by assurance adapters.

use serde_json::Value as JsonValue;
use yaml_serde::Value as YamlValue;

/// Parse one YAML document into its JSON value without merge semantics.
///
/// Returns `None` for malformed YAML, duplicate mapping keys, merge keys, or a
/// value that cannot be represented as JSON. The function performs no I/O.
#[must_use]
pub fn parse_unambiguous_yaml_json(bytes: &[u8]) -> Option<JsonValue> {
    let yaml = yaml_serde::from_slice::<YamlValue>(bytes).ok()?;
    if contains_merge_key(&yaml) {
        return None;
    }
    serde_json::to_value(yaml).ok()
}

fn contains_merge_key(value: &YamlValue) -> bool {
    match value {
        YamlValue::Mapping(mapping) => mapping.iter().any(|(key, value)| {
            key.as_str() == Some("<<") || contains_merge_key(key) || contains_merge_key(value)
        }),
        YamlValue::Sequence(sequence) => sequence.iter().any(contains_merge_key),
        YamlValue::Tagged(tagged) => contains_merge_key(&tagged.value),
        YamlValue::Null | YamlValue::Bool(_) | YamlValue::Number(_) | YamlValue::String(_) => false,
    }
}
