// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Requirement tests for the semantic contract, carried natively.
//!
//! Every criterion exercised here was previously carried only by
//! `tests/test_verification_semantics.py`. Retiring that suite without moving
//! these assertions would have left fifteen Test Matrix rows green and backed
//! by nothing, which is the exact failure the two previous rounds of this
//! campaign produced. Each test below therefore states the property it can
//! catch, not the capability it exercises.

use std::{collections::BTreeSet, fs, path::PathBuf};

use engineering_assurance::semantics::{
    Authority, SemanticConcept, SemanticError, SemanticErrorKind, SemanticFixture,
    validate_ownership_registry_bytes, validate_semantic_bundle, validate_semantic_fixture_bytes,
};
use ix_trace_rs::trace;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        root()
            .join("engineering_assurance/fixtures/verification-semantics")
            .join(name),
    )
    .expect("governed semantic fixture must be readable")
}

fn ownership_registry() -> serde_json::Value {
    serde_json::from_slice(
        &fs::read(
            root().join("engineering_assurance/contracts/verification-semantics-ownership-v1.json"),
        )
        .expect("ownership registry must be readable"),
    )
    .expect("ownership registry must parse")
}

fn canonical_fixture() -> SemanticFixture {
    validate_semantic_fixture_bytes(&fixture("canonical-references.json"))
        .expect("the canonical fixture must validate")
}

/// Apply one mutation to the packaged ownership registry and report the refusal.
fn refuse_mutated_registry(mutate: impl FnOnce(&mut serde_json::Value)) -> SemanticError {
    let mut value = ownership_registry();
    mutate(&mut value);
    validate_ownership_registry_bytes(&serde_json::to_vec(&value).expect("mutant must serialize"))
        .expect_err("the mutated registry must be refused")
}

/// Re-encode a reference bundle, apply one mutation to the raw JSON, and report
/// the refusal. Mutating the encoded form rather than the typed value is what
/// exercises the decoder's own refusals, which several of these properties
/// depend on rather than on post-decode validation.
fn refuse_mutated_fixture(mutate: impl FnOnce(&mut serde_json::Value)) -> SemanticError {
    let mut value: serde_json::Value =
        serde_json::from_slice(&fixture("canonical-references.json"))
            .expect("the canonical fixture must parse");
    mutate(&mut value);
    validate_semantic_fixture_bytes(&serde_json::to_vec(&value).expect("mutant must serialize"))
        .expect_err("the mutated fixture must be refused")
}

#[trace("TC-053", "US-005-AC-1")]
#[trace("TC-056", "FR-008-AC-1")]
#[test]
fn tc_053_ownership_registry_assigns_every_concept_to_exactly_one_authority() {
    // The expected allocation is restated here rather than read from the
    // library, so that a concept quietly reassigned in both the registry and
    // the library's own table still fails. A test that asks the library what it
    // expects and then checks the registry against that answer cannot catch a
    // coordinated change, which is the change most likely to happen by accident.
    let expected = [
        ("verification_definition", "quire"),
        ("verification_execution", "native_producer"),
        ("check_result", "native_producer"),
        ("evidence", "quoin"),
        ("measurement", "quoin"),
        ("diagnostic", "originating_producer"),
        ("report", "quoin"),
        ("human_decision", "ix_flow"),
    ];
    let registry = ownership_registry();
    let concepts = registry["concepts"]
        .as_array()
        .expect("the registry must declare a concept array");
    assert_eq!(
        concepts.len(),
        expected.len(),
        "the registry must name exactly the reviewed concept population"
    );
    for (concept, authority) in expected {
        let entry = concepts
            .iter()
            .find(|item| item["concept"] == concept)
            .unwrap_or_else(|| panic!("the registry must name {concept}"));
        assert_eq!(
            entry["authority"], authority,
            "{concept} is allocated to the wrong authority"
        );
    }
    assert_eq!(
        registry["non_executing"],
        serde_json::Value::Bool(true),
        "the registry must declare itself non-executing"
    );

    // The criterion is one authority *and one link direction* per concept. A
    // concept whose direction went blank would still have exactly one
    // authority, so the authority check above cannot stand in for this one.
    for entry in concepts {
        let direction = entry["link_direction"]
            .as_str()
            .expect("every concept must declare a link direction");
        assert!(
            !direction.is_empty(),
            "{} declares an empty link direction",
            entry["concept"]
        );
    }

    // A registry that simply drops a concept is the failure a shrinking
    // vocabulary produces, and it is the mutation that leaves every other
    // registry rule satisfied.
    assert_eq!(
        refuse_mutated_registry(|value| {
            value["concepts"]
                .as_array_mut()
                .expect("the registry must declare a concept array")
                .pop();
        })
        .kind(),
        SemanticErrorKind::IncompleteOwnershipConceptSet
    );

    // The row names four registry refusals. Exercising only one of them under
    // this tag left the other three backed by a test carrying FR-015 tags,
    // which is how a row comes to describe work its own criterion does not
    // reach.
    assert_eq!(
        refuse_mutated_registry(|value| {
            value["concepts"][0]["authority"] = serde_json::json!("quoin");
        })
        .kind(),
        SemanticErrorKind::AuthorityMismatch
    );
    assert_eq!(
        refuse_mutated_registry(|value| {
            value["concepts"][1] = value["concepts"][0].clone();
        })
        .kind(),
        SemanticErrorKind::DuplicateOwnershipConcept
    );
    assert_eq!(
        refuse_mutated_registry(|value| {
            value["non_executing"] = serde_json::json!(false);
        })
        .kind(),
        SemanticErrorKind::InvalidOwnershipBoundary
    );
}

