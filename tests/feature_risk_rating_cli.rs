// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary acceptance tests for signed feature risk ratings.
//!
//! External acceptance contract: PLAT-1448 FR‑021 and its linked FR‑027 ACs.
//! Local trace attributes name the Engineering Assurance signature and CLI
//! requirements exercised by these tests; the external requirement is not a
//! local `FR‑027`, which identifies atomic publish in this repository.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use ix_trace_rs::trace;
use serde_json::{json, Value};

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    fn repo(&self) -> PathBuf {
        self.root.path().join("repo")
    }

    fn key(&self) -> PathBuf {
        self.root.path().join("signer")
    }

    fn other_key(&self) -> PathBuf {
        self.root.path().join("other")
    }

    fn allowed_signers(&self) -> PathBuf {
        self.root.path().join("allowed_signers")
    }
}

fn run(program: &str, cwd: &Path, args: &[&str]) -> Output {
    Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_DIR")
        .output()
        .expect("process starts")
}

fn git(repo: &Path, args: &[&str]) -> String {
    let output = run("git", repo, args);
    assert!(output.status.success(), "git {args:?}: {output:?}");
    String::from_utf8(output.stdout).expect("git output is UTF-8")
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().expect("temporary fixture directory");
    let fixture = Fixture { root };
    let key = fixture.key();
    for path in [&key, &fixture.other_key()] {
        let output = run(
            "ssh-keygen",
            fixture.root.path(),
            &[
                "-q",
                "-t",
                "ed25519",
                "-N",
                "",
                "-C",
                "rating-test",
                "-f",
                path.to_str().expect("key path is UTF-8"),
            ],
        );
        assert!(output.status.success(), "ssh-keygen: {output:?}");
    }
    let public = fs::read_to_string(key.with_extension("pub")).expect("public key");
    set_allowed_signers(&fixture, "alice@example.test", &public);
    fs::create_dir_all(fixture.repo().join("spec/validation")).expect("validation directory");
    git(&fixture.repo(), &["init", "-q", "-b", "main"]);
    git(&fixture.repo(), &["config", "user.name", "Rating Test"]);
    git(
        &fixture.repo(),
        &["config", "user.email", "rating@example.test"],
    );
    git(&fixture.repo(), &["config", "gpg.format", "ssh"]);
    git(
        &fixture.repo(),
        &[
            "config",
            "user.signingkey",
            key.to_str().expect("key path is UTF-8"),
        ],
    );
    fs::write(fixture.repo().join("README.md"), "fixture repository\n").expect("base file");
    git(&fixture.repo(), &["add", "README.md"]);
    git(
        &fixture.repo(),
        &["commit", "--quiet", "--no-gpg-sign", "-m", "fixture base"],
    );
    fixture
}

fn set_allowed_signers(fixture: &Fixture, principal: &str, public_key: &str) {
    fs::write(
        fixture.allowed_signers(),
        format!("{principal} {public_key}"),
    )
    .expect("allowed-signers file");
}

fn public_key(path: &Path) -> String {
    fs::read_to_string(path.with_extension("pub")).expect("public key")
}

fn rating_file(fixture: &Fixture) -> PathBuf {
    rating_file_with_status(fixture, "accepted")
}

fn rating_file_with_status(fixture: &Fixture, status: &str) -> PathBuf {
    let path = fixture.repo().join("spec/validation/FRR-001.md");
    fs::write(
        &path,
        format!("---\nid: FRR-001\ntype: FeatureRiskRating\nstatus: {status}\nrated_by: alice@example.test\n---\n\n# Rating\n"),
    )
    .expect("rating document");
    path
}

fn commit_rating(fixture: &Fixture, include_meaning: bool) {
    git(&fixture.repo(), &["add", "spec/validation/FRR-001.md"]);
    let mut args = vec![
        "commit",
        "--quiet",
        "-S",
        "-m",
        "Accept feature risk rating",
    ];
    if include_meaning {
        args.extend(["-m", "Signed-off-meaning: feature-risk-rating-accepted"]);
    }
    git(&fixture.repo(), &args);
}

fn acceptance(fixture: &Fixture) -> (Output, Value) {
    let request = json!({
        "protocol": "engineering-assurance.rating-acceptance-request/v1",
        "repository": fixture.repo(),
        "allowed_signers": fixture.allowed_signers(),
    });
    let request = serde_json::to_string(&request).expect("request JSON");
    let output = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["rating-acceptance", "--request", &request])
        .output()
        .expect("rating-acceptance starts");
    let result = serde_json::from_slice(&output.stdout).expect("acceptance JSON");
    (output, result)
}

fn verify_head(fixture: &Fixture) -> Value {
    let request = json!({
        "protocol": "engineering-assurance.signature-verify-request/v1",
        "repository": fixture.repo(),
        "revision": "HEAD",
        "kind": "commit",
        "allowed_signers": fixture.allowed_signers(),
    });
    let request = serde_json::to_string(&request).expect("request JSON");
    let output = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["signature-verify", "--request", &request])
        .output()
        .expect("signature-verify starts");
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).expect("signature verification JSON")
}

