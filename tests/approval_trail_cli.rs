// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary tests for `approval-trail` (FR-043) on real repositories
//! whose annotated tags are signed with fictional ed25519 keys generated inside
//! each test.

use std::{
    fmt::Write as _,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const RECORD_ID: &str = "change-record-17";
const DIGEST: &str = "sha256:0123456789abcdef";
const REQUEST_PROTOCOL: &str = "engineering-assurance.approval-trail-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.approval-trail-result/v1";
const ALICE: &str = "alice@example.test";

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn repo(&self) -> PathBuf {
        self.root().join("repo")
    }

    fn allowed(&self) -> PathBuf {
        self.root().join("allowed_signers")
    }
}

fn run_program(
    program: &str,
    directory: &Path,
    arguments: &[&str],
    input: Option<&[u8]>,
) -> Vec<u8> {
    let mut child = Command::new(program)
        .args(arguments)
        .current_dir(directory)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("program must start");
    let mut stdin = child.stdin.take().expect("stdin");
    if let Some(input) = input {
        stdin.write_all(input).expect("input");
    }
    drop(stdin);
    let output = child.wait_with_output().expect("output");
    assert!(
        output.status.success(),
        "{program} {arguments:?}: {output:?}"
    );
    output.stdout
}

fn git(repo: &Path, arguments: &[&str]) -> String {
    let mut all = vec!["-c", "user.name=t", "-c", "user.email=t@example.invalid"];
    all.extend_from_slice(arguments);
    String::from_utf8(run_program("git", repo, &all, None)).expect("utf-8")
}

/// A repository with one commit, keys `a` and `b`, and an allowed-signers file
/// mapping `a` to alice (key `b` is unlisted).
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    for key in ["a", "b"] {
        let path = root.join(key);
        run_program(
            "ssh-keygen",
            root,
            &[
                "-q",
                "-t",
                "ed25519",
                "-N",
                "",
                "-C",
                key,
                "-f",
                path.to_str().expect("utf-8"),
            ],
            None,
        );
    }
    let public = fs::read_to_string(root.join("a.pub")).expect("key");
    fs::write(root.join("allowed_signers"), format!("{ALICE} {public}")).expect("signers");
    let repo = root.join("repo");
    fs::create_dir(&repo).expect("repo dir");
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "base"]);
    Fixture { dir }
}

fn message(id: &str, digest: &str, meaning: Option<&str>) -> String {
    let base =
        format!("Approve the change\n\nApproved-Record-Id: {id}\nApproved-Record-Digest: {digest}");
    meaning.map_or_else(
        || base.clone(),
        |meaning| format!("{base}\nSigned-off-meaning: {meaning}"),
    )
}

fn good(meaning: &str) -> String {
    message(RECORD_ID, DIGEST, Some(meaning))
}

/// Signs an annotated tag whose tagger date is `date` (a git date with a zone).
fn signed_tag(fixture: &Fixture, key: &str, name: &str, text: &str, date: &str) {
    let public = fixture.root().join(format!("{key}.pub"));
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.invalid",
            "-c",
            "gpg.format=ssh",
            "-c",
            &format!("user.signingkey={}", public.display()),
            "tag",
            "-s",
            "-m",
            text,
            name,
        ])
        .current_dir(fixture.repo())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_COMMITTER_DATE", date)
        .env_remove("GIT_DIR")
        .output()
        .expect("git");
    assert!(output.status.success(), "{output:?}");
}

/// An unsigned annotated tag object written as is, with the given header lines.
fn forged_tag(fixture: &Fixture, name: &str, tagger: Option<&str>, message: &[u8]) {
    let repo = fixture.repo();
    let commit = git(&repo, &["rev-parse", "HEAD"]);
    let tagger = tagger.map_or_else(String::new, |line| format!("tagger {line}\n"));
    let mut object = format!(
        "object {}\ntype commit\ntag {name}\n{tagger}\n",
        commit.trim()
    )
    .into_bytes();
    object.extend_from_slice(message);
    let id = run_program(
        "git",
        &repo,
        &["hash-object", "-t", "tag", "-w", "--literally", "--stdin"],
        Some(&object),
    );
    let id = String::from_utf8(id).expect("utf-8");
    git(
        &repo,
        &["update-ref", &format!("refs/tags/{name}"), id.trim()],
    );
}

