// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Public contract tests for the integrator-package writer, strict reader and
//! candidate-bound store (FR-030, FR-031, IT-007).

use std::{fs, path::Path, process::Command, thread};

use engineering_assurance::{
    atomic_publish::Published,
    integrator_package::{
        AdopterObligation, AdopterSubject, AttemptId, CandidateId, FactId, FactKind, FactState,
        INTEGRATOR_PACKAGE_PROTOCOL, IntegratorPackage, MAX_PACKAGE_BYTES, ObligationId,
        ObligationState, ReadError, SourceFact, SourceRef, SubjectId, Violation, WriteError,
        read_package, write_package,
    },
    integrator_package_store::{StoreError, publish_package, read_published},
};
use ix_trace_rs::trace;
use serde_json::{Value, json};

fn candidate(id: &str) -> CandidateId {
    CandidateId::new(id).expect("candidate")
}

fn attempt(id: &str) -> AttemptId {
    AttemptId::new(id).expect("attempt")
}

fn fact(index: usize, kind: FactKind, state: FactState) -> SourceFact {
    let established = state == FactState::Established;
    let rights = matches!(kind, FactKind::License | FactKind::ReuseRight);
    SourceFact {
        id: FactId::new(format!("fact-{index:02}")).expect("fact id"),
        kind,
        state,
        authority: "release-board".to_owned(),
        statement: format!("statement for {kind:?} in state {state:?}"),
        value: (established && rights).then(|| "MIT OR Apache-2.0".to_owned()),
        sources: if established {
            vec![SourceRef {
                issuer: "fictional-issuer".to_owned(),
                reference: format!("source-{index}"),
            }]
        } else {
            Vec::new()
        },
    }
}

/// One fact per kind; the state varies so every state is exercised.
fn facts() -> Vec<SourceFact> {
    const STATES: [FactState; 7] = [
        FactState::Established,
        FactState::Open,
        FactState::Negative,
        FactState::Incomplete,
        FactState::Conditional,
        FactState::Conflict,
        FactState::Refused,
    ];
    FactKind::ALL
        .into_iter()
        .enumerate()
        .map(|(index, kind)| fact(index, kind, STATES[index % STATES.len()]))
        .collect()
}

fn subject(id: &str, completed: bool) -> AdopterSubject {
    let status = if completed {
        ObligationState::Completed {
            attributed_to: "adopter-engineer".to_owned(),
            response: "supplied by the adopter".to_owned(),
        }
    } else {
        ObligationState::Open
    };
    AdopterSubject {
        id: SubjectId::new(id).expect("subject"),
        intended_use: format!("intended use of {id}"),
        configuration: format!("configuration of {id}"),
        obligations: vec![AdopterObligation {
            id: ObligationId::new("validate").expect("obligation"),
            statement: "validate the release for the intended use".to_owned(),
            status,
        }],
    }
}

fn package(attempt_id: &str) -> IntegratorPackage {
    IntegratorPackage::new(
        candidate("cand-1"),
        attempt(attempt_id),
        facts(),
        vec![subject("use-a", false), subject("use-b", true)],
    )
}

fn json_of(package: &IntegratorPackage) -> Value {
    serde_json::from_slice(&write_package(package).expect("write")).expect("json")
}

/// Canonical bytes of an edited document; `edit` mutates the JSON value.
fn mutated(edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut value = json_of(&package("a1"));
    edit(&mut value);
    serde_json_canonicalizer::to_vec(&value).expect("canonical")
}

fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(dir)
        .expect("read_dir")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

#[test]
#[trace("TC-219", "FR-030-AC-1")]
fn tc_214_round_trip_preserves_every_fact_and_is_deterministic() {
    let original = package("a1");
    let bytes = write_package(&original).expect("write");
    assert_eq!(read_package(&bytes).expect("read"), original);
    assert_eq!(write_package(&original).expect("again"), bytes);
    // Insertion order is not identity: the constructor orders by id.
    let mut reversed = facts();
    reversed.reverse();
    let shuffled = IntegratorPackage::new(
        candidate("cand-1"),
        attempt("a1"),
        reversed,
        vec![subject("use-b", true), subject("use-a", false)],
    );
    assert_eq!(write_package(&shuffled).expect("shuffled"), bytes);
    assert_eq!(original.protocol, INTEGRATOR_PACKAGE_PROTOCOL);
}

