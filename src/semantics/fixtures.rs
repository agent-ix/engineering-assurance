// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Deterministic rendering of inert compatibility fixtures.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{
    CANONICAL_FIXTURE_BYTES, NON_SUCCESS_STATES_BYTES, SemanticError, SemanticErrorKind,
    SemanticFixture, validate_semantic_fixture_bytes,
};
use crate::compatibility_corpus::{CorpusCase, CorpusError, CorpusIndex};

/// Translate an accepted-corpus refusal into the semantic refusal vocabulary.
///
/// The generator has no error vocabulary of its own and must not grow one: a
/// caller distinguishes a corpus this implementation does not accept from a
/// corpus that is merely malformed, and both answers already exist here. An
/// unknown corpus version is a version discriminator this build does not
/// support, which is exactly what [`SemanticErrorKind::UnsupportedProtocol`]
/// names; an absent or empty required member is content the corpus does not
/// carry, which is what [`SemanticErrorKind::EmptyFixture`] names.
fn semantic_refusal(error: &CorpusError) -> SemanticError {
    let kind = match error {
        CorpusError::UnknownVersion { .. } => SemanticErrorKind::UnsupportedProtocol,
        CorpusError::MissingMember { .. }
        | CorpusError::MissingRequiredKind { .. }
        | CorpusError::EmptyMember { .. } => SemanticErrorKind::EmptyFixture,
        _ => SemanticErrorKind::InvalidInputEncoding,
    };
    SemanticError::new(kind, format!("invalid compatibility corpus: {error}"))
}

