// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Requirement and reference-parity tests for the Rust semantic/projection slice.
//!
//! Each reference here is a file, not a process. Every byte was captured once
//! from `engineering_assurance/verification_semantics.py` as it stood at
//! `bd6d2ce`, the candidate revision that cut this capability over, and
//! committed so that the cutover could delete that lane without deleting the
//! evidence that Rust reproduces it. While the reference was a `python3`
//! invocation the two were inseparable: deleting the implementation would have
//! deleted the only statement of what its output was.
//!
//! A reader who wants to re-derive the captures rather than trust them can
//! restore that file from `bd6d2ce` and run `map_pgm01_bytes`,
//! `render_report_json` and `render_report_markdown` over the same governed
//! fixtures, dumping each result with sorted keys and no separator padding.
//! That is how these bytes were produced and how they have been checked since.

use std::{collections::BTreeMap, fmt::Write as _, fs, path::PathBuf};

use engineering_assurance::semantics::{
    Pgm01Outcome, ReportProjection, SemanticErrorKind, SemanticFixture, map_pgm01_bytes,
    render_generated_fixtures, validate_embedded_ownership_registry,
    validate_ownership_registry_bytes, validate_semantic_fixture_bytes,
};
use ix_trace_rs::trace;
use sha2::{Digest, Sha256};

/// The two accepted PGM-01 views the retired Python lane produced for the
/// governed v1 and v2 fixtures, captured verbatim in sorted-key compact JSON.
const REFERENCE_ACCEPTED_VIEWS: &[u8] =
    include_bytes!("fixtures/semantics-pgm01-accepted-views.json");

/// The five adverse scalar-identity views the retired Python lane produced,
/// captured verbatim. These fix the identity fallbacks an empty, numeric,
/// boolean, or null `schemaVersion`/`recordId` resolves to.
const REFERENCE_ADVERSE_IDENTITY_VIEWS: &[u8] =
    include_bytes!("fixtures/semantics-pgm01-adverse-identity-views.json");

/// The rendered bounded report the retired Python lane produced for the
/// governed projection fixture, as its JSON and Markdown strings.
const REFERENCE_REPORT_RENDER: &[u8] = include_bytes!("fixtures/semantics-report-render.json");

/// What the retired Python lane did with a structured PGM-01 identity, recorded
/// because FR-015 declares a deliberate divergence there rather than parity.
const REFERENCE_STRUCTURED_IDENTITY: &[u8] =
    include_bytes!("fixtures/semantics-structured-identity-observations.json");

/// Compare one produced value against captured reference bytes.
///
/// The comparison is on the encoded bytes rather than on parsed structures
/// because a projection that widened a number, re-spelled a value, or renamed a
/// field would still compare equal under a looser reading while no longer being
/// the same record. Key order is not among the differences this catches: the
/// encoder sorts object keys, which is also what makes the captured reference
/// comparable at all. Trailing newlines differ between a file and an in-memory
/// encoding and are not part of the record, so they are trimmed from both.
fn assert_matches_reference(produced: &serde_json::Value, reference: &[u8], what: &str) {
    let encoded = serde_json::to_string(produced).expect("produced value must serialize");
    let expected = String::from_utf8(reference.to_vec()).expect("captured reference must be UTF-8");
    assert_eq!(
        encoded.trim_end(),
        expected.trim_end(),
        "{what} drifted from the captured reference bytes"
    );
}

/// Hash bytes with SHA-256 and render the digest as lowercase hexadecimal.
///
/// This deliberately does not reach for the library's own digest helper: a test
/// that asked the code under test to confirm its own arithmetic would agree with
/// it whatever that arithmetic became.
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in hasher.finalize() {
        write!(encoded, "{byte:02x}").expect("a String write cannot fail");
    }
    encoded
}

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