#[test]
#[trace("TC-220", "FR-030-AC-2")]
fn tc_215_open_and_negative_states_and_adopter_input_stay_as_written() {
    let read = read_package(&write_package(&package("a1")).expect("write")).expect("read");
    for state in [
        FactState::Open,
        FactState::Negative,
        FactState::Incomplete,
        FactState::Conditional,
        FactState::Conflict,
        FactState::Refused,
    ] {
        assert!(
            read.facts.iter().any(|fact| fact.state == state),
            "{state:?} survives"
        );
    }
    // Two distinct adopter subjects; the open one is open, the completed one
    // keeps its attribution and is not listed as open or as accepted.
    let open: Vec<_> = read.open_obligations().collect();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].0.as_str(), "use-a");
    let completed = &read.adopter_subjects[1].obligations[0].status;
    assert_eq!(
        completed,
        &ObligationState::Completed {
            attributed_to: "adopter-engineer".to_owned(),
            response: "supplied by the adopter".to_owned(),
        }
    );
    // The contract has no way to say "accepted".
    for key in ["accepted", "acceptance", "approved", "disposition"] {
        let bytes = mutated(|value| {
            value[key] = json!(true);
        });
        assert!(
            matches!(read_package(&bytes), Err(ReadError::Malformed(_))),
            "{key} is refused"
        );
    }
    let bytes = mutated(|value| {
        value["adopter_subjects"][0]["obligations"][0]["status"] = json!({"state": "accepted"});
    });
    assert!(matches!(read_package(&bytes), Err(ReadError::Malformed(_))));
}