#[trace("TC-052", "StR-002-VC-1")]
#[trace("TC-057", "FR-008-AC-2")]
#[test]
fn tc_052_canonical_fixture_keeps_every_identity_distinct_and_externally_linked() {
    let accepted = canonical_fixture();
    let identities: BTreeSet<&str> = accepted
        .references
        .iter()
        .map(|reference| reference.semantic_id.as_str())
        .collect();
    assert_eq!(
        identities.len(),
        accepted.references.len(),
        "two semantic references share an identity"
    );

    // Without this the fixture could shrink to a single concept and every
    // cross-concept property below would still pass, having been asked about
    // nothing. The fixture is the population for TC-052, so the population is
    // asserted rather than assumed.
    let concepts: BTreeSet<SemanticConcept> = accepted
        .references
        .iter()
        .map(|reference| reference.concept)
        .collect();
    assert_eq!(
        concepts.len(),
        8,
        "the cross-component fixture must exercise every semantic concept"
    );

    // A measurement plan is an Engineering Assurance definition with its own
    // identity, so `measurement_plan` is deliberately not resolved inside the
    // bundle. If it were reclassified as an internal link, the bundle would
    // start demanding a reference that by design lives elsewhere.
    let measurement = accepted
        .references
        .iter()
        .find(|reference| reference.concept == SemanticConcept::Measurement)
        .expect("the fixture must carry a measurement reference");
    assert_eq!(
        measurement.links.measurement_plan.as_deref(),
        Some("MP-001"),
        "the external measurement-plan link must survive validation unresolved"
    );
    validate_semantic_bundle(&accepted.references)
        .expect("an external plan link must not be demanded inside the bundle");
}

#[trace("TC-058", "FR-008-AC-3")]
#[test]
fn tc_058_absent_confused_self_and_omitted_links_each_refuse_distinctly() {
    let accepted = canonical_fixture();

    // A link whose target is simply not in the bundle is the plainest form of
    // this failure, and the one a dropped reference produces.
    let mut absent = accepted.clone();
    absent.references.remove(0);
    assert_eq!(
        absent
            .validate()
            .expect_err("a link to an absent reference must refuse")
            .kind(),
        SemanticErrorKind::MissingReference
    );

    let mut confused = accepted.clone();
    confused.references[3].links.result = Some("sem:report-001".to_owned());
    assert_eq!(
        confused
            .validate()
            .expect_err("a link to the wrong concept must refuse")
            .kind(),
        SemanticErrorKind::ReferenceConceptMismatch
    );

    // A reference that links to itself satisfies every "the target exists"
    // check, so only an explicit self-reference refusal catches it. Nothing
    // exercised this refusal before the Python lane was retired.
    let mut reflexive = accepted.clone();
    reflexive.references[1].links.definition = Some(reflexive.references[1].semantic_id.clone());
    assert_eq!(
        reflexive
            .validate()
            .expect_err("a self-link must refuse")
            .kind(),
        SemanticErrorKind::SelfReference
    );

    // A check result without its execution link is provenance-free: it records
    // an outcome with no statement of what produced it. Omission, unlike
    // misdirection, leaves no wrong value behind to notice.
    let mut omitted = accepted;
    omitted.references[2].links.execution = None;
    assert_eq!(
        omitted
            .validate()
            .expect_err("an omitted required link must refuse")
            .kind(),
        SemanticErrorKind::MissingRequiredLink
    );
}

