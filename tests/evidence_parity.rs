// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Native Rust coverage for evidence classification and canonical identity.

use std::collections::BTreeSet;

use engineering_assurance::evidence::{
    AvailabilityState, EvidenceEnvelope, GoverningVersions, OperatorObservation, ProducerAttempt,
    VersionIdentity, classify_producer, validate_state_labels,
};
use ix_trace_rs::trace;
use serde_json::{Value, json};

const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn identity(name: &str, version: &str) -> VersionIdentity {
    VersionIdentity {
        name: name.to_owned(),
        version: version.to_owned(),
        digest: DIGEST.to_owned(),
    }
}

fn governing() -> GoverningVersions {
    GoverningVersions {
        module: identity("engineering-assurance", "1.2.3"),
        plugin: identity("engineering-assurance-plugin", "1.2.3"),
        skill: identity("assurance-onboarding", "1.2.3"),
        workflow: identity("assurance-intake", "1.2.3"),
        quire: identity("quire", "1.2.3"),
        quoin: identity("quoin", "1.2.3"),
        ix_flow: identity("ix-flow", "1.2.3"),
        schema: identity("producer-output-v1", "1.2.3"),
        producer: identity("fictional-producer", "1.2.3"),
    }
}

fn observation(
    outcome: &str,
    exit_code: Option<i64>,
    diagnostic: Option<&str>,
) -> OperatorObservation {
    OperatorObservation {
        command: vec!["fictional-producer".to_owned(), "--json".to_owned()],
        elapsed_ms: 17,
        exit_code,
        outcome: outcome.to_owned(),
        diagnostic_category: diagnostic.map(str::to_owned),
    }
}

fn observed() -> ProducerAttempt {
    ProducerAttempt {
        producer_id: "fictional-producer".to_owned(),
        applicable: true,
        invoked: true,
        observation: observation("succeeded", Some(0), None),
        governing: Some(governing()),
        output: Some(json!({
            "valid": true,
            "count": 3,
            "numeric": [
                1.0, -0.0, 1e-10, 1e-9, 1e-8, 1e-7, 1e-6, 1e-5, 1.2345e-5,
                -1.2345e-5, 1e-4, 1e15, 1e16, 1e17, 1e20, 1e21, 5e-324,
                1.797_693_134_862_315_7e308
            ],
            "text": "é/\u{2028}\n\u{0001}",
            "nested": {"z": "é", "a": [2, 1]}
        })),
        output_valid: true,
        next_action: None,
        owner: None,
        boundary_rationale: None,
        quoin_reference: Some("ix://agent-ix/quoin/EvidenceRecord-001".to_owned()),
    }
}

fn changed(source: &ProducerAttempt, change: impl FnOnce(&mut ProducerAttempt)) -> ProducerAttempt {
    let mut result = source.clone();
    change(&mut result);
    result
}

fn with_producer_version(source: &ProducerAttempt, version: &str) -> ProducerAttempt {
    changed(source, |case| {
        version.clone_into(
            &mut case
                .governing
                .as_mut()
                .expect("fixture has governing versions")
                .producer
                .version,
        );
    })
}