#[test]
#[trace("TC-221", "FR-030-AC-3")]
fn tc_216_strict_reader_refuses_unknown_and_non_canonical_content() {
    let good = write_package(&package("a1")).expect("write");
    // Unknown field, at every level.
    for edit in [
        (|v: &mut Value| v["extra"] = json!(1)) as fn(&mut Value),
        |v| v["facts"][0]["extra"] = json!(1),
        |v| v["facts"][0]["sources"][0]["extra"] = json!(1),
        |v| v["adopter_subjects"][0]["extra"] = json!(1),
        |v| v["adopter_subjects"][0]["obligations"][0]["extra"] = json!(1),
        // Unknown variants.
        |v| v["facts"][0]["kind"] = json!("novel_kind"),
        |v| v["facts"][0]["state"] = json!("accepted"),
        |v| v["facts"][0]["id"] = json!("bad/id"),
        |v| v["candidate"] = json!(""),
        |v| v["candidate"] = json!("Upper"),
        |v| v["attempt"] = json!("trailing."),
    ] {
        let bytes = mutated(edit);
        assert!(
            matches!(read_package(&bytes), Err(ReadError::Malformed(_))),
            "{}",
            String::from_utf8_lossy(&bytes)
        );
    }
    // Not canonical: whitespace, a trailing newline, a duplicate key.
    let text = String::from_utf8(good.clone()).expect("utf8");
    let spaced = text.replacen(',', ", ", 1);
    assert_eq!(
        read_package(spaced.as_bytes()),
        Err(ReadError::NotCanonical)
    );
    let mut newline = good.clone();
    newline.push(b'\n');
    assert_eq!(read_package(&newline), Err(ReadError::NotCanonical));
    let duplicate = text.replacen('{', r#"{"candidate":"cand-1","#, 1);
    assert!(matches!(
        read_package(duplicate.as_bytes()),
        Err(ReadError::Malformed(_))
    ));
    // Unordered identities.
    let bytes = mutated(|v| v["facts"].as_array_mut().expect("facts").reverse());
    assert!(matches!(
        read_package(&bytes),
        Err(ReadError::Invalid(Violation::Unordered { .. }))
    ));
    // Not a package at all, and too large.
    assert!(matches!(read_package(b"{"), Err(ReadError::NotAPackage(_))));
    assert!(matches!(
        read_package(b"[]"),
        Err(ReadError::NotAPackage(_))
    ));
    assert_eq!(
        read_package(&vec![b' '; MAX_PACKAGE_BYTES + 1]),
        Err(ReadError::TooLarge)
    );
}

#[test]
#[trace("TC-222", "FR-030-AC-4")]
fn tc_217_other_protocol_versions_are_refused_by_kind() {
    let with = |protocol: &str| mutated(|v| v["protocol"] = json!(protocol));
    let family = "engineering-assurance.integrator-package/v";
    assert!(matches!(
        read_package(&with(&format!("{family}0"))),
        Err(ReadError::Downgraded(_))
    ));
    assert!(matches!(
        read_package(&with(&format!("{family}2"))),
        Err(ReadError::UnsupportedVersion(_))
    ));
    for odd in [
        format!("{family}01"),
        format!("{family}x"),
        family.to_owned(),
    ] {
        assert!(matches!(
            read_package(&with(&odd)),
            Err(ReadError::UnknownProtocol(_))
        ));
    }
    assert!(matches!(
        read_package(&with("other.package/v1")),
        Err(ReadError::UnknownProtocol(_))
    ));
}

#[test]
#[trace("TC-223", "FR-030-AC-5")]
fn tc_218_every_loss_or_improvement_is_refused_not_repaired() {
    let invalid = |edit: fn(&mut Value)| match read_package(&mutated(edit)) {
        Err(ReadError::Invalid(violation)) => violation,
        other => panic!("expected an invalid package, got {other:?}"),
    };
    // Dropping a fact with its count updated still loses a kind.
    let dropped = invalid(|v| {
        let facts = v["facts"].as_array_mut().expect("facts");
        let index = facts
            .iter()
            .position(|f| f["kind"] == "license")
            .expect("license");
        facts.remove(index);
        v["fact_count"] = json!(facts.len());
    });
    assert_eq!(dropped, Violation::MissingKind(FactKind::License));
    // Dropping a fact without touching the count is a count mismatch.
    assert!(matches!(
        invalid(|v| {
            v["facts"].as_array_mut().expect("facts").remove(0);
        }),
        Violation::CountMismatch { what: "fact", .. }
    ));
    assert!(matches!(
        invalid(|v| {
            v["adopter_subjects"]
                .as_array_mut()
                .expect("subjects")
                .remove(0);
        }),
        Violation::CountMismatch {
            what: "subject",
            ..
        }
    ));
    // A fact made "established" without a basis is an improvement.
    assert!(matches!(
        invalid(|v| v["facts"][1]["state"] = json!("established")),
        Violation::Unsupported {
            missing: "sources",
            ..
        }
    ));
    // An established license or reuse right needs its value.
    assert!(matches!(
        invalid(|v| {
            let facts = v["facts"].as_array_mut().expect("facts");
            for f in facts.iter_mut().filter(|f| f["kind"] == "license") {
                f["state"] = json!("established");
                f["sources"] = json!([{"issuer": "i", "reference": "r"}]);
                f["value"] = Value::Null;
            }
        }),
        Violation::Unsupported {
            missing: "value",
            ..
        }
    ));
    // A value on a fact that is not established is refused.
    assert!(matches!(
        invalid(|v| v["facts"][1]["value"] = json!("MIT")),
        Violation::UnestablishedValue { .. }
    ));
    assert!(matches!(
        invalid(|v| v["facts"][1]["id"] = json!("fact-00")),
        Violation::Duplicate { what: "fact", .. }
    ));
    assert!(matches!(
        invalid(|v| v["facts"][0]["statement"] = json!("")),
        Violation::Text { .. }
    ));
    assert!(matches!(
        invalid(|v| v["facts"][0]["authority"] = json!(" padded ")),
        Violation::Text { .. }
    ));
    assert!(matches!(
        invalid(
            |v| v["adopter_subjects"][1]["obligations"][0]["status"]["attributed_to"] = json!("")
        ),
        Violation::Text { .. }
    ));
    assert!(matches!(
        invalid(|v| v["adopter_subjects"][0]["obligations"] = json!([])),
        Violation::EmptySubject { .. }
    ));
}

#[test]
#[trace("TC-224", "FR-030-AC-6")]
fn tc_219_the_writer_refuses_what_the_reader_would_refuse() {
    let mut missing_kind = package("a1");
    missing_kind.facts.retain(|f| f.kind != FactKind::Review);
    missing_kind.fact_count = missing_kind.facts.len();
    assert!(matches!(
        write_package(&missing_kind),
        Err(WriteError::Invalid(Violation::MissingKind(
            FactKind::Review
        )))
    ));
    let mut stale = package("a1");
    stale.fact_count += 1;
    assert!(matches!(
        write_package(&stale),
        Err(WriteError::Invalid(Violation::CountMismatch { .. }))
    ));
    let mut other_protocol = package("a1");
    other_protocol.protocol = "engineering-assurance.integrator-package/v2".to_owned();
    assert!(matches!(
        write_package(&other_protocol),
        Err(WriteError::Protocol)
    ));
    let mut oversize = package("a1");
    oversize.facts[0].statement = "x".repeat(20_000);
    assert!(matches!(
        write_package(&oversize),
        Err(WriteError::Invalid(Violation::Text { .. }))
    ));
}

#[test]
#[trace("TC-225", "FR-031-AC-1")]
fn tc_220_publication_is_candidate_bound_verified_and_leaves_no_temporary() {
    let root = tempfile::tempdir().expect("root");
    let original = package("a1");
    let published = publish_package(root.path(), &original).expect("publish");
    assert_eq!(published.outcome, Published::Written);
    assert_eq!(published.path, root.path().join("cand-1").join("a1.json"));
    assert_eq!(entries(&root.path().join("cand-1")), ["a1.json"]);
    assert_eq!(
        read_published(root.path(), &candidate("cand-1"), &attempt("a1")).expect("read"),
        original
    );
    // An invalid package writes nothing, not even a directory.
    let mut invalid = package("a2");
    invalid.fact_count = 0;
    assert!(matches!(
        publish_package(root.path(), &invalid),
        Err(StoreError::Write(_))
    ));
    assert_eq!(entries(root.path()), ["cand-1"]);
    assert_eq!(entries(&root.path().join("cand-1")), ["a1.json"]);
    // The root must exist.
    assert!(publish_package(&root.path().join("absent"), &original).is_err());
}

#[test]
#[trace("TC-226", "FR-031-AC-2")]
fn tc_221_identical_retry_resolves_to_the_one_object() {
    let root = tempfile::tempdir().expect("root");
    let original = package("a1");
    let first = publish_package(root.path(), &original).expect("first");
    let bytes = fs::read(&first.path).expect("bytes");
    for _ in 0..3 {
        let again = publish_package(root.path(), &original).expect("retry");
        assert_eq!(again.outcome, Published::AlreadyPresent);
        assert_eq!(again.path, first.path);
    }
    assert_eq!(fs::read(&first.path).expect("bytes"), bytes);
    assert_eq!(entries(&root.path().join("cand-1")), ["a1.json"]);
}

#[test]
#[trace("TC-227", "FR-031-AC-3")]
fn tc_222_attempt_reuse_with_changed_input_is_a_conflict_with_no_winner() {
    let root = tempfile::tempdir().expect("root");
    let original = package("a1");
    let first = publish_package(root.path(), &original).expect("first");
    let before = fs::read(&first.path).expect("bytes");
    let mut changed = package("a1");
    changed.facts[0].statement = "a changed statement".to_owned();
    assert!(matches!(
        publish_package(root.path(), &changed),
        Err(StoreError::Conflict { .. })
    ));
    // Neither version is presented any more, and nothing was replaced.
    assert!(matches!(
        read_published(root.path(), &candidate("cand-1"), &attempt("a1")),
        Err(StoreError::Conflict { .. })
    ));
    assert_eq!(fs::read(&first.path).expect("bytes"), before);
    // The original writer's identical retry no longer succeeds.
    assert!(matches!(
        publish_package(root.path(), &original),
        Err(StoreError::Conflict { .. })
    ));
    assert_eq!(
        entries(&root.path().join("cand-1")),
        ["a1.conflict", "a1.json"]
    );
}

#[test]
#[trace("TC-228", "FR-031-AC-4")]
fn tc_223_racing_writers_conflict_if_non_identical_and_agree_if_identical() {
    let race = |distinct: bool| {
        let root = tempfile::tempdir().expect("root");
        let results: Vec<_> = (0..8)
            .map(|writer| {
                let path = root.path().to_owned();
                thread::spawn(move || {
                    let mut candidate_package = package("race");
                    if distinct {
                        candidate_package.facts[0].statement = format!("writer {writer}");
                    }
                    publish_package(&path, &candidate_package).map(|done| done.outcome)
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().expect("no writer panicked"))
            .collect();
        (root, results)
    };
    let (root, results) = race(true);
    let written = results
        .iter()
        .filter(|r| matches!(r, Ok(Published::Written)))
        .count();
    let conflicts = results
        .iter()
        .filter(|r| matches!(r, Err(StoreError::Conflict { .. })))
        .count();
    // The creating writer may itself see a loser's marker before it returns.
    assert!(written <= 1, "at most one writer created the object");
    assert!(conflicts >= 7, "every other writer saw the conflict");
    assert_eq!(written + conflicts, 8, "no other outcome: {results:?}");
    assert!(matches!(
        read_published(root.path(), &candidate("cand-1"), &attempt("race")),
        Err(StoreError::Conflict { .. })
    ));
    let (root, results) = race(false);
    assert!(results.iter().all(Result::is_ok), "{results:?}");
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Ok(Published::Written)))
            .count(),
        1
    );
    read_published(root.path(), &candidate("cand-1"), &attempt("race"))
        .expect("identical race reads");
}

#[test]
#[trace("TC-229", "FR-031-AC-5")]
fn tc_224_reader_refuses_unbound_partial_and_non_regular_objects() {
    let root = tempfile::tempdir().expect("root");
    let first = publish_package(root.path(), &package("a1")).expect("publish");
    let dir = root.path().join("cand-1");
    // The same bytes moved to another attempt or candidate are not bound.
    fs::copy(&first.path, dir.join("a9.json")).expect("copy");
    assert!(matches!(
        read_published(root.path(), &candidate("cand-1"), &attempt("a9")),
        Err(StoreError::Unbound { .. })
    ));
    fs::create_dir(root.path().join("cand-2")).expect("mkdir");
    fs::copy(&first.path, root.path().join("cand-2/a1.json")).expect("copy");
    assert!(matches!(
        read_published(root.path(), &candidate("cand-2"), &attempt("a1")),
        Err(StoreError::Unbound { .. })
    ));
    // A truncated object is refused by the strict reader.
    let bytes = fs::read(&first.path).expect("bytes");
    fs::write(dir.join("a3.json"), &bytes[..bytes.len() / 2]).expect("partial");
    assert!(matches!(
        read_published(root.path(), &candidate("cand-1"), &attempt("a3")),
        Err(StoreError::Read { .. })
    ));
    // A corrupt or non-regular occupant is reported as itself and marks nothing.
    let mut occupant = package("a7");
    fs::write(dir.join("a7.json"), b"garbage").expect("garbage");
    assert!(matches!(
        publish_package(root.path(), &occupant),
        Err(StoreError::Read { .. })
    ));
    fs::create_dir(dir.join("a8.json")).expect("dir");
    occupant.attempt = attempt("a8");
    assert!(matches!(
        publish_package(root.path(), &occupant),
        Err(StoreError::NotRegular { .. })
    ));
    assert!(!dir.join("a7.conflict").exists() && !dir.join("a8.conflict").exists());
    // A directory is not an object, and nothing is present for an absent attempt.
    fs::create_dir(dir.join("a4.json")).expect("dir");
    assert!(matches!(
        read_published(root.path(), &candidate("cand-1"), &attempt("a4")),
        Err(StoreError::NotRegular { .. })
    ));
    assert!(matches!(
        read_published(root.path(), &candidate("cand-1"), &attempt("a5")),
        Err(StoreError::Io { .. })
    ));
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&first.path, dir.join("a6.json")).expect("symlink");
        assert!(matches!(
            read_published(root.path(), &candidate("cand-1"), &attempt("a6")),
            Err(StoreError::NotRegular { .. })
        ));
    }
}

