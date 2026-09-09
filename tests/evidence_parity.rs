// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Additive-migration parity with the retained Python evidence classifier.

use std::{
    io::Write,
    process::{Command, Stdio},
};

use engineering_assurance::evidence::{
    AvailabilityState, GoverningVersions, OperatorObservation, ProducerAttempt, VersionIdentity,
    classify_producer, validate_state_labels,
};
use ix_trace_rs::trace;
use serde_json::{Value, json};

const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

const PYTHON_DIGEST_REFERENCE: &str = r#"
import hashlib
import json
import sys

raw_values = json.load(sys.stdin)
digests = []
for raw in raw_values:
    value = json.loads(raw)
    canonical = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    digests.append(hashlib.sha256(canonical).hexdigest())
print(json.dumps(digests, separators=(",", ":")))
"#;

const PYTHON_REFERENCE: &str = r#"
import json
from dataclasses import asdict, replace
from engineering_assurance.evidence import (
    GoverningVersions, OperatorObservation, ProducerAttempt, VersionIdentity,
    classify_producer,
)

DIGEST = "a" * 64
def identity(name, version="1.2.3"):
    return VersionIdentity(name, version, DIGEST)
def governing():
    return GoverningVersions(
        module=identity("engineering-assurance"),
        plugin=identity("engineering-assurance-plugin"),
        skill=identity("assurance-onboarding"),
        workflow=identity("assurance-intake"),
        quire=identity("quire"), quoin=identity("quoin"),
        ix_flow=identity("ix-flow"), schema=identity("producer-output-v1"),
        producer=identity("fictional-producer"),
    )
def observation(outcome="succeeded", exit_code=0, diagnostic=None):
    return OperatorObservation(
        command=("fictional-producer", "--json"), elapsed_ms=17,
        exit_code=exit_code, outcome=outcome, diagnostic_category=diagnostic,
    )
def observed():
    return ProducerAttempt(
        producer_id="fictional-producer", applicable=True, invoked=True,
        observation=observation(), governing=governing(),
        output={"valid": True, "count": 3, "numeric": [1.0, -0.0, 1e-10, 1e-9, 1e-8, 1e-7, 1e-6, 1e-5, 1.2345e-5, -1.2345e-5, 1e-4, 1e15, 1e16, 1e17, 1e20, 1e21, 5e-324, 1.7976931348623157e308], "text": "é/\u2028\n\u0001", "nested": {"z": "é", "a": [2, 1]}},
        output_valid=True,
        quoin_reference="ix://agent-ix/quoin/EvidenceRecord-001",
    )

base = observed()
cases = [
    base,
    replace(base, observation=observation("failed", 127, "executable-not-found"), output=None, output_valid=False, governing=None, quoin_reference=None),
    ProducerAttempt("fictional-producer", True, False, observation("not-run", None), owner="measurement-owner"),
    ProducerAttempt("fictional-producer", False, False, observation("not-run", None), boundary_rationale="outside the selected service boundary"),
    replace(base, output_valid=False),
    replace(base, governing=replace(governing(), producer=identity("fictional-producer", "latest"))),
    replace(base, quoin_reference=None),
    replace(base, producer_id=" "),
    replace(base, observation=replace(observation(), command=())),
    replace(base, observation=replace(observation(), elapsed_ms=-1)),
    replace(base, observation=observation("foreign", None)),
    replace(base, observation=observation("not-run", 0)),
    replace(base, observation=observation("succeeded", None)),
    replace(base, observation=observation("failed", 1)),
    ProducerAttempt("fictional-producer", False, False, observation("not-run", None)),
    ProducerAttempt("fictional-producer", True, False, observation("not-run", None)),
    replace(base, observation=observation("not-run", None)),
    replace(base, output=None),
    replace(base, output=[]),
    replace(base, governing=None),
    replace(base, governing=replace(governing(), producer=VersionIdentity(" ", " ", "bad"))),
    replace(base, quoin_reference="ix://agent-ix/not-quoin/EvidenceRecord-001"),
    replace(base, governing=replace(governing(), producer=identity("fictional-producer", "1.2.3+linux-x86_64"))),
    replace(base, governing=replace(governing(), producer=identity("fictional-producer", "1.x"))),
    replace(base, governing=replace(governing(), producer=identity("fictional-producer", "Straße"))),
    replace(base, output={"v": float("inf")}),
]
print(json.dumps([asdict(classify_producer(case)) for case in cases], sort_keys=True, separators=(",", ":"), ensure_ascii=False))
"#;

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

fn retained_identity_digests(raw_values: &[String]) -> Vec<String> {
    let payload = serde_json::to_string(raw_values).expect("identity corpus must serialize");
    let mut child = Command::new("python3")
        .args(["-c", PYTHON_DIGEST_REFERENCE])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("retained Python identity reference must execute during additive migration");
    child
        .stdin
        .take()
        .expect("Python reference stdin must be piped")
        .write_all(payload.as_bytes())
        .expect("identity corpus must reach the retained Python reference");
    let python = child
        .wait_with_output()
        .expect("retained Python identity reference must complete");
    assert!(
        python.status.success(),
        "{}",
        String::from_utf8_lossy(&python.stderr)
    );
    serde_json::from_slice(&python.stdout).expect("retained identity digests must be JSON")
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
fn tc_100_evidence_classification_and_canonical_digest_match_retained_reference_bytes() {
    let results = parity_cases()
        .iter()
        .map(classify_producer)
        .collect::<Vec<_>>();
    let rust_value = serde_json::to_value(results).expect("typed results must serialize");
    let rust_bytes = serde_json::to_vec(&rust_value).expect("JSON values must serialize");

    let python = Command::new("python3")
        .args(["-c", PYTHON_REFERENCE])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("retained Python reference must execute during additive migration");
    assert!(
        python.status.success(),
        "{}",
        String::from_utf8_lossy(&python.stderr)
    );
    assert_eq!(
        rust_bytes,
        python.stdout.strip_suffix(b"\n").unwrap_or(&python.stdout)
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
fn tc_100_accepted_and_generated_json_values_match_retained_identity_digests() {
    let raw_values = generated_identity_corpus();
    let retained = retained_identity_digests(&raw_values);
    assert_eq!(retained.len(), raw_values.len());

    for (raw, expected_digest) in raw_values.iter().zip(retained) {
        let output: Value = serde_json::from_str(raw).expect("identity fixture must be valid JSON");
        let attempt = changed(&observed(), |case| case.output = Some(output));
        let result = classify_producer(&attempt);
        assert_eq!(
            result.output_digest.as_deref(),
            Some(expected_digest.as_str()),
            "identity divergence for {raw}"
        );
        assert!(result.is_valid(), "valid identity input was refused: {raw}");
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
    assert!(
        identity("producer", "1.2.3+linux-x86_64")
            .errors()
            .is_empty()
    );
    for version in ["1.x", "1.*", "^1.2.3", "latest", "Straße", "1.2.3 beta"] {
        assert!(
            !identity("producer", version).errors().is_empty(),
            "invalid or mutable version was accepted: {version}"
        );
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