fn generated_identity_corpus() -> Vec<String> {
    let mut raw_values = vec![
        include_str!("../corpus/compatibility/producers/producer-code-graph.json").to_owned(),
        include_str!("../corpus/compatibility/producers/producer-measurement.json").to_owned(),
        include_str!("../corpus/compatibility/producers/producer-contract-conformance.json")
            .to_owned(),
        r#"{"v":18446744073709551615}"#.to_owned(),
        r#"{"v":18446744073709551616}"#.to_owned(),
        r#"{"v":-9223372036854775808}"#.to_owned(),
        r#"{"v":-9223372036854775809}"#.to_owned(),
        r#"{"v":1234567890123456789012345}"#.to_owned(),
        r#"{"v":-1234567890123456789012345}"#.to_owned(),
        r#"{"v":-0}"#.to_owned(),
        r#"{"v":1.2300}"#.to_owned(),
        r#"{"v":1E+09}"#.to_owned(),
        r#"{"v":-0.0}"#.to_owned(),
        r#"{"nested":[{"v":99999999999999999999999999999999999999}]}"#.to_owned(),
    ];
    for width in 20..=80 {
        let digits = (0..width)
            .map(|index| {
                char::from(b'1' + u8::try_from(index % 9).expect("digit index is bounded"))
            })
            .collect::<String>();
        raw_values.push(format!(r#"{{"v":{digits}}}"#));
        raw_values.push(format!(r#"{{"v":-{digits}}}"#));
    }
    raw_values
}

fn parity_cases() -> Vec<ProducerAttempt> {
    let base = observed();
    let not_computed = changed(&base, |case| {
        case.invoked = false;
        case.observation = observation("not-run", None, None);
        case.governing = None;
        case.output = None;
        case.output_valid = false;
        case.owner = Some("measurement-owner".to_owned());
        case.quoin_reference = None;
    });
    let not_applicable = changed(&not_computed, |case| {
        case.applicable = false;
        case.owner = None;
        case.boundary_rationale = Some("outside the selected service boundary".to_owned());
    });

    vec![
        base.clone(),
        changed(&base, |case| {
            case.observation = observation("failed", Some(127), Some("executable-not-found"));
            case.output = None;
            case.output_valid = false;
            case.governing = None;
            case.quoin_reference = None;
        }),
        not_computed.clone(),
        not_applicable.clone(),
        changed(&base, |case| case.output_valid = false),
        with_producer_version(&base, "latest"),
        changed(&base, |case| case.quoin_reference = None),
        changed(&base, |case| " ".clone_into(&mut case.producer_id)),
        changed(&base, |case| case.observation.command = Vec::new()),
        changed(&base, |case| case.observation.elapsed_ms = -1),
        changed(&base, |case| {
            case.observation = observation("foreign", None, None);
        }),
        changed(&base, |case| {
            case.observation = observation("not-run", Some(0), None);
        }),
        changed(&base, |case| {
            case.observation = observation("succeeded", None, None);
        }),
        changed(&base, |case| {
            case.observation = observation("failed", Some(1), None);
        }),
        changed(&not_applicable, |case| case.boundary_rationale = None),
        changed(&not_computed, |case| case.owner = None),
        changed(&base, |case| {
            case.observation = observation("not-run", None, None);
        }),
        changed(&base, |case| case.output = None),
        changed(&base, |case| case.output = Some(json!([]))),
        changed(&base, |case| case.governing = None),
        changed(&base, |case| {
            case.governing
                .as_mut()
                .expect("fixture has governing versions")
                .producer = VersionIdentity {
                name: " ".to_owned(),
                version: " ".to_owned(),
                digest: "bad".to_owned(),
            };
        }),
        changed(&base, |case| {
            case.quoin_reference = Some("ix://agent-ix/not-quoin/EvidenceRecord-001".to_owned());
        }),
        with_producer_version(&base, "1.2.3+linux-x86_64"),
        with_producer_version(&base, "1.x"),
        with_producer_version(&base, "Straße"),
        changed(&base, |case| {
            case.output = Some(
                serde_json::from_str(r#"{"v":1e309}"#)
                    .expect("overflowing finite-number syntax must remain inspectable"),
            );
        }),
    ]
}

#[trace("TC-100", "FR-015-AC-1")]
#[test]
fn tc_100_evidence_classification_preserves_accepted_state_fixtures() {
    let results = parity_cases()[..4]
        .iter()
        .map(classify_producer)
        .collect::<Vec<_>>();
    assert!(results.iter().all(EvidenceEnvelope::is_valid));
    assert_eq!(
        results
            .iter()
            .map(|result| result.availability)
            .collect::<Vec<_>>(),
        vec![
            Some(AvailabilityState::Observed),
            Some(AvailabilityState::Unavailable),
            Some(AvailabilityState::NotComputed),
            Some(AvailabilityState::NotApplicable),
        ]
    );
    assert_eq!(
        results[0].output_digest.as_deref(),
        Some("013ad71438e02c083ba28636ab02fae1eb6196ecfd773ddbe3b1d7a0b865071f")
    );
}

#[trace("TC-102", "FR-015-AC-2")]
#[test]
fn tc_102_evidence_availability_states_remain_distinct() {
    let states = parity_cases()[..4]
        .iter()
        .map(classify_producer)
        .map(|result| result.availability.expect("first four cases are valid"))
        .collect::<Vec<_>>();
    assert_eq!(
        states,
        [
            AvailabilityState::Observed,
            AvailabilityState::Unavailable,
            AvailabilityState::NotComputed,
            AvailabilityState::NotApplicable,
        ]
    );
    for labels in [
        vec![],
        vec!["observed", "observed"],
        vec!["observed", "unavailable"],
        vec!["unknown"],
    ] {
        let error = validate_state_labels(labels).expect_err("invalid labels must be refused");
        assert!(!error.message().is_empty());
    }
    for state in ["observed", "unavailable", "not_computed", "not_applicable"] {
        assert_eq!(
            validate_state_labels([state])
                .expect("one known state")
                .as_str(),
            state
        );
    }
}

#[trace("TC-103", "FR-015-AC-3")]
#[test]
fn tc_103_evidence_malformed_and_missing_provenance_fail_explicitly() {
    let results = parity_cases()[4..22]
        .iter()
        .map(classify_producer)
        .collect::<Vec<_>>();
    assert_eq!(results[0].validation_errors, ["producer-output-malformed"]);
    assert_eq!(
        results[1].validation_errors,
        ["producer:identity-version-mutable"]
    );
    assert_eq!(results[2].validation_errors, ["quoin-handoff-missing"]);
    assert_eq!(results[3].validation_errors, ["producer-id-missing"]);
    assert!(
        results
            .iter()
            .all(|result| !result.is_valid() && result.availability.is_none())
    );
}

#[trace("TC-100", "FR-015-AC-1")]
#[test]
fn tc_100_accepted_and_generated_json_values_match_canonical_identity_fixtures() {
    let raw_values = generated_identity_corpus();
    for raw in &raw_values {
        let output: Value = serde_json::from_str(raw).expect("identity fixture must be valid JSON");
        let attempt = changed(&observed(), |case| case.output = Some(output));
        let result = classify_producer(&attempt);
        assert!(result.is_valid(), "valid identity input was refused: {raw}");
    }

    for (raw, expected_digest) in [
        (
            r#"{"v":18446744073709551615}"#,
            "846afd99dea73da7038d6689d7f3295477e36ca591b3f7e5d2a7c10aef8d601a",
        ),
        (
            r#"{"v":18446744073709551616}"#,
            "f429894757452b732330279b4314a7ff3140fba29799935078a0328d544343c3",
        ),
        (
            r#"{"v":-9223372036854775808}"#,
            "9e52feb18cdc447ae70062ecd2e763c4d1cfac1607ac1b1499738f28cae639fb",
        ),
        (
            r#"{"v":-9223372036854775809}"#,
            "17b3cb5dec17af647887b3ce0d336fbbe9c7335c4d2ab86c156b50a729ee0c9f",
        ),
        (
            r#"{"v":1234567890123456789012345}"#,
            "afbf326118eedf9766b39b6ae455697437642bfbfee0a9fbd9692a05e3162001",
        ),
        (
            r#"{"v":-1234567890123456789012345}"#,
            "38c88e8f9ceff3d3095b7104bdc569b88b4511cabeb6a49966eb7e2b804589fd",
        ),
        (
            r#"{"v":-0}"#,
            "ec4f95abcb4e2e3dbe856c3eb2f81995eacb3823d40aa2e78ed4c5e1798f664d",
        ),
        (
            r#"{"v":1.2300}"#,
            "a3b18e44bc2c41c0d44c6a475749f2127d38b39d2448ff95653a1fd4407ee86c",
        ),
        (
            r#"{"v":1E+09}"#,
            "416d2d40c79a32ea746f530b4857832aa2ee4b57203cfcf4bd8b0cea37b27818",
        ),
        (
            r#"{"v":-0.0}"#,
            "3804efdcecdab4b071c214185d5593cb6c3f0d386b6dd057bacd0e3740459dff",
        ),
        (
            r#"{"nested":[{"v":99999999999999999999999999999999999999}]}"#,
            "3e0e8486a2c981ce467e866ff023449d2b5734eb68e508d3e538942ec183631c",
        ),
    ] {
        let output = serde_json::from_str(raw).expect("canonical fixture must parse");
        let result = classify_producer(&changed(&observed(), |case| case.output = Some(output)));
        assert_eq!(
            result.output_digest.as_deref(),
            Some(expected_digest),
            "digest drift for {raw}"
        );
    }
}

#[trace("TC-103", "FR-015-AC-3")]
#[test]
fn tc_103_non_finite_and_overflowing_numeric_identity_inputs_are_refused() {
    for token in ["NaN", "Infinity", "-Infinity"] {
        let raw = format!(r#"{{"v":{token}}}"#);
        assert!(
            serde_json::from_str::<Value>(&raw).is_err(),
            "non-RFC 8259 token must fail at the JSON boundary: {token}"
        );
    }

    for raw in [r#"{"v":1e309}"#, r#"{"v":-1e309}"#] {
        let output: Value = serde_json::from_str(raw)
            .expect("arbitrary-precision parsing must retain finite JSON number syntax");
        let retained_output = output.clone();
        let attempt = changed(&observed(), |case| case.output = Some(output));
        let result = classify_producer(&attempt);
        assert_eq!(
            result.validation_errors,
            ["producer-output-number-non-finite"]
        );
        assert_eq!(result.output_digest, None);
        assert!(!result.is_valid());
        assert_eq!(attempt.output, Some(retained_output));
    }
}

#[trace("TC-103", "FR-015-AC-3")]
#[test]
fn tc_103_version_identity_rejects_actual_mutability_without_rejecting_metadata() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../engineering_assurance/fixtures/evidence-version-policy.json"
    ))
    .expect("version-policy fixture must be valid JSON");
    assert_eq!(
        fixture["schema_version"],
        "engineering-assurance.evidence-version-policy/v1"
    );
    let mut classes = BTreeSet::new();
    for case in fixture["cases"]
        .as_array()
        .expect("version-policy cases must be an array")
    {
        let class = case["class"]
            .as_str()
            .expect("fixture class must be a string");
        classes.insert(class);
        let version = case["version"]
            .as_str()
            .expect("fixture version must be a string");
        let accepted_errors = case["accepted_errors"]
            .as_array()
            .expect("accepted errors must be an array")
            .iter()
            .map(|error| error.as_str().expect("fixture error must be a string"))
            .collect::<Vec<_>>();
        let prior_errors = case["prior_errors"]
            .as_array()
            .expect("prior errors must be an array")
            .iter()
            .map(|error| error.as_str().expect("fixture error must be a string"))
            .collect::<Vec<_>>();
        assert_eq!(
            prior_errors != accepted_errors,
            case["policy_changed"]
                .as_bool()
                .expect("policy_changed must be a boolean"),
            "policy-change marker disagrees for {version}"
        );
        assert_eq!(
            identity("producer", version).errors(),
            accepted_errors,
            "accepted version policy drifted for {version}"
        );
    }
    assert_eq!(
        classes,
        BTreeSet::from([
            "ascii-control",
            "ascii-space",
            "ascii-space-interior",
            "empty-token",
            "immutable-exact",
            "immutable-x-metadata",
            "immutable-x-metadata-uppercase",
            "mutable-alias",
            "non-ascii",
            "range-operator",
            "wildcard-component",
        ]),
        "version-policy fixture omitted or invented a governed input class"
    );

    for version in ["1.x", "^1.2.3", "latest", "Straße", "1.2.3 beta"] {
        assert!(!identity("producer", version).errors().is_empty());
    }
    assert_eq!(
        identity("producer", "Straße").errors(),
        ["identity-version-invalid-character"]
    );
}

#[trace("TC-100", "FR-015-AC-1")]
#[test]
fn tc_100_canonical_json_digest_fixture_is_stable() {
    let result = classify_producer(&observed());
    assert_eq!(
        result.output_digest.as_deref(),
        Some("013ad71438e02c083ba28636ab02fae1eb6196ecfd773ddbe3b1d7a0b865071f")
    );
    assert_eq!(
        Value::String("observed".to_owned()),
        serde_json::to_value(result.availability).expect("availability must serialize")
    );
}