#[test]
#[trace("TC-230", "FR-031-AC-6")]
fn tc_225_attempts_and_candidates_are_independent() {
    let root = tempfile::tempdir().expect("root");
    publish_package(root.path(), &package("a1")).expect("a1");
    let mut second = package("a2");
    second.facts[0].statement = "different attempt, different content".to_owned();
    publish_package(root.path(), &second).expect("a2 coexists");
    let mut other = IntegratorPackage::new(
        candidate("cand-2"),
        attempt("a1"),
        facts(),
        vec![subject("use-a", false)],
    );
    other.facts[0].statement = "other candidate".to_owned();
    publish_package(root.path(), &other).expect("other candidate");
    assert_eq!(entries(&root.path().join("cand-1")), ["a1.json", "a2.json"]);
    read_published(root.path(), &candidate("cand-1"), &attempt("a1")).expect("a1 intact");
}

#[test]
#[trace("TC-231", "FR-030-AC-7", "IT-007-SC-01", "IT-007-SC-02")]
fn tc_226_a_real_consumer_uses_only_the_integrator_package_feature() {
    let consumer = tempfile::tempdir().expect("consumer root");
    fs::create_dir(consumer.path().join("src")).expect("src");
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    fs::write(
        consumer.path().join("Cargo.toml"),
        format!(
            "[package]\nname='integrator-consumer-fixture'\nversion='0.0.0'\nedition='2024'\n[dependencies]\nengineering-assurance={{path={manifest_dir:?},default-features=false,features=['integrator-package']}}\n"
        ),
    )
    .expect("manifest");
    fs::write(
        consumer.path().join("src/main.rs"),
        r#"use engineering_assurance::integrator_package::*;
use engineering_assurance::integrator_package_store::*;
fn main() {
    let dir = std::env::args().nth(1).expect("store root");
    let fact = |index: usize, kind: FactKind| SourceFact {
        id: FactId::new(format!("f{index:02}")).unwrap(),
        kind,
        state: FactState::Incomplete,
        authority: "consumer".to_owned(),
        statement: "not yet examined".to_owned(),
        value: None,
        sources: Vec::new(),
    };
    let facts = FactKind::ALL.into_iter().enumerate().map(|(i, k)| fact(i, k)).collect();
    let package = IntegratorPackage::new(
        CandidateId::new("c1").unwrap(),
        AttemptId::new("a1").unwrap(),
        facts,
        Vec::new(),
    );
    let first = publish_package(std::path::Path::new(&dir), &package).unwrap();
    let again = publish_package(std::path::Path::new(&dir), &package).unwrap();
    assert_ne!(first.outcome, again.outcome);
    let read = read_published(std::path::Path::new(&dir), &package.candidate, &package.attempt).unwrap();
    assert_eq!(read, package);
}
"#,
    )
    .expect("source");
    let store = tempfile::tempdir().expect("store");
    let status = Command::new(env!("CARGO"))
        .args(["run", "--offline", "--quiet", "--manifest-path"])
        .arg(consumer.path().join("Cargo.toml"))
        .arg("--")
        .arg(store.path())
        .env("CARGO_BUILD_JOBS", "2")
        .env("CARGO_TARGET_DIR", consumer.path().join("target"))
        .status()
        .expect("consumer cargo run must launch");
    assert!(status.success(), "the consumer builds and round-trips");
    let metadata = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--offline",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(consumer.path().join("Cargo.toml"))
        .output()
        .expect("metadata");
    let graph: Value = serde_json::from_slice(&metadata.stdout).expect("metadata JSON");
    let names: Vec<_> = graph["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .filter_map(|p| p["name"].as_str())
        .collect();
    for unrelated in [
        "clap",
        "tar",
        "zip",
        "flate2",
        "jsonschema",
        "regex",
        "yaml_serde",
    ] {
        assert!(
            !names.contains(&unrelated),
            "{unrelated} must not be pulled in"
        );
    }
}