#[trace("TC-060", "FR-009-AC-1")]
#[trace("TC-062", "FR-009-AC-3")]
#[test]
fn tc_060_producer_tuple_is_complete_over_a_non_empty_population() {
    let expected_fields = ["identity", "version", "environment", "definition_version"];
    let accepted = canonical_fixture();
    for reference in &accepted.references {
        let Some(producer) = &reference.producer else {
            continue;
        };
        let encoded = serde_json::to_value(producer).expect("producer must serialize");
        let observed: BTreeSet<&str> = encoded
            .as_object()
            .expect("a producer tuple must be an object")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            observed,
            expected_fields.into_iter().collect::<BTreeSet<_>>(),
            "{} carries an incomplete producer tuple",
            reference.semantic_id
        );
    }

    // A producer-owned concept with no tuple at all is what the row calls an
    // absent tuple. The loop above skips `None` producers by construction, so
    // without this the row's plainest case would be exercised nowhere under its
    // own tag.
    let mut stripped = accepted;
    stripped.references[1].producer = None;
    assert_eq!(
        stripped
            .validate()
            .expect_err("a producer-owned concept without its tuple must refuse")
            .kind(),
        SemanticErrorKind::MissingProducer
    );

    // A tuple missing one field is refused by the decoder, before any semantic
    // rule runs. That is the boundary the criterion depends on: an incomplete
    // tuple must never become a typed value in the first place.
    assert_eq!(
        refuse_mutated_fixture(|value| {
            value["references"][1]["producer"]
                .as_object_mut()
                .expect("a producer tuple must be an object")
                .remove("version");
        })
        .kind(),
        SemanticErrorKind::InvalidInputEncoding
    );

    // An empty environment decodes cleanly and is refused semantically: it
    // records that an execution had an environment while stating nothing about
    // it, which is indistinguishable from provenance that was never captured.
    assert_eq!(
        refuse_mutated_fixture(|value| {
            value["references"][1]["producer"]["environment"] = serde_json::json!({});
        })
        .kind(),
        SemanticErrorKind::EmptyProducerEnvironment
    );
}

#[trace("TC-061", "FR-009-AC-2")]
#[test]
fn tc_061_every_declared_non_success_state_survives_validation() {
    let states: Vec<String> =
        serde_json::from_slice(&fixture("non-success-states.json")).expect("states must parse");

    // The loop below passes over an empty list having validated nothing, which
    // is how a state vocabulary could silently collapse to success while this
    // row stayed green. Naming three states the set must contain is what makes
    // the loop's success mean something.
    for required in ["failed", "skipped", "unreadable"] {
        assert!(
            states.iter().any(|state| state == required),
            "the declared non-success vocabulary no longer names {required}"
        );
    }

    let accepted = canonical_fixture();
    let template = serde_json::to_value(&accepted.references[2]).expect("reference must serialize");
    for state in states {
        let mut candidate = template.clone();
        candidate["state"] = serde_json::Value::String(state.clone());
        let parsed: engineering_assurance::semantics::SemanticReference =
            serde_json::from_value(candidate).expect("a declared state must decode");
        parsed
            .validate()
            .unwrap_or_else(|error| panic!("state {state} failed: {error}"));
        assert_eq!(
            serde_json::to_value(parsed).expect("state must serialize")["state"],
            state,
            "state {state} did not survive the round trip unchanged"
        );
    }
}