fn rust_sources_under(directory: &std::path::Path) -> Vec<(PathBuf, String)> {
    let mut sources = Vec::new();
    for entry in fs::read_dir(directory).expect("Rust source directory must be readable") {
        let path = entry.expect("Rust source entry must be readable").path();
        if path.is_dir() {
            sources.extend(rust_sources_under(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let source = fs::read_to_string(&path).expect("Rust source must be UTF-8");
            sources.push((path, source));
        }
    }
    sources
}

fn forbidden_semantic_capability(source: &str) -> Option<&'static str> {
    let identifiers: std::collections::BTreeSet<&str> = source
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .filter(|identifier| !identifier.is_empty())
        .collect();
    ["fs", "process", "env", "Command", "io", "net", "libc"]
        .into_iter()
        .find(|forbidden| identifiers.contains(forbidden))
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
    assert_eq!(error.kind(), SemanticErrorKind::AuthorityMismatch);

    let mut non_executing: serde_json::Value =
        serde_json::from_slice(&ownership).expect("ownership registry must parse");
    non_executing["non_executing"] = serde_json::json!(false);
    assert_eq!(
        validate_ownership_registry_bytes(
            &serde_json::to_vec(&non_executing).expect("mutant must serialize")
        )
        .unwrap_err()
        .kind(),
        SemanticErrorKind::InvalidOwnershipBoundary
    );

    let mut duplicate: serde_json::Value =
        serde_json::from_slice(&ownership).expect("ownership registry must parse");
    duplicate["concepts"][1] = duplicate["concepts"][0].clone();
    assert_eq!(
        validate_ownership_registry_bytes(
            &serde_json::to_vec(&duplicate).expect("mutant must serialize")
        )
        .unwrap_err()
        .kind(),
        SemanticErrorKind::DuplicateOwnershipConcept
    );
}

#[trace("TC-057", "FR-008-AC-2")]
#[trace("TC-102", "FR-015-AC-2")]
#[trace("TC-103", "FR-015-AC-3")]
#[test]
fn tc_103_semantic_reference_failures_do_not_collapse() {
    let accepted: SemanticFixture =
        serde_json::from_slice(&fixture("canonical-references.json")).expect("fixture must parse");

    let mut missing = accepted.clone();
    missing.references.remove(0);
    assert_eq!(
        missing.validate().unwrap_err().kind(),
        SemanticErrorKind::MissingReference
    );

    let mut confused = accepted.clone();
    confused.references[3].links.result = Some("sem:report-001".to_owned());
    assert_eq!(
        confused.validate().unwrap_err().kind(),
        SemanticErrorKind::ReferenceConceptMismatch
    );

    let mut wrong_authority = accepted.clone();
    wrong_authority.references[0].authority = engineering_assurance::semantics::Authority::Quoin;
    assert_eq!(
        wrong_authority.validate().unwrap_err().kind(),
        SemanticErrorKind::AuthorityMismatch
    );

    let mut missing_producer = accepted.clone();
    missing_producer.references[1].producer = None;
    assert_eq!(
        missing_producer.validate().unwrap_err().kind(),
        SemanticErrorKind::MissingProducer
    );

    let mut duplicate_id = accepted.clone();
    duplicate_id.references[5].semantic_id = duplicate_id.references[4].semantic_id.clone();
    assert_eq!(
        duplicate_id.validate().unwrap_err().kind(),
        SemanticErrorKind::DuplicateSemanticIdentity
    );

    let mut unknown_version = accepted;
    unknown_version.references[0].source.schema_version = "99".to_owned();
    assert_eq!(
        unknown_version.validate().unwrap_err().kind(),
        SemanticErrorKind::SourcePremiseMismatch
    );
}

#[trace("TC-100", "FR-015-AC-1")]
#[trace("TC-103", "FR-015-AC-3")]
#[test]
fn tc_100_pgm01_views_match_the_captured_accepted_reference() {
    // The digest each reference records is compared against a hash of the
    // fixture bytes computed here, not against anything the mapper produced.
    // That ordering matters: a governed fixture edited without its reference
    // moving with it would otherwise surface below as a mapping regression,
    // blaming the mapper for a stale record. Checking it against the bytes on
    // disk first names the real cause, and is the one statement in this test
    // that the mapper cannot satisfy by agreeing with itself.
    let recorded: Vec<serde_json::Value> =
        serde_json::from_slice(REFERENCE_ACCEPTED_VIEWS).expect("reference must be JSON");
    let names = ["pgm01-v1.json", "pgm01-v2.json"];
    assert_eq!(
        recorded.len(),
        names.len(),
        "the reference must cover both PGM-01 versions"
    );
    for (index, name) in names.into_iter().enumerate() {
        assert_eq!(
            recorded[index]["source_digest"].as_str(),
            Some(sha256_hex(&fixture(name)).as_str()),
            "the captured reference describes a different {name}"
        );
    }

    let actual = serde_json::Value::Array(
        names
            .into_iter()
            .map(|name| {
                serde_json::to_value(
                    map_pgm01_bytes(&fixture(name), None).expect("mapping must complete"),
                )
                .expect("mapping must serialize")
            })
            .collect(),
    );
    assert_matches_reference(
        &actual,
        REFERENCE_ACCEPTED_VIEWS,
        "the accepted PGM-01 v1 and v2 views",
    );
}

#[trace("TC-066", "FR-010-AC-3")]
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
#[trace("TC-103", "FR-015-AC-3")]
#[test]
fn tc_100_pgm01_adverse_scalar_identity_views_match_the_captured_reference() {
    // An empty, numeric, boolean, or absent scalar identity each resolve to a
    // declared fallback rather than to an invented one. These five cases reach
    // every fallback branch, so a mapper that started echoing an empty string
    // or an integer as a record identity would fail here.
    let cases = [
        br#"{"schemaVersion":"","recordId":"legacy-1"}"#.as_slice(),
        br#"{"schemaVersion":"quire.pgm01-evidence/v99","recordId":""}"#.as_slice(),
        br#"{"schemaVersion":123,"recordId":"legacy-1"}"#.as_slice(),
        br#"{"schemaVersion":"quire.pgm01-evidence/v99","recordId":42}"#.as_slice(),
        br#"{"schemaVersion":false,"recordId":null}"#.as_slice(),
    ];
    let actual = serde_json::Value::Array(
        cases
            .iter()
            .map(|raw| {
                serde_json::to_value(map_pgm01_bytes(raw, None).expect("case must classify"))
                    .expect("view must serialize")
            })
            .collect(),
    );
    assert_matches_reference(
        &actual,
        REFERENCE_ADVERSE_IDENTITY_VIEWS,
        "the adverse scalar-identity views",
    );
}

#[trace("TC-100", "FR-015-AC-1")]
#[trace("TC-103", "FR-015-AC-3")]
#[test]
fn tc_103_pgm01_structured_identity_divergences_are_explicit() {
    let object_id = br#"{"schemaVersion":"quire.pgm01-evidence/v99","recordId":{"a":1}}"#;
    let rust_object = map_pgm01_bytes(object_id, None).expect("Rust must classify object identity");
    assert_eq!(rust_object.outcome, Pgm01Outcome::Incompatible);
    assert_eq!(rust_object.source_record_id, "incompatible-source");

    let list_schema = br#"{"schemaVersion":[1,2],"recordId":"legacy-1"}"#;
    let rust_list = map_pgm01_bytes(list_schema, None).expect("Rust must classify list identity");
    assert_eq!(rust_list.outcome, Pgm01Outcome::Incompatible);
    assert_eq!(rust_list.source_schema_version, "unknown");

    // FR-015 declares this one case a deliberate divergence rather than parity:
    // the retired implementation rendered a structured identity as a
    // language-specific object literal, and raised an interpreter exception on
    // a list. Three of the four assertions below pin the captured record rather
    // than the implementation — nothing in `src/` can make them fail, and they
    // fail only if someone edits the observation. That is their purpose: the
    // divergence must stay a decision on the record, not quietly become an
    // absence of comparison. The `assert_ne!` is the one that reads Rust's own
    // output, and it is what refuses a silent convergence back onto the
    // language-specific spelling.
    let recorded: serde_json::Value =
        serde_json::from_slice(REFERENCE_STRUCTURED_IDENTITY).expect("observation must be JSON");
    assert_eq!(recorded[0]["result"]["source_record_id"], "{'a': 1}");
    assert_ne!(
        recorded[0]["result"]["source_record_id"],
        serde_json::Value::String(rust_object.source_record_id.clone()),
        "the recorded divergence must still be a divergence"
    );
    assert_eq!(recorded[1]["error_type"], "TypeError");
    assert_eq!(recorded[1]["message"], "unhashable type: 'list'");
}

#[trace("TC-067", "FR-010-AC-4")]
#[trace("TC-100", "FR-015-AC-1")]
#[test]
fn tc_100_report_rendering_matches_the_captured_reference() {
    let raw = fixture("report-projection.json");
    let report: ReportProjection = serde_json::from_slice(&raw).expect("report must parse");
    let rust_json = report.render_json().expect("report JSON must render");
    let rust_markdown = report
        .render_markdown()
        .expect("report Markdown must render");
    let expected: serde_json::Value =
        serde_json::from_slice(REFERENCE_REPORT_RENDER).expect("reference must be JSON");
    assert_eq!(
        serde_json::Value::String(rust_json),
        expected["json"],
        "the rendered report JSON drifted from the captured reference"
    );
    assert_eq!(
        serde_json::Value::String(rust_markdown),
        expected["markdown"],
        "the rendered report Markdown drifted from the captured reference"
    );

    let mut extended: serde_json::Value =
        serde_json::from_slice(&raw).expect("report must parse as generic JSON");
    extended["overall_verdict"] = serde_json::json!("passed");
    assert!(
        serde_json::from_value::<ReportProjection>(extended).is_err(),
        "an aggregate verdict must not enter the bounded report type"
    );
}

#[trace("TC-100", "FR-015-AC-1")]
#[trace("TC-104", "FR-015-AC-4", "FR-015-CON-2")]
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
    assert!(
        !expected.is_empty(),
        "the generated-fixture comparison read no committed projection"
    );
    let corpus = fs::read(root().join("corpus/compatibility/corpus.json"))
        .expect("pinned compatibility corpus must be readable");
    assert_eq!(
        render_generated_fixtures(&corpus).expect("Rust generator must run"),
        expected
    );

    // FR-015-CON-2 forbids qualification from executing a generated
    // foreign-language fixture, so the audit has to read the files that decide
    // what qualification runs. Reading only `.github/workflows` satisfied a
    // population floor of one while never opening the Makefile, which AGENTS.md
    // names as this repository's entry point and which is where such a call
    // would most plausibly be added. The roots below are walked recursively and
    // the named repository-root files are read individually.
    let mut audited = 0_usize;
    let mut audit = |path: &std::path::Path| {
        let body = fs::read_to_string(path).unwrap_or_else(|error| {
            panic!("audit input {} must be readable: {error}", path.display())
        });
        assert!(
            !body.contains("fixtures/verification-semantics/generated"),
            "generated foreign-language fixture is executed by {}",
            path.display()
        );
        audited += 1;
    };
    for relative in [".github", "scripts"] {
        let audit_root = root().join(relative);
        if !audit_root.is_dir() {
            continue;
        }
        let mut pending = vec![audit_root];
        while let Some(directory) = pending.pop() {
            for entry in fs::read_dir(&directory).expect("existing audit root must be readable") {
                let path = entry.expect("audit entry must be readable").path();
                if path.is_dir() {
                    pending.push(path);
                } else if path.is_file() {
                    audit(&path);
                }
            }
        }
    }
    for named in [
        "Makefile",
        "pyproject.toml",
        "package.json",
        "opencode.json",
    ] {
        let path = root().join(named);
        if path.is_file() {
            audit(&path);
        }
    }

    // A renamed entry point or a deleted directory audits nothing, and an empty
    // population would pass every assertion above by default. Five is the count
    // of first-party executable-configuration files this repository has, so a
    // floor below it would accept exactly that silence.
    assert!(
        audited >= 5,
        "the executable-path audit read {audited} first-party files, too few to mean anything"
    );
}

