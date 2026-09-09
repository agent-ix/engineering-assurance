// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Requirement and differential tests for the Rust semantic/projection slice.

use std::{collections::BTreeMap, fs, path::PathBuf, process::Command};

use engineering_assurance::semantics::{
    Pgm01Outcome, ReportProjection, SemanticFixture, map_pgm01_bytes, render_generated_fixtures,
    validate_embedded_ownership_registry, validate_ownership_registry_bytes,
    validate_semantic_fixture_bytes,
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

#[trace("TC-100", "FR-015-AC-1")]
#[trace("TC-102", "FR-015-AC-2")]
#[test]
fn tc_100_semantic_fixture_and_non_success_states_are_complete() {
    validate_embedded_ownership_registry().expect("ownership registry must remain valid");
    let accepted = validate_semantic_fixture_bytes(&fixture("canonical-references.json"))
        .expect("canonical fixture must validate");
    assert_eq!(accepted.references.len(), 8);

    let states: Vec<String> =
        serde_json::from_slice(&fixture("non-success-states.json")).expect("states must parse");
    let encoded = serde_json::to_value(&accepted).expect("fixture must serialize");
    let mut reference = encoded["references"][2].clone();
    for state in states {
        reference["state"] = serde_json::Value::String(state.clone());
        let parsed: engineering_assurance::semantics::SemanticReference =
            serde_json::from_value(reference.clone()).expect("declared state must parse");
        parsed
            .validate()
            .unwrap_or_else(|error| panic!("state {state} failed: {error}"));
        assert_eq!(
            serde_json::to_value(parsed).expect("state must serialize")["state"],
            state
        );
    }

    let ownership = fs::read(
        root().join("engineering_assurance/contracts/verification-semantics-ownership-v1.json"),
    )
    .expect("ownership registry must be readable");
    let mut wrong: serde_json::Value =
        serde_json::from_slice(&ownership).expect("ownership registry must parse");
    wrong["concepts"][0]["authority"] = serde_json::json!("quoin");
    let encoded = serde_json::to_vec(&wrong).expect("mutant must serialize");
    let error = validate_ownership_registry_bytes(&encoded).unwrap_err();
    assert_eq!(error.code(), "invalid_semantic_contract");
    assert!(error.message().contains("authority must be"));

    let mut non_executing: serde_json::Value =
        serde_json::from_slice(&ownership).expect("ownership registry must parse");
    non_executing["non_executing"] = serde_json::json!(false);
    assert!(
        validate_ownership_registry_bytes(
            &serde_json::to_vec(&non_executing).expect("mutant must serialize")
        )
        .is_err()
    );

    let mut duplicate: serde_json::Value =
        serde_json::from_slice(&ownership).expect("ownership registry must parse");
    duplicate["concepts"][1] = duplicate["concepts"][0].clone();
    assert!(
        validate_ownership_registry_bytes(
            &serde_json::to_vec(&duplicate).expect("mutant must serialize")
        )
        .unwrap_err()
        .message()
        .contains("repeats")
    );
}

#[trace("TC-102", "FR-015-AC-2")]
#[trace("TC-103", "FR-015-AC-3")]
#[test]
fn tc_103_semantic_reference_failures_do_not_collapse() {
    let accepted: SemanticFixture =
        serde_json::from_slice(&fixture("canonical-references.json")).expect("fixture must parse");

    let mut missing = accepted.clone();
    missing.references.remove(0);
    assert!(
        missing
            .validate()
            .unwrap_err()
            .message()
            .contains("missing reference")
    );

    let mut confused = accepted.clone();
    confused.references[3].links.result = Some("sem:report-001".to_owned());
    assert!(
        confused
            .validate()
            .unwrap_err()
            .message()
            .contains("must target check_result")
    );

    let mut wrong_authority = accepted.clone();
    wrong_authority.references[0].authority = engineering_assurance::semantics::Authority::Quoin;
    assert!(
        wrong_authority
            .validate()
            .unwrap_err()
            .message()
            .contains("authority must be quire")
    );

    let mut missing_producer = accepted.clone();
    missing_producer.references[1].producer = None;
    assert!(
        missing_producer
            .validate()
            .unwrap_err()
            .message()
            .contains("complete producer tuple")
    );

    let mut duplicate_id = accepted.clone();
    duplicate_id.references[5].semantic_id = duplicate_id.references[4].semantic_id.clone();
    assert!(
        duplicate_id
            .validate()
            .unwrap_err()
            .message()
            .contains("distinct")
    );

    let mut unknown_version = accepted;
    unknown_version.references[0].source.schema_version = "99".to_owned();
    assert!(
        unknown_version
            .validate()
            .unwrap_err()
            .message()
            .contains("premises differ")
    );
}

#[trace("TC-100", "FR-015-AC-1")]
#[trace("TC-103", "FR-015-AC-3")]
#[test]
fn tc_100_pgm01_views_match_retained_python_for_accepted_inputs() {
    let script = r"
import json
from pathlib import Path
from engineering_assurance.verification_semantics import map_pgm01_bytes
root = Path('engineering_assurance/fixtures/verification-semantics')
print(json.dumps([map_pgm01_bytes((root / name).read_bytes()) for name in ('pgm01-v1.json', 'pgm01-v2.json')], sort_keys=True, separators=(',', ':')))
";
    let output = Command::new("python3")
        .args(["-c", script])
        .current_dir(root())
        .output()
        .expect("retained Python reference must execute during additive parity");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let expected: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("Python reference must emit JSON");
    let actual = serde_json::Value::Array(
        ["pgm01-v1.json", "pgm01-v2.json"]
            .into_iter()
            .map(|name| {
                serde_json::to_value(
                    map_pgm01_bytes(&fixture(name), None).expect("mapping must complete"),
                )
                .expect("mapping must serialize")
            })
            .collect(),
    );
    assert_eq!(actual, expected);
}

#[trace("TC-102", "FR-015-AC-2")]
#[trace("TC-103", "FR-015-AC-3")]
#[test]
fn tc_103_pgm01_adverse_outcomes_preserve_source_identity() {
    let unreadable = map_pgm01_bytes(b"not-json", None).expect("unreadable is a result");
    assert_eq!(unreadable.outcome, Pgm01Outcome::Unreadable);
    assert!(unreadable.mappings.is_empty());

    let unsupported = map_pgm01_bytes(
        br#"{"schemaVersion":"quire.pgm01-evidence/v99","recordId":"legacy-1"}"#,
        None,
    )
    .expect("unsupported is a result");
    assert_eq!(unsupported.outcome, Pgm01Outcome::Incompatible);

    let original = fixture("pgm01-v2.json");
    let digest = map_pgm01_bytes(&original, None)
        .expect("fixture must map")
        .source_digest;
    let altered = String::from_utf8(original.clone())
        .expect("governed fixture must be UTF-8")
        .replace("fictional-collector", "fictional-tampered")
        .into_bytes();
    assert_ne!(
        altered, original,
        "fixture must contain the controlled identity"
    );
    let tampered = map_pgm01_bytes(&altered, Some(&digest)).expect("tamper is a result");
    assert_eq!(tampered.outcome, Pgm01Outcome::Incompatible);
    assert!(tampered.mappings.is_empty());
    assert!(tampered.unmapped_fields[0].reason.contains("tampered"));

    let mut unbounded_integer: serde_json::Value =
        serde_json::from_slice(&original).expect("fixture must parse");
    unbounded_integer["commands"][0]["stdout"]["bytes"] =
        serde_json::from_str("184467440737095516160").expect("large integer must remain exact");
    let view = map_pgm01_bytes(
        &serde_json::to_vec(&unbounded_integer).expect("mutant must serialize"),
        None,
    )
    .expect("arbitrary-precision non-negative integer must map");
    let mapped = view
        .mappings
        .iter()
        .find(|item| item.source_path == "/commands/0/stdout/bytes")
        .expect("stdout byte count must remain mapped");
    assert_eq!(mapped.value.to_string(), "184467440737095516160");

    unbounded_integer["commands"][0]["stdout"]["bytes"] =
        serde_json::from_str("-0").expect("negative-zero integer token must parse");
    let view = map_pgm01_bytes(
        &serde_json::to_vec(&unbounded_integer).expect("mutant must serialize"),
        None,
    )
    .expect("Python-compatible negative-zero integer must map");
    let mapped = view
        .mappings
        .iter()
        .find(|item| item.source_path == "/commands/0/stdout/bytes")
        .expect("stdout byte count must remain mapped");
    assert_eq!(mapped.value, serde_json::json!(0));

    let mut malformed = unbounded_integer.clone();
    malformed
        .as_object_mut()
        .expect("fixture must remain an object")
        .remove("parameters");
    assert_eq!(
        map_pgm01_bytes(
            &serde_json::to_vec(&malformed).expect("mutant must serialize"),
            None
        )
        .expect("malformed source is a classified result")
        .outcome,
        Pgm01Outcome::Unreadable
    );

    let mut stale: serde_json::Value =
        serde_json::from_slice(&original).expect("fixture must parse");
    stale["historicalDisposition"] = serde_json::json!("retracted");
    let stale = map_pgm01_bytes(
        &serde_json::to_vec(&stale).expect("mutant must serialize"),
        None,
    )
    .expect("retracted source must map as stale");
    assert!(stale.mappings.iter().any(|mapping| {
        mapping.source_path == "/historicalDisposition" && mapping.value == "stale"
    }));
}

#[trace("TC-100", "FR-015-AC-1")]
#[test]
fn tc_100_report_rendering_matches_retained_python() {
    let raw = fixture("report-projection.json");
    let report: ReportProjection = serde_json::from_slice(&raw).expect("report must parse");
    let rust_json = report.render_json().expect("report JSON must render");
    let rust_markdown = report
        .render_markdown()
        .expect("report Markdown must render");
    let script = r"
import json
from pathlib import Path
from engineering_assurance.verification_semantics import render_report_json, render_report_markdown
value = json.loads(Path('engineering_assurance/fixtures/verification-semantics/report-projection.json').read_text())
print(json.dumps({'json': render_report_json(value), 'markdown': render_report_markdown(value)}, sort_keys=True, separators=(',', ':')))
";
    let output = Command::new("python3")
        .args(["-c", script])
        .current_dir(root())
        .output()
        .expect("retained Python reference must execute during additive parity");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let expected: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("Python reference must emit JSON");
    assert_eq!(rust_json, expected["json"]);
    assert_eq!(rust_markdown, expected["markdown"]);

    let mut extended: serde_json::Value =
        serde_json::from_slice(&raw).expect("report must parse as generic JSON");
    extended["overall_verdict"] = serde_json::json!("passed");
    assert!(
        serde_json::from_value::<ReportProjection>(extended).is_err(),
        "an aggregate verdict must not enter the bounded report type"
    );
}

#[trace("TC-100", "FR-015-AC-1")]
#[trace("TC-104", "FR-015-AC-4")]
#[test]
fn tc_100_rust_generator_matches_all_committed_inert_fixtures() {
    let expected: BTreeMap<String, String> = fs::read_dir(
        root().join("engineering_assurance/fixtures/verification-semantics/generated"),
    )
    .expect("generated fixture directory must exist")
    .map(|entry| {
        let path = entry.expect("fixture entry must be readable").path();
        let name = path
            .file_name()
            .expect("fixture must have a name")
            .to_string_lossy()
            .into_owned();
        let body = fs::read_to_string(path).expect("fixture must be UTF-8");
        (name, body)
    })
    .collect();
    let corpus = fs::read(root().join("corpus/compatibility/corpus.json"))
        .expect("pinned compatibility corpus must be readable");
    assert_eq!(
        render_generated_fixtures(&corpus).expect("Rust generator must run"),
        expected
    );

    // FR-008-AC-4 / NFR-004-AC-2 are negative capability requirements. Static
    // inspection is the direct gate: there is no runtime path to exercise for
    // an execution or persistence capability that must not exist.
    let semantic_sources = [
        include_str!("../src/semantics/mod.rs"),
        include_str!("../src/semantics/fixtures.rs"),
        include_str!("../src/semantics/pgm01.rs"),
        include_str!("../src/semantics/report.rs"),
    ]
    .concat();
    for forbidden in ["std::fs", "std::process", "std::env", "Command::new"] {
        assert!(
            !semantic_sources.contains(forbidden),
            "pure semantic library contains {forbidden}"
        );
    }

    let contracts = fs::read_dir(root().join("engineering_assurance/contracts"))
        .expect("contract directory must exist")
        .chain(
            fs::read_dir(root().join("engineering_assurance/schemas"))
                .expect("schema directory must exist"),
        )
        .map(|entry| {
            let path = entry.expect("contract entry must be readable").path();
            if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                fs::read_to_string(path).expect("contract JSON must be UTF-8")
            } else {
                String::new()
            }
        })
        .collect::<String>();
    for forbidden in [
        "\"record_type\"",
        "\"retention\"",
        "\"evidence_store\"",
        "generic evidence envelope",
    ] {
        assert!(
            !contracts.to_lowercase().contains(&forbidden.to_lowercase()),
            "semantic contracts duplicate persisted evidence field {forbidden}"
        );
    }

    for relative in [".github/workflows", "scripts"] {
        for entry in fs::read_dir(root().join(relative)).expect("audit root must exist") {
            let path = entry.expect("audit entry must be readable").path();
            if path.is_file() {
                let body = fs::read_to_string(&path).unwrap_or_else(|error| {
                    panic!("audit input {} must be readable: {error}", path.display())
                });
                assert!(
                    !body.contains("fixtures/verification-semantics/generated"),
                    "generated foreign-language fixture is executed by {}",
                    path.display()
                );
            }
        }
    }
}