#[test]
/// PLAT-1448 FR‑021 / FR‑027 AC-1: a mapped signed commit with the required meaning is accepted.
/// PLAT-1448 FR‑021 / FR‑027 AC-10: a later unsigned edit invalidates acceptance.
#[trace("FR-036-AC-1", "FR-036-AC-2", "FR-037-AC-1", "FR-037-AC-5")]
fn signed_rating_is_accepted_and_a_later_worktree_edit_invalidates_it() {
    let fixture = fixture();
    let path = rating_file(&fixture);
    commit_rating(&fixture, true);
    let verified = verify_head(&fixture);
    assert_eq!(verified["status"], "signed_valid", "{verified}");
    assert_eq!(
        verified["signer"]["principal"], "alice@example.test",
        "{verified}"
    );

    let (accepted, result) = acceptance(&fixture);
    assert!(accepted.status.success(), "{accepted:?}");
    assert_eq!(result["results"][0]["status"], "accepted", "{result}");

    fs::write(
        &path,
        "---\nid: FRR-001\ntype: FeatureRiskRating\nrated_by: alice@example.test\n---\n\n# Rating\n\nChanged after the signed commit.\n",
    )
    .expect("later unsigned edit");
    let (invalidated, result) = acceptance(&fixture);
    assert_eq!(invalidated.status.code(), Some(1), "{invalidated:?}");
    assert_eq!(result["results"][0]["status"], "not_accepted", "{result}");
    assert_eq!(result["results"][0]["reason"], "not_committed", "{result}");

    git(&fixture.repo(), &["add", "spec/validation/FRR-001.md"]);
    git(
        &fixture.repo(),
        &[
            "commit",
            "--quiet",
            "--no-gpg-sign",
            "-m",
            "unsigned rating edit",
        ],
    );
    let (unsigned_edit, result) = acceptance(&fixture);
    assert_eq!(unsigned_edit.status.code(), Some(1), "{unsigned_edit:?}");
    assert_eq!(result["results"][0]["reason"], "unsigned", "{result}");
}

#[test]
#[trace("FR-027-AC-14")]
fn proposed_signed_rating_is_not_accepted() {
    let fixture = fixture();
    rating_file_with_status(&fixture, "proposed");
    commit_rating(&fixture, true);

    let (output, result) = acceptance(&fixture);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(result["results"][0]["status"], "not_accepted", "{result}");
    assert_eq!(
        result["results"][0]["reason"], "status_proposed",
        "{result}"
    );
}

#[test]
/// PLAT-1448 FR‑021 / FR‑027 AC-5: missing acceptance meaning is refused.
#[trace("FR-037-AC-6")]
fn signed_rating_without_acceptance_meaning_has_exact_reason() {
    let fixture = fixture();
    rating_file(&fixture);
    commit_rating(&fixture, false);
    let (output, result) = acceptance(&fixture);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        result["results"][0]["reason"], "meaning_missing",
        "{result}"
    );
}

#[test]
/// PLAT-1448 FR‑021 / FR‑027 AC-2: unsigned rating commits are refused.
#[trace("FR-036-AC-2")]
fn unsigned_rating_commit_has_exact_reason() {
    let fixture = fixture();
    rating_file(&fixture);
    git(&fixture.repo(), &["add", "spec/validation/FRR-001.md"]);
    git(
        &fixture.repo(),
        &[
            "commit",
            "--quiet",
            "--no-gpg-sign",
            "-m",
            "unsigned rating",
        ],
    );
    let (output, result) = acceptance(&fixture);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(result["results"][0]["reason"], "unsigned", "{result}");
}

#[test]
/// PLAT-1448 FR‑021 / FR‑027 allowed-signers obligation: invalid trust input is never accepted.
#[trace("FR-037-AC-3", "FR-014-AC-3")]
fn missing_or_malformed_allowed_signers_fails_closed() {
    for malformed in [false, true] {
        let fixture = fixture();
        rating_file(&fixture);
        commit_rating(&fixture, true);
        let allowed_signers = fixture.allowed_signers();
        fs::remove_file(&allowed_signers).expect("remove allowed-signers file");
        if malformed {
            fs::create_dir(&allowed_signers).expect("malformed allowed-signers directory");
        }

        let (output, result) = acceptance(&fixture);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(result["results"][0]["status"], "not_accepted", "{result}");
        assert_eq!(result["results"][0]["reason"], "unverifiable", "{result}");
    }
}

#[test]
/// PLAT-1448 FR‑021 / FR‑027 AC-3: an unmapped signer is refused.
#[trace("FR-036-AC-3", "FR-037-AC-2")]
fn valid_signature_without_a_mapped_signer_has_exact_reason() {
    let fixture = fixture();
    rating_file(&fixture);
    commit_rating(&fixture, true);
    set_allowed_signers(
        &fixture,
        "bob@example.test",
        &public_key(&fixture.other_key()),
    );
    let (output, result) = acceptance(&fixture);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        result["results"][0]["reason"], "signer_unmapped",
        "{result}"
    );
}

#[test]
/// PLAT-1448 FR‑021 / FR‑027 AC-4: a different mapped principal is refused.
fn mapped_signer_with_the_wrong_principal_has_exact_reason() {
    let fixture = fixture();
    rating_file(&fixture);
    commit_rating(&fixture, true);
    set_allowed_signers(&fixture, "bob@example.test", &public_key(&fixture.key()));
    let (output, result) = acceptance(&fixture);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        result["results"][0]["reason"], "principal_mismatch",
        "{result}"
    );
}

#[test]
/// PLAT-1448 FR‑021 / FR‑027: an uncommitted rating cannot be accepted.
fn uncommitted_rating_file_has_exact_reason() {
    let fixture = fixture();
    rating_file(&fixture);
    let (output, result) = acceptance(&fixture);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(result["results"][0]["reason"], "not_committed", "{result}");
}