#[trace("TC-059", "FR-008-AC-4")]
#[trace("TC-059", "NFR-004-AC-2")]
#[trace("TC-104", "FR-015-AC-4")]
#[test]
fn tc_059_semantic_library_reaches_for_no_execution_or_persistence_capability() {
    // FR-008-AC-4 and NFR-004-AC-2 are negative capability requirements. Static
    // inspection is the direct gate: there is no runtime path to exercise for
    // an execution or persistence capability that must not exist.
    let semantic_sources = rust_sources_under(&root().join("src/semantics"));
    assert!(
        semantic_sources.len() >= 4,
        "the capability audit read fewer semantic modules than this library has"
    );
    for (path, source) in semantic_sources {
        assert_eq!(
            forbidden_semantic_capability(&source),
            None,
            "pure semantic library {} contains a forbidden capability identifier",
            path.display()
        );
    }

    // The audit above is a detector, and a detector that has stopped detecting
    // reports a compromised library indistinguishably from a clean one. Each
    // mutant below is a real way a capability enters a module: a direct call,
    // an aliased type, and a renamed module.
    for mutant in [
        "use std::{collections::BTreeMap, fs}; fn read() { fs::read(\"x\"); }",
        "use std::{collections::BTreeMap, process::Command as Spawn}; fn run() { Spawn::new(\"x\"); }",
        "use std::{collections::BTreeMap, env as ambient}; fn read() { ambient::var(\"X\"); }",
        "use std::{collections::BTreeMap, io}; fn read() { io::stdout(); }",
        "use std::{collections::BTreeMap, net::TcpStream}; fn dial() { net::connect(); }",
        "extern crate libc; fn call() { libc::exit(0); }",
    ] {
        assert!(
            forbidden_semantic_capability(mutant).is_some(),
            "negative-capability mutant escaped the static gate: {mutant}"
        );
    }
}