#[trace("TC-057", "FR-008-AC-2")]
#[trace("TC-062", "FR-009-AC-3")]
#[test]
fn tc_057_authority_confusion_and_unknown_source_versions_refuse() {
    // Reassigning one concept's authority is the change that would let a
    // producer mint a record another component owns.
    let mut confused = canonical_fixture();
    confused.references[0].authority = Authority::Quoin;
    assert_eq!(
        confused
            .validate()
            .expect_err("a reassigned authority must refuse")
            .kind(),
        SemanticErrorKind::AuthorityMismatch
    );

    // A fixture's premises state the exact source versions it was written
    // against. A reference whose version drifts away from them must fail rather
    // than be interpreted under premises that no longer describe it.
    assert_eq!(
        refuse_mutated_fixture(|value| {
            value["references"][0]["source"]["schema_version"] = serde_json::json!("99");
        })
        .kind(),
        SemanticErrorKind::SourcePremiseMismatch
    );

    // A repeated premise makes the premise set smaller than it reads, so a
    // reference could satisfy it by accident.
    assert_eq!(
        refuse_mutated_fixture(|value| {
            let premises = value["source_version_premises"]
                .as_array_mut()
                .expect("premises must be an array");
            let first = premises[0].clone();
            premises.push(first);
        })
        .kind(),
        SemanticErrorKind::DuplicateSourcePremise
    );
}

/// Compile one packaged schema and refuse a schema this repository ships but
/// cannot compile, which would otherwise make every check against it vacuous.
fn packaged_validator(name: &str) -> jsonschema::Validator {
    let raw = fs::read(root().join("engineering_assurance/schemas").join(name))
        .unwrap_or_else(|error| panic!("packaged schema {name} must be readable: {error}"));
    let schema: serde_json::Value = serde_json::from_slice(&raw)
        .unwrap_or_else(|error| panic!("packaged schema {name} must be JSON: {error}"));
    jsonschema::meta::options()
        .validate(&schema)
        .unwrap_or_else(|error| panic!("packaged schema {name} is not a valid schema: {error}"));
    jsonschema::options()
        .offline()
        .should_validate_formats(true)
        .build(&schema)
        .unwrap_or_else(|error| panic!("packaged schema {name} must compile: {error}"))
}

fn assert_conforms(validator: &jsonschema::Validator, instance: &serde_json::Value, what: &str) {
    if let Err(error) = validator.validate(instance) {
        panic!("{what} no longer conforms to the schema this repository publishes: {error}");
    }
}

#[trace("TC-052", "StR-002-VC-1")]
#[trace("TC-063", "FR-009-AC-4")]
#[trace("TC-068", "NFR-004-AC-1")]
#[test]
fn tc_068_published_schemas_still_describe_the_values_this_library_produces() {
    // The retired Python validated every instance against these packaged
    // schemas before it applied a single semantic rule. The Rust port replaced
    // that with typed decoding, which is stricter about structure but says
    // nothing about the schema files — and those files are still shipped by
    // `setup.cfg`, so they remain this repository's published description of
    // these records. Without
    // this test the two can drift apart indefinitely, and the first party to
    // notice would be an external consumer reading the published contract.
    let fixture_schema = packaged_validator("verification-semantics-fixture-v1.schema.json");
    let canonical: serde_json::Value =
        serde_json::from_slice(&fixture("canonical-references.json"))
            .expect("the canonical fixture must parse");
    assert_conforms(
        &fixture_schema,
        &canonical,
        "the canonical semantic fixture",
    );

    // Each reference is also published on its own, so it is checked on its own
    // rather than only as a member of the bundle.
    let reference_schema = packaged_validator("semantic-reference-v1.schema.json");
    let references = canonical["references"]
        .as_array()
        .expect("the fixture must carry a reference array");
    assert_eq!(
        references.len(),
        8,
        "the per-reference schema check read the wrong population"
    );
    for reference in references {
        assert_conforms(
            &reference_schema,
            reference,
            "a canonical semantic reference",
        );
    }

    let ownership_schema = packaged_validator("verification-semantics-ownership-v1.schema.json");
    assert_conforms(
        &ownership_schema,
        &ownership_registry(),
        "the ownership registry",
    );
}

#[trace("TC-063", "FR-009-AC-4")]
#[test]
fn tc_063_a_reference_must_cite_an_absolute_field_path_and_a_non_empty_record() {
    // A source citation that is relative, or names no record, points at nothing.
    for field_path in ["", "relative/path", "state"] {
        let error = refuse_mutated_fixture(|value| {
            value["references"][0]["source"]["field_path"] = serde_json::json!(field_path);
        });
        assert_eq!(
            error.kind(),
            SemanticErrorKind::InvalidSourcePath,
            "field_path {field_path:?}"
        );
    }
    let error = refuse_mutated_fixture(|value| {
        value["references"][0]["source"]["record_id"] = serde_json::json!("");
    });
    assert_eq!(error.kind(), SemanticErrorKind::InvalidIdentity);
}