/// Project one accepted corpus case onto the inert generated-fixture shape.
///
/// The projection is built as a sorted map rather than a struct because the
/// emitted key order is load-bearing: every committed `compatibility_cases.*`
/// fixture is compared byte for byte against this output, and three languages
/// read those bytes. A struct serializes its members in declaration order, so
/// the previous alphabetical output held only because the members happened to
/// be declared alphabetically, and reordering them — a change no reviewer would
/// read as behavioral — would silently rewrite every generated fixture. A
/// [`BTreeMap`] keyed by the field name makes the order a property of the data
/// structure instead of a property of how the source is typed.
fn generated_case(case: CorpusCase) -> BTreeMap<&'static str, Value> {
    BTreeMap::from([
        ("constructed", Value::Bool(case.constructed())),
        ("expected_outcome", Value::String(case.expected.outcome)),
        ("family", Value::String(case.family)),
        ("id", Value::String(case.id)),
        ("kind", Value::String(case.kind)),
        ("retained_path", Value::String(case.retained_path)),
        ("retained_sha256", Value::String(case.retained_sha256)),
    ])
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

    // The generator reads the corpus through the same validating parser the
    // accepted-corpus gate uses, rather than through a private shape that
    // deserializes whatever happens to have a `cases` array. Without this an
    // index carrying an unknown `corpus_version`, or one missing its producer
    // cases, chain or stated limitations, would still render generated fixtures
    // that three languages then read as authoritative.
    let corpus =
        CorpusIndex::parse(compatibility_corpus_bytes).map_err(|error| semantic_refusal(&error))?;
    let cases: Vec<BTreeMap<&'static str, Value>> =
        corpus.cases.into_iter().map(generated_case).collect();
    let cases_json = serde_json::to_string(&cases).map_err(|error| {
        SemanticError::new(
            SemanticErrorKind::Serialization,
            format!("compatibility fixture serialization failed: {error}"),
        )
    })?;
    let quoted_cases = serde_json::to_string(&cases_json).map_err(|error| {
        SemanticError::new(
            SemanticErrorKind::Serialization,
            format!("compatibility fixture quoting failed: {error}"),
        )
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

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use serde_json::{Value, json};

    use super::{
        BTreeMap, CorpusIndex, SemanticErrorKind, render_generated_fixtures, semantic_refusal,
    };
    use crate::compatibility_corpus::{CORPUS_VERSION, REQUIRED_KINDS, sha256_hex};

    /// A structurally valid synthetic corpus index.
    ///
    /// Synthetic rather than the accepted corpus, because this is the reusable
    /// library: embedding qualification corpus bytes here would carry them into
    /// the production artifact. The accepted corpus is read by the confined
    /// host adapter in the qualification tree.
    fn valid_index() -> Value {
        let digest = sha256_hex(b"retained bytes");
        let cases: Vec<Value> = REQUIRED_KINDS
            .iter()
            .map(|kind| {
                json!({
                    "id": format!("case-{kind}"),
                    "kind": kind,
                    "family": "pgm01-v1",
                    "retained_path": format!("records/{kind}.json"),
                    "retained_sha256": digest,
                    "origin": null,
                    "derivation": null,
                    "expected": {"outcome": "lossy", "note": "a fictional expectation"},
                })
            })
            .collect();
        json!({
            "corpus_version": CORPUS_VERSION,
            "purpose": "a fictional index for generator-boundary tests",
            "limitations": ["Every case here is fictional."],
            "cases": cases,
            "producer_cases": [{
                "id": "producer-retained",
                "language": "rust",
                "producer": "agent-ix/fictional-producer",
                "revision": "0".repeat(40),
                "path": "tests/golden/fixture.json",
                "feeds": "check_result",
                "retention": "retained",
                "note": "a fictional retained producer case",
                "source_sha256": digest,
                "retained_path": "producers/retained.json",
                "retained_sha256": digest,
            }],
            "chain": {
                "purpose": "a fictional chain",
                "subject": {"repository": "agent-ix/fictional", "revision": "2".repeat(40)},
                "tools": {"quire": {"version": "0.0.0"}},
                "artifacts": [{
                    "role": "change_assurance_record",
                    "produced_by": "quoin",
                    "retention": "retained",
                    "retained_path": "chain/record.json",
                    "retained_sha256": digest,
                }],
                "referenced_inputs": [],
            },
        })
    }

    fn rendered(index: &Value) -> BTreeMap<String, String> {
        render_generated_fixtures(&serde_json::to_vec(index).expect("index must serialize"))
            .expect("a valid corpus index must render")
    }

    #[test]
    #[trace("TC-104", "FR-015-AC-4")]
    fn an_unknown_corpus_version_refuses_before_any_fixture_is_generated() {
        let mut unknown = valid_index();
        unknown["corpus_version"] = json!("engineering-assurance.compatibility-corpus/v2");
        let error =
            render_generated_fixtures(&serde_json::to_vec(&unknown).expect("index must serialize"))
                .expect_err("an unaccepted corpus version must not produce generated fixtures");
        assert_eq!(error.kind(), SemanticErrorKind::UnsupportedProtocol);

        // The generated fixtures are read as authoritative by three languages,
        // so a corpus that merely deserializes is not enough: dropping a member
        // the accepted corpus must carry has to refuse here too, or a truncated
        // corpus would quietly regenerate a smaller committed fixture set.
        for member in ["producer_cases", "limitations"] {
            let mut truncated = valid_index();
            truncated[member] = json!([]);
            let error = render_generated_fixtures(
                &serde_json::to_vec(&truncated).expect("index must serialize"),
            )
            .expect_err("a corpus missing a required member must not render");
            assert_eq!(error.kind(), SemanticErrorKind::EmptyFixture, "{member}");
        }

        let mut absent = valid_index();
        absent
            .as_object_mut()
            .expect("the index is an object")
            .remove("chain");
        assert_eq!(
            render_generated_fixtures(&serde_json::to_vec(&absent).expect("index must serialize"))
                .expect_err("a corpus with no chain must not render")
                .kind(),
            SemanticErrorKind::InvalidInputEncoding
        );

        // Every refusal arm is reachable from a real corpus error, so the
        // mapping cannot silently collapse onto one answer.
        let unknown_version = CorpusIndex::parse(
            &serde_json::to_vec(&{
                let mut index = valid_index();
                index["corpus_version"] = json!("other/v9");
                index
            })
            .expect("index must serialize"),
        )
        .expect_err("an unaccepted version must refuse");
        assert_eq!(
            semantic_refusal(&unknown_version).kind(),
            SemanticErrorKind::UnsupportedProtocol
        );
    }

    #[test]
    #[trace("TC-104", "FR-015-AC-4")]
    fn generated_compatibility_case_keys_are_emitted_in_sorted_order() {
        let generated = rendered(&valid_index());
        let source = generated
            .get("compatibility_cases.rs")
            .expect("the compatibility fixture must be generated");
        let start = source
            .find('[')
            .expect("the generated fixture embeds a JSON array");
        let end = source
            .rfind(']')
            .expect("the generated fixture embeds a JSON array");
        let embedded = &source[start..=end];
        let cases: Vec<BTreeMap<String, Value>> =
            serde_json::from_str(embedded).expect("the embedded array must parse");
        assert!(!cases.is_empty(), "the generator emitted no case");

        // Three committed fixtures are compared byte for byte against this
        // output, so the key order is part of the contract rather than a
        // cosmetic detail. The keys are read out of the emitted text rather
        // than out of the decoded cases: decoding sorts them, so a decoded case
        // agrees with its own sort whatever the bytes said, and the assertion
        // would hold even if the generator emitted them in any order at all.
        let expected_shape = [
            "constructed",
            "expected_outcome",
            "family",
            "id",
            "kind",
            "retained_path",
            "retained_sha256",
        ];
        let mut emitted = Vec::new();
        let mut rest = embedded;
        while let Some(open) = rest.find('"') {
            let after = &rest[open + 1..];
            let Some(close) = after.find('"') else { break };
            let token = &after[..close];
            let tail = &after[close + 1..];
            if tail.starts_with(':') {
                emitted.push(token);
            }
            rest = tail;
        }
        assert_eq!(
            emitted.len(),
            cases.len() * expected_shape.len(),
            "the emitted text does not carry one complete key set per case"
        );
        for (index, chunk) in emitted.chunks(expected_shape.len()).enumerate() {
            assert_eq!(
                chunk, expected_shape,
                "case {index} was emitted with a different key shape or order"
            );
        }
    }
}