fn request(fixture: &Fixture) -> Value {
    json!({
        "protocol": REQUEST_PROTOCOL,
        "repository": fixture.repo().display().to_string(),
        "record_id": RECORD_ID,
        "record_digest": DIGEST,
        "allowed_signers": fixture.allowed().display().to_string(),
    })
}

fn run_command(subcommand: &str, body: &[u8], extra: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args([subcommand, "--request", "-"])
        .args(extra)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the Cargo-built CLI must start");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(body)
        .expect("request");
    child.wait_with_output().expect("output")
}

fn trail_of(request: &Value) -> Value {
    let output = run_command("approval-trail", request.to_string().as_bytes(), &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = std::str::from_utf8(&output.stdout).expect("utf-8");
    assert!(
        text.ends_with('\n') && text.matches('\n').count() == 1,
        "{text:?}"
    );
    let value: Value = serde_json::from_str(text).expect("json line");
    assert_eq!(value["protocol"], RESULT_PROTOCOL);
    value
}

fn trail(fixture: &Fixture) -> Value {
    trail_of(&request(fixture))
}

fn tags(trail: &Value) -> Vec<String> {
    trail["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .map(|e| e["tag"].as_str().expect("tag").to_owned())
        .collect()
}

fn entry<'a>(trail: &'a Value, tag: &str) -> &'a Value {
    trail["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .find(|e| e["tag"] == tag)
        .unwrap_or_else(|| panic!("{tag} not listed: {trail}"))
}

#[test]
#[trace("TC-352", "FR-043-AC-1", "FR-043-CON-3")]
fn tc_352_an_approval_and_a_later_revocation_are_listed_in_time_order() {
    let fixture = fixture();
    // The later tag sorts first by name and is created first.
    signed_tag(
        &fixture,
        "a",
        "a-revoke",
        &good("revoked"),
        "2026-05-02T09:00:00+0000",
    );
    signed_tag(
        &fixture,
        "a",
        "z-approve",
        &good("approved"),
        "2026-05-01T09:00:00+0000",
    );
    let result = trail(&fixture);
    assert_eq!(tags(&result), ["z-approve", "a-revoke"]);
    for (tag, meaning, at) in [
        ("z-approve", "approved", "2026-05-01T09:00:00Z"),
        ("a-revoke", "revoked", "2026-05-02T09:00:00Z"),
    ] {
        let listed = entry(&result, tag);
        assert_eq!(listed["status"], "approved", "{listed}");
        assert_eq!(listed["approver"], ALICE);
        assert_eq!(listed["meaning"], json!([meaning]));
        assert_eq!(listed["signed_at"], at);
        assert_eq!(listed["claimed_at"], at);
    }
    assert_eq!(result["ordering"], "claimed_at_ascending");
    assert_eq!(result["record_id"], RECORD_ID);
    assert_eq!(result["record_digest"], DIGEST);
    assert_eq!(
        result["trust"]["allowed_signers"],
        fixture.allowed().display().to_string()
    );
    assert_eq!(result["considered"], 2);
    assert_eq!(result["skipped"], 0);
    assert_eq!(result["truncated"], false);
    assert!(result.get("truncation").is_none());
}

#[test]
#[trace("TC-353", "FR-043-AC-2", "FR-043-CON-3")]
fn tc_353_failed_and_untrusted_approvals_are_listed_with_their_verdict() {
    let fixture = fixture();
    let date = "2026-05-01T09:00:00+0000";
    signed_tag(&fixture, "a", "v-ok", &good("approved"), date);
    signed_tag(&fixture, "a", "v-bad", &good("approved"), date);
    let repo = fixture.repo();
    let object = git(&repo, &["cat-file", "tag", "v-bad"]).replace("reviewed", "x");
    let tampered = object.replace("Approve the change", "Approve the changes");
    let id = run_program(
        "git",
        &repo,
        &["hash-object", "-t", "tag", "-w", "--literally", "--stdin"],
        Some(tampered.as_bytes()),
    );
    git(
        &repo,
        &[
            "update-ref",
            "refs/tags/v-bad",
            String::from_utf8(id).expect("utf-8").trim(),
        ],
    );
    signed_tag(&fixture, "b", "v-untrusted", &good("approved"), date);
    signed_tag(
        &fixture,
        "a",
        "v-digest",
        &message(RECORD_ID, "sha256:other", Some("approved")),
        date,
    );
    signed_tag(
        &fixture,
        "a",
        "v-nomeaning",
        &message(RECORD_ID, DIGEST, None),
        date,
    );
    let result = trail(&fixture);
    assert_eq!(result["entries"].as_array().expect("entries").len(), 5);
    for (tag, expected) in [
        ("v-ok", "approved"),
        ("v-bad", "signed_bad"),
        ("v-untrusted", "untrusted_signer"),
        ("v-digest", "wrong_record"),
        ("v-nomeaning", "meaning_missing"),
    ] {
        assert_eq!(entry(&result, tag)["status"], expected, "{tag}");
    }
    assert!(entry(&result, "v-untrusted").get("approver").is_none());
}

#[test]
#[trace("TC-354", "FR-043-AC-3")]
fn tc_354_other_records_and_non_approval_tags_are_not_listed() {
    let fixture = fixture();
    let date = "2026-05-01T09:00:00+0000";
    signed_tag(&fixture, "a", "v-mine", &good("approved"), date);
    signed_tag(
        &fixture,
        "a",
        "v-other",
        &message("another-record", DIGEST, Some("approved")),
        date,
    );
    git(&fixture.repo(), &["tag", "v-light"]);
    git(
        &fixture.repo(),
        &["tag", "-a", "-m", "plain release note", "v-plain"],
    );
    let result = trail(&fixture);
    assert_eq!(tags(&result), ["v-mine"]);
    assert_eq!(result["considered"], 4);
    assert_eq!(result["skipped"], 0);
}

#[test]
#[trace("TC-355", "FR-043-AC-4", "FR-043-CON-1")]
fn tc_355_each_entry_is_the_approval_verify_result_for_its_tag() {
    let fixture = fixture();
    let date = "2026-05-01T09:00:00-0800";
    signed_tag(&fixture, "a", "v-ok", &good("approved"), date);
    signed_tag(&fixture, "b", "v-untrusted", &good("approved"), date);
    signed_tag(
        &fixture,
        "a",
        "v-digest",
        &message(RECORD_ID, "sha256:other", Some("approved")),
        date,
    );
    let result = trail(&fixture);
    for tag in ["v-ok", "v-untrusted", "v-digest"] {
        let mut body = request(&fixture);
        body["tag"] = json!(tag);
        body["protocol"] = json!("engineering-assurance.approval-verify-request/v1");
        let output = run_command("approval-verify", body.to_string().as_bytes(), &[]);
        let mut verified: Value = serde_json::from_slice(&output.stdout).expect("json");
        let members = verified.as_object_mut().expect("object");
        for echo in ["protocol", "record_id", "record_digest", "trust"] {
            members.remove(echo);
        }
        let mut listed = entry(&result, tag).clone();
        let claimed = listed.as_object_mut().expect("object").remove("claimed_at");
        assert_eq!(listed, verified, "{tag}");
        assert_eq!(claimed, Some(json!("2026-05-01T17:00:00Z")), "{tag}");
    }
}

#[test]
#[trace("TC-356", "FR-043-AC-5")]
fn tc_356_entries_without_a_time_follow_the_timed_ones_by_tag_name() {
    let fixture = fixture();
    let unsigned = message(RECORD_ID, DIGEST, Some("approved"));
    forged_tag(&fixture, "n-b", None, unsigned.as_bytes());
    forged_tag(
        &fixture,
        "n-a",
        Some("t <t@example.invalid> yesterday +0000"),
        unsigned.as_bytes(),
    );
    forged_tag(
        &fixture,
        "t-late",
        Some("t <t@example.invalid> 1800000000 +0000"),
        unsigned.as_bytes(),
    );
    signed_tag(
        &fixture,
        "b",
        "t-early",
        &good("approved"),
        "2026-01-01T00:00:00+0000",
    );
    let result = trail(&fixture);
    assert_eq!(tags(&result), ["t-early", "t-late", "n-a", "n-b"]);
    // The claim is shown whatever the verdict, and attests nothing.
    let early = entry(&result, "t-early");
    assert_eq!(early["status"], "untrusted_signer");
    assert_eq!(early["claimed_at"], "2026-01-01T00:00:00Z");
    assert!(early.get("signed_at").is_none());
    assert_eq!(entry(&result, "t-late")["status"], "unsigned");
    assert!(entry(&result, "n-a").get("claimed_at").is_none());
    assert!(entry(&result, "n-b").get("claimed_at").is_none());
}

#[test]
#[trace("TC-357", "FR-043-AC-6")]
fn tc_357_more_than_ten_thousand_tags_are_truncated_and_say_so() {
    let fixture = fixture();
    let repo = fixture.repo();
    let commit = git(&repo, &["rev-parse", "HEAD"]);
    let mut updates = String::new();
    for n in 0..10_001 {
        writeln!(updates, "create refs/tags/l-{n:05} {}", commit.trim()).expect("string");
    }
    run_program(
        "git",
        &repo,
        &["update-ref", "--stdin"],
        Some(updates.as_bytes()),
    );
    let result = trail(&fixture);
    assert_eq!(result["truncated"], true);
    assert_eq!(result["truncation"], "tag_limit");
    assert_eq!(result["considered"], 10_000);
    assert_eq!(tags(&result), Vec::<String>::new());
}

#[test]
#[trace("TC-358", "FR-043-AC-7")]
fn tc_358_hostile_tag_names_are_listed_and_run_nothing() {
    let fixture = fixture();
    signed_tag(
        &fixture,
        "a",
        "base",
        &good("approved"),
        "2026-05-01T09:00:00+0000",
    );
    let repo = fixture.repo();
    let id = git(&repo, &["rev-parse", "refs/tags/base"]);
    let names = [
        "--help",
        "--end-of-options",
        "HEAD",
        "main",
        "v1;touch;pwned",
        "$(touch,pwned)",
        "it's",
        "a\"b",
        "a`touch,pwned`",
    ];
    for name in names {
        git(
            &repo,
            &["update-ref", &format!("refs/tags/{name}"), id.trim()],
        );
    }
    let result = trail(&fixture);
    let mut listed = tags(&result);
    listed.sort();
    let mut expected: Vec<String> = names.iter().map(|n| (*n).to_owned()).collect();
    expected.push("base".to_owned());
    expected.sort();
    assert_eq!(listed, expected);
    for tag in &listed {
        assert_eq!(entry(&result, tag)["status"], "approved", "{tag}");
    }
    assert!(!repo.join("pwned").exists());
}

#[test]
#[trace("TC-359", "FR-043-AC-8")]
fn tc_359_a_tag_whose_message_cannot_be_read_is_skipped_and_counted() {
    let fixture = fixture();
    signed_tag(
        &fixture,
        "a",
        "v-ok",
        &good("approved"),
        "2026-05-01T09:00:00+0000",
    );
    let nul = format!(
        "Approve the change\n\nApproved-Record-Id: {RECORD_ID}\0\nApproved-Record-Digest: {DIGEST}\n"
    );
    forged_tag(
        &fixture,
        "v-nul",
        Some("t <t@example.invalid> 1700000000 +0000"),
        nul.as_bytes(),
    );
    let result = trail(&fixture);
    assert_eq!(tags(&result), ["v-ok"]);
    assert_eq!(result["skipped"], 1);
    assert_eq!(result["considered"], 2);
}

#[test]
#[trace("TC-360", "FR-043-AC-9")]
fn tc_360_the_table_lists_the_trail_for_people() {
    let fixture = fixture();
    signed_tag(
        &fixture,
        "a",
        "v-1",
        &good("approved"),
        "2026-05-01T09:00:00+0000",
    );
    signed_tag(
        &fixture,
        "a",
        "v-2",
        &good("revoked"),
        "2026-05-02T09:00:00+0000",
    );
    signed_tag(
        &fixture,
        "b",
        "v-3",
        &good("approved"),
        "2026-05-03T09:00:00+0000",
    );
    let output = run_command(
        "approval-trail",
        request(&fixture).to_string().as_bytes(),
        &["--format", "table"],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = String::from_utf8(output.stdout).expect("utf-8");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 5, "{text}");
    let columns = |line: &str| {
        line.split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        columns(lines[0]),
        ["CLAIMED_AT", "STATUS", "APPROVER", "MEANING", "TAG"]
    );
    assert_eq!(
        columns(lines[1]),
        ["2026-05-01T09:00:00Z", "approved", ALICE, "approved", "v-1"]
    );
    assert_eq!(
        columns(lines[2]),
        ["2026-05-02T09:00:00Z", "approved", ALICE, "revoked", "v-2"]
    );
    assert_eq!(
        columns(lines[3]),
        ["2026-05-03T09:00:00Z", "untrusted_signer", "-", "-", "v-3"]
    );
    assert_eq!(lines[4], "considered 3, skipped 0, truncated no");
}

#[test]
#[trace("TC-361", "FR-043-AC-10")]
fn tc_361_invalid_requests_and_repositories_are_machine_errors() {
    let fixture = fixture();
    let long = "a".repeat(257);
    let gitsign = |value: Value| json!({"gitsign": value});
    let mutations: Vec<(&str, Value)> = vec![
        ("unknown", json!({"extra": 1})),
        ("tag member", json!({"tag": "v1"})),
        ("protocol", json!({"protocol": "wrong"})),
        ("empty id", json!({"record_id": ""})),
        ("long id", json!({"record_id": long})),
        ("dash digest", json!({"record_digest": "-d"})),
        ("control id", json!({"record_id": "a\nb"})),
        ("relative repo", json!({"repository": "repo"})),
        ("relative signers", json!({"allowed_signers": "signers"})),
        ("gitsign empty", gitsign(json!({}))),
    ];
    for (label, mutation) in mutations {
        let mut body = request(&fixture);
        for (key, value) in mutation.as_object().expect("object") {
            body[key] = value.clone();
        }
        let output = run_command("approval-trail", body.to_string().as_bytes(), &[]);
        assert_eq!(output.status.code(), Some(2), "{label}: {output:?}");
        let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
        assert_eq!(error["protocol"], "engineering-assurance.error/v1");
        assert_eq!(error["capability"], "approval-trail");
        assert_eq!(error["code"], "approval_trail_request_invalid", "{label}");
        assert!(!output.stderr.is_empty());
    }
    let code = |output: Output| {
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
        error["code"].as_str().expect("code").to_owned()
    };
    assert_eq!(
        code(run_command("approval-trail", b"{not json", &[])),
        "approval_trail_request_unreadable"
    );
    let mut body = request(&fixture);
    body["allowed_signers"] = json!(fixture.root().join("missing").display().to_string());
    assert_eq!(
        code(run_command(
            "approval-trail",
            body.to_string().as_bytes(),
            &[]
        )),
        "approval_trail_allowed_signers_invalid"
    );
    let mut body = request(&fixture);
    body["repository"] = json!(fixture.root().join("missing").display().to_string());
    assert_eq!(
        code(run_command(
            "approval-trail",
            body.to_string().as_bytes(),
            &[]
        )),
        "approval_trail_repository_unavailable"
    );
}