#[trace("TC-068", "NFR-004-AC-1")]
#[trace("TC-104", "FR-015-AC-4", "FR-015-CON-3")]
#[test]
fn tc_068_semantic_contracts_declare_no_parallel_record_family() {
    let mut audited = 0_usize;
    let mut contracts = String::new();
    for relative in [
        "engineering_assurance/contracts",
        "engineering_assurance/schemas",
    ] {
        for entry in
            fs::read_dir(root().join(relative)).expect("contract directory must be readable")
        {
            let path = entry.expect("contract entry must be readable").path();
            if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                contracts.push_str(&fs::read_to_string(path).expect("contract JSON must be UTF-8"));
                audited += 1;
            }
        }
    }

    // Concatenating a directory and searching the result passes trivially when
    // the directory is empty or has been renamed out from under the audit, so
    // the population is counted before it is searched.
    assert!(
        audited >= 4,
        "the contract audit read no meaningful contract population"
    );

    // These four tokens are how a second persisted evidence family would first
    // appear: a record discriminator, a retention policy, a store reference, or
    // the envelope named outright.
    let folded = contracts.to_lowercase();
    for forbidden in [
        "\"record_type\"",
        "\"retention\"",
        "\"evidence_store\"",
        "generic evidence envelope",
    ] {
        assert!(
            !folded.contains(&forbidden.to_lowercase()),
            "semantic contracts duplicate persisted evidence field {forbidden}"
        );
    }
}

