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
    // Every variant is named rather than swept into a catch-all. `CorpusError`
    // is a closed enum this crate owns, so a wildcard arm would silently
    // relabel a future variant as an encoding failure and the compiler would
    // never say so; the one place that is visible is a caller acting on the
    // wrong reason. Listing the variants makes adding one a compile error here,
    // which is exactly where the decision belongs.
    let kind = match error {
        CorpusError::UnknownVersion { .. } => SemanticErrorKind::UnsupportedProtocol,
        CorpusError::MissingMember { .. }
        | CorpusError::MissingRequiredKind { .. }
        | CorpusError::EmptyMember { .. } => SemanticErrorKind::EmptyFixture,
        CorpusError::IndexTooLarge
        | CorpusError::InvalidIndex { .. }
        | CorpusError::DuplicateIdentity { .. }
        | CorpusError::UnsafeRetainedPath { .. }
        | CorpusError::InvalidRecordedDigest { .. }
        | CorpusError::RetainedCaseWithoutPath { .. }
        | CorpusError::RetainedCaseWithoutDigest { .. }
        | CorpusError::ReferencedCaseWithPath { .. }
        | CorpusError::ReferencedCaseWithDigest { .. }
        | CorpusError::RetainedArtifactWithoutPath { .. }
        | CorpusError::RetainedArtifactWithoutDigest { .. }
        | CorpusError::NotRetained { .. }
        | CorpusError::DigestMismatch { .. }
        | CorpusError::UnknownIdentity { .. } => SemanticErrorKind::InvalidInputEncoding,
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

    // `parse` alone is not what the gate requires. The gate also calls
    // `require_all_kinds`, and without the same call here a corpus that had
    // dropped every `tampered` case — or any other required state — still
    // rendered fixtures, which three languages then read as the full set of
    // states they must distinguish. A generator that validates less than the
    // gate it claims to share is a generator that can emit fixtures the gate
    // would have refused.
    corpus
        .require_all_kinds()
        .map_err(|error| semantic_refusal(&error))?;
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
    use std::fmt;

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

        // Dropping a required state has to refuse here for the same reason.
        // `CorpusIndex::parse` does not check the required states — the gate
        // calls `require_all_kinds` separately — so a corpus that had lost
        // every `tampered` case parsed cleanly and rendered a fixture set that
        // three languages then read as the full set of states. The generator
        // claims to read the corpus through the gate's parser; this asserts it
        // applies the gate's whole contract and not the half of it that
        // `parse` happens to cover.
        for dropped in REQUIRED_KINDS {
            let mut incomplete = valid_index();
            incomplete["cases"] = json!(
                incomplete["cases"]
                    .as_array()
                    .expect("the fictional index lists its cases")
                    .iter()
                    .filter(|case| case["kind"] != json!(dropped))
                    .cloned()
                    .collect::<Vec<Value>>()
            );
            let error = render_generated_fixtures(
                &serde_json::to_vec(&incomplete).expect("index must serialize"),
            )
            .expect_err("a corpus missing a required state must not render fixtures");
            assert_eq!(error.kind(), SemanticErrorKind::EmptyFixture, "{dropped}");
        }

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

    /// Case keys in the order the generated bytes actually present them.
    ///
    /// Deserializing into a map would re-order the keys before they could be
    /// compared, which is exactly the mistake this capture exists to avoid: it
    /// visits the emitted object once and keeps the encounter order.
    struct EmittedKeys(Vec<String>);

    impl<'de> serde::Deserialize<'de> for EmittedKeys {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct Visitor;

            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = EmittedKeys;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("one generated compatibility case object")
                }

                fn visit_map<M>(self, mut map: M) -> Result<EmittedKeys, M::Error>
                where
                    M: serde::de::MapAccess<'de>,
                {
                    let mut keys = Vec::new();
                    while let Some(key) = map.next_key::<String>()? {
                        map.next_value::<serde::de::IgnoredAny>()?;
                        keys.push(key);
                    }
                    Ok(EmittedKeys(keys))
                }
            }

            deserializer.deserialize_map(Visitor)
        }
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
        let cases: Vec<EmittedKeys> =
            serde_json::from_str(&source[start..=end]).expect("the embedded array must parse");
        assert!(!cases.is_empty(), "the generator emitted no case");

        // Three committed fixtures are compared byte for byte against this
        // output, so the key order is part of the contract rather than a
        // cosmetic detail. The keys are captured in the order they appear in
        // the emitted text, because reading them back into a map would sort
        // them on the way in and leave the assertion unable to fail: the order
        // that actually reached the bytes would never reach the comparison.
        for case in &cases {
            let emitted = case.0.iter().map(String::as_str).collect::<Vec<_>>();
            let mut sorted = emitted.clone();
            sorted.sort_unstable();
            assert_eq!(emitted, sorted, "generated case keys are not sorted");
            assert_eq!(
                emitted,
                [
                    "constructed",
                    "expected_outcome",
                    "family",
                    "id",
                    "kind",
                    "retained_path",
                    "retained_sha256"
                ],
                "the generated case shape changed"
            );
        }
    }
}
