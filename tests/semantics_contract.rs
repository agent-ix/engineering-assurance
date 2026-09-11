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

use std::{collections::BTreeSet, fmt::Write as _, fs, path::PathBuf};

use engineering_assurance::semantics::{
    Authority, Pgm01Outcome, ReportProjection, SemanticConcept, SemanticError, SemanticErrorKind,
    SemanticFixture, map_pgm01_bytes, validate_ownership_registry_bytes, validate_semantic_bundle,
    validate_semantic_fixture_bytes,
};
use ix_trace_rs::trace;
use sha2::{Digest, Sha256};

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

    // A registry that simply drops a concept is the failure a shrinking
    // vocabulary produces, and it is the one registry mutation no other test
    // exercises: the authority, duplicate, and non-executing mutants all leave
    // the population intact.
    let mut shrunk = registry;
    shrunk["concepts"]
        .as_array_mut()
        .expect("the registry must declare a concept array")
        .pop();
    assert_eq!(
        validate_ownership_registry_bytes(
            &serde_json::to_vec(&shrunk).expect("mutant must serialize")
        )
        .expect_err("an incomplete registry must be refused")
        .kind(),
        SemanticErrorKind::IncompleteOwnershipConceptSet
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
    let expected_fields = [
        "identity",
        "version",
        "configuration_digest",
        "source_revision",
        "environment",
        "definition_version",
    ];
    let accepted = canonical_fixture();
    let mut audited = 0_usize;
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
        audited += 1;
    }

    // A fixture with no producer-owned reference would satisfy the loop above
    // having compared nothing. Five of the eight concepts are producer-derived,
    // so five is the floor this criterion is entitled to.
    assert_eq!(
        audited, 5,
        "the producer-tuple audit read the wrong population"
    );

    // A tuple missing one field is refused by the decoder, before any semantic
    // rule runs. That is the boundary the criterion depends on: an incomplete
    // tuple must never become a typed value in the first place.
    assert_eq!(
        refuse_mutated_fixture(|value| {
            value["references"][1]["producer"]
                .as_object_mut()
                .expect("a producer tuple must be an object")
                .remove("configuration_digest");
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
    // row stayed green. Naming three states the set must contain, and a floor
    // on its size, is what makes the loop's success mean something.
    assert!(
        states.len() >= 17,
        "the declared non-success vocabulary lost states"
    );
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

#[trace("TC-055", "US-005-AC-3")]
#[trace("TC-063", "FR-009-AC-4")]
#[trace("TC-064", "FR-010-AC-1")]
#[test]
fn tc_064_pgm01_mapping_is_traceable_lossy_and_leaves_its_source_unchanged() {
    for name in ["pgm01-v1.json", "pgm01-v2.json"] {
        let before = fixture(name);
        let view = map_pgm01_bytes(&before, None).expect("the governed fixture must map");
        let after = fixture(name);
        assert_eq!(before, after, "{name} changed while being mapped");

        // The digest is recomputed here from the bytes on disk rather than read
        // back out of the view, so the assertion does not ask the mapper to
        // confirm its own arithmetic.
        let mut hasher = Sha256::new();
        hasher.update(&before);
        let mut expected = String::with_capacity(64);
        for byte in hasher.finalize() {
            write!(expected, "{byte:02x}").expect("a String write cannot fail");
        }
        assert_eq!(
            view.source_digest, expected,
            "{name} minted a foreign identity"
        );

        // A historical record read as fully compatible would be a claim that
        // nothing was lost, which is the opposite of what these views record.
        assert_eq!(
            view.outcome,
            Pgm01Outcome::Lossy,
            "{name} was read as lossless"
        );
        assert!(
            !view.mappings.is_empty(),
            "{name} projected no field at all"
        );
        assert!(
            !view.unmapped_fields.is_empty(),
            "{name} declared no unmapped field"
        );
        assert!(
            !view.limitations.is_empty(),
            "{name} declared no compatibility limitation"
        );
        for mapping in &view.mappings {
            assert!(
                mapping.source_path.starts_with('/'),
                "{name} projected {} without an absolute source path",
                mapping.target_field
            );
            assert!(
                !mapping.target_field.is_empty(),
                "{name} projected a value into no field"
            );
        }
        for unmapped in &view.unmapped_fields {
            assert!(
                unmapped.source_path.starts_with('/'),
                "{name} recorded an unmapped field with no absolute source path"
            );
            assert!(
                !unmapped.reason.is_empty(),
                "{name} recorded an unmapped field with no stated reason"
            );
        }
    }
}

/// Collect the values a view projected from one source path.
fn projected(
    view: &engineering_assurance::semantics::Pgm01View,
    path: &str,
) -> Vec<serde_json::Value> {
    view.mappings
        .iter()
        .filter(|mapping| mapping.source_path == path)
        .map(|mapping| mapping.value.clone())
        .collect()
}

#[trace("TC-065", "FR-010-AC-2")]
#[test]
fn tc_065_pgm01_v1_preserves_identity_results_outputs_and_declared_gaps() {
    let view = map_pgm01_bytes(&fixture("pgm01-v1.json"), None).expect("v1 fixture must map");
    assert_eq!(
        projected(&view, "/subjectRevision"),
        [serde_json::json!("b".repeat(40))]
    );
    assert_eq!(
        projected(&view, "/collector/implementation"),
        [serde_json::json!("fictional collector")]
    );

    // A v1 record spells its states in the historical vocabulary. These two
    // assert that the translation lands on the retained spelling rather than on
    // the source token, and that a skipped check is not read as a pass.
    assert_eq!(
        projected(&view, "/checks/0/status"),
        [serde_json::json!("passed")]
    );
    assert_eq!(
        projected(&view, "/checks/1/status"),
        [serde_json::json!("skipped")]
    );
    assert_eq!(
        projected(&view, "/outputs/0"),
        [serde_json::json!("result.json")]
    );

    // The identities v1 never recorded must stay declared as absent. Dropping
    // one of these from the view would let a reader conclude the record carried
    // a producer version or a configuration digest that it never had.
    let unmapped: BTreeSet<&str> = view
        .unmapped_fields
        .iter()
        .map(|item| item.source_path.as_str())
        .collect();
    for required in ["/collector/version", "/configurationDigest", "/decision"] {
        assert!(
            unmapped.contains(required),
            "v1 stopped declaring {required} as unrecorded"
        );
    }
}

#[trace("TC-065", "FR-010-AC-2")]
#[test]
fn tc_065_pgm01_v2_preserves_producer_configuration_definition_and_outputs() {
    let view = map_pgm01_bytes(&fixture("pgm01-v2.json"), None).expect("v2 fixture must map");
    assert_eq!(
        projected(&view, "/collector/id"),
        [serde_json::json!("fictional-collector")]
    );
    assert_eq!(
        projected(&view, "/collector/version"),
        [serde_json::json!("1.0.0")]
    );
    assert_eq!(
        projected(&view, "/overallStatus"),
        [serde_json::json!("failed")]
    );

    // Three digests in this record mean three different things. Landing any of
    // them in the wrong target field would silently re-attribute a parameter
    // digest as a producer identity, or a profile digest as configuration.
    let target = |path: &str| -> (String, String) {
        let mapping = view
            .mappings
            .iter()
            .find(|mapping| mapping.source_path == path)
            .unwrap_or_else(|| panic!("v2 must project {path}"));
        (
            mapping.target_concept.to_string(),
            mapping.target_field.clone(),
        )
    };
    assert_eq!(
        target("/collector/sha256"),
        (
            "verification_execution".to_owned(),
            "producer.executable_digest".to_owned()
        )
    );
    assert_eq!(
        target("/parameters/sha256"),
        (
            "verification_execution".to_owned(),
            "producer.configuration_digest".to_owned()
        )
    );
    assert_eq!(
        target("/profile/sha256"),
        (
            "verification_definition".to_owned(),
            "definition_version".to_owned()
        )
    );
    assert_eq!(
        target("/commands/0/stdout/sha256"),
        ("evidence".to_owned(), "retained_output.sha256".to_owned())
    );

    // Corroboration and intake status are the two v2 fields most likely to be
    // mistaken for a verdict. They stay declared as deliberately unimported.
    let unmapped: BTreeSet<&str> = view
        .unmapped_fields
        .iter()
        .map(|item| item.source_path.as_str())
        .collect();
    for required in ["/commands/*/corroboration", "/quoin/status"] {
        assert!(
            unmapped.contains(required),
            "v2 stopped declaring {required} as unimported"
        );
    }
}

#[trace("TC-066", "FR-010-AC-3")]
#[trace("TC-062", "FR-009-AC-3")]
#[test]
fn tc_066_invalid_expected_digest_and_non_integer_byte_counts_refuse() {
    // An unusable expected digest must fail the call rather than classify the
    // input. Classifying it would report an identity verdict that no identity
    // was actually compared against, which is worse than refusing.
    assert_eq!(
        map_pgm01_bytes(b"{}", Some("not-a-digest"))
            .expect_err("an invalid expected digest must refuse")
            .kind(),
        SemanticErrorKind::InvalidExpectedDigest
    );

    let mut record: serde_json::Value =
        serde_json::from_slice(&fixture("pgm01-v2.json")).expect("v2 fixture must parse");

    // A retained byte count that arrives as a string is the shape a legacy
    // exporter produces when it stringifies numbers. Accepting it would put a
    // string where every consumer expects a count.
    record["commands"][0]["stdout"]["bytes"] = serde_json::json!("64");
    assert_eq!(
        map_pgm01_bytes(
            &serde_json::to_vec(&record).expect("mutant must serialize"),
            None
        )
        .expect("a malformed record is a classified result")
        .outcome,
        Pgm01Outcome::Unreadable
    );

    // A negative count is arithmetically valid and semantically impossible.
    record["commands"][0]["stdout"]["bytes"] = serde_json::json!(-1);
    assert_eq!(
        map_pgm01_bytes(
            &serde_json::to_vec(&record).expect("mutant must serialize"),
            None
        )
        .expect("a malformed record is a classified result")
        .outcome,
        Pgm01Outcome::Unreadable
    );
}

#[trace("TC-054", "US-005-AC-2")]
#[trace("TC-067", "FR-010-AC-4")]
#[test]
fn tc_054_bounded_report_round_trips_and_declares_no_aggregate_verdict() {
    let raw = fixture("report-projection.json");
    let report: ReportProjection = serde_json::from_slice(&raw).expect("report must parse");
    let source: serde_json::Value = serde_json::from_slice(&raw).expect("report must parse");
    assert_eq!(
        serde_json::to_value(&report).expect("report must serialize"),
        source,
        "the bounded report did not survive its round trip unchanged"
    );

    // Rendering twice is the cheapest statement that the projection carries no
    // iteration-order or timestamp dependence, which a report used as evidence
    // cannot have.
    let first = report.render_json().expect("report JSON must render");
    assert_eq!(
        first,
        report.render_json().expect("report JSON must render")
    );
    assert!(
        first.ends_with('\n'),
        "the rendered JSON must end in one newline"
    );

    let markdown = report
        .render_markdown()
        .expect("report Markdown must render");
    for heading in [
        "## Claims",
        "## Evidence",
        "## Counterevidence",
        "## Gaps",
        "## Owner",
        "## Actions",
        "## Human decision reference",
    ] {
        assert!(
            markdown.contains(heading),
            "the rendered report omits {heading}"
        );
    }

    // The rendered view is where an aggregate verdict would most plausibly
    // reappear as prose, having been kept out of the typed record.
    let folded = markdown.to_lowercase();
    for forbidden in ["trust score", "overall verdict", "overall score"] {
        assert!(
            !folded.contains(forbidden),
            "the rendered report states a {forbidden}"
        );
    }

    // All four aggregate fields are refused, not just the one. Each of these is
    // a different way to smuggle a single summary judgement into a record whose
    // whole purpose is to keep claims, evidence and gaps separate.
    for forbidden in [
        "trust_score",
        "overall_score",
        "overall_verdict",
        "approved",
    ] {
        let mut extended: serde_json::Value =
            serde_json::from_slice(&raw).expect("report must parse");
        extended[forbidden] = serde_json::json!("passed");
        assert!(
            serde_json::from_value::<ReportProjection>(extended).is_err(),
            "the bounded report admitted {forbidden}"
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