#[trace("TC-052", "StR-002-VC-1")]
#[trace("TC-061", "FR-009-AC-2")]
#[trace("TC-104", "FR-015-AC-4")]
#[test]
fn tc_061_committed_generated_fixtures_agree_across_every_language() {
    let generated = root().join("engineering_assurance/fixtures/verification-semantics/generated");
    let read = |name: &str| -> String {
        fs::read_to_string(generated.join(name)).expect("committed fixture must be UTF-8")
    };

    // Each committed projection is inert data in its own language's syntax.
    // Extracting the payload from each and comparing the decoded values is the
    // assertion: a generator that diverged per language would still round-trip
    // against itself, and only a cross-language comparison catches it.
    let python = read("canonical_references.py");
    let python_json: String = serde_json::from_str(
        python
            .lines()
            .nth(1)
            .expect("the Python fixture must assign on its second line")
            .split_once(" = ")
            .expect("the Python fixture must be an assignment")
            .1,
    )
    .expect("the Python payload must be a JSON string literal");

    let typescript = read("canonical_references.ts");
    let typescript_json: String = serde_json::from_str(
        typescript
            .lines()
            .nth(1)
            .expect("the TypeScript fixture must assign on its second line")
            .split_once(" = ")
            .expect("the TypeScript fixture must be an assignment")
            .1
            .strip_suffix(';')
            .expect("the TypeScript assignment must end in a semicolon"),
    )
    .expect("the TypeScript payload must be a JSON string literal");

    let rust = read("canonical_references.rs");
    let rust_json = rust
        .lines()
        .nth(1)
        .expect("the Rust fixture must assign on its second line")
        .split_once("r#\"")
        .expect("the Rust fixture must use a raw string literal")
        .1
        .strip_suffix("\"#;")
        .expect("the Rust raw string must be terminated")
        .to_owned();

    let expected: serde_json::Value = serde_json::from_slice(&fixture("canonical-references.json"))
        .expect("the canonical semantic source must be JSON");
    for (language, payload) in [
        ("Python", python_json),
        ("TypeScript", typescript_json),
        ("Rust", rust_json),
    ] {
        let decoded: serde_json::Value =
            serde_json::from_str(&payload).expect("the committed payload must be JSON");
        assert_eq!(
            decoded, expected,
            "the committed {language} projection does not decode to the canonical source"
        );
    }

    // The state projections carry the same set, in the same order, in all three.
    let states: Vec<String> = serde_json::from_slice(&fixture("non-success-states.json"))
        .expect("the non-success-state source must be JSON");

    // An empty source makes every extracted list empty too, and three empty
    // lists agree with each other and with the source. The floor is repeated
    // here rather than relied on from the contract suite, because a guard in
    // another test binary is a coincidence, not an invariant of this one.
    assert!(
        states.len() >= 17,
        "the declared non-success vocabulary lost states"
    );
    for (name, prefix, suffix) in [
        ("non_success_states.py", "    \"", "\","),
        ("non_success_states.ts", "  \"", "\","),
        ("non_success_states.rs", "    \"", "\","),
    ] {
        let body = read(name);
        let listed: Vec<&str> = body
            .lines()
            .filter_map(|line| line.strip_prefix(prefix)?.strip_suffix(suffix))
            .collect();
        assert_eq!(listed, states, "{name} lists a different state set");
    }
}
