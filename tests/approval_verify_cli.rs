// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary tests for `approval-verify` (FR-041) on real repositories
//! whose annotated tags are signed with fictional ed25519 keys generated inside
//! each test.

use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const RECORD_ID: &str = "change-record-17";
const DIGEST: &str = "sha256:0123456789abcdef";
const REQUEST_PROTOCOL: &str = "engineering-assurance.approval-verify-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.approval-verify-result/v1";
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

    fn public(&self, key: &str) -> PathBuf {
        self.root().join(format!("{key}.pub"))
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

fn git(repo: &Path, key: Option<&str>, arguments: &[&str]) -> String {
    let mut all: Vec<String> = ["-c", "user.name=t", "-c", "user.email=t@example.invalid"]
        .map(str::to_owned)
        .to_vec();
    if let Some(key) = key {
        all.extend([
            "-c".to_owned(),
            "gpg.format=ssh".to_owned(),
            "-c".to_owned(),
            format!("user.signingkey={key}"),
        ]);
    }
    all.extend(arguments.iter().map(|a| (*a).to_owned()));
    let refs: Vec<&str> = all.iter().map(String::as_str).collect();
    String::from_utf8(run_program("git", repo, &refs, None)).expect("utf-8")
}

/// A repository with one commit, keys `a`, `b` and `m`, and an allowed-signers
/// file mapping `a` to alice and `m` to mallory (key `b` is unlisted).
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    for key in ["a", "b", "m"] {
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
    let public = |key: &str| fs::read_to_string(root.join(format!("{key}.pub"))).expect("key");
    fs::write(
        root.join("allowed_signers"),
        format!(
            "{ALICE} {}mallory@example.test {}",
            public("a"),
            public("m")
        ),
    )
    .expect("allowed signers");
    let repo = root.join("repo");
    fs::create_dir(&repo).expect("repo dir");
    git(&repo, None, &["init", "-q", "-b", "main"]);
    git(
        &repo,
        None,
        &["commit", "-q", "--allow-empty", "-m", "base"],
    );
    Fixture { dir }
}

/// The message of an approval: a subject, then the given trailer lines.
fn approval(trailers: &[&str]) -> String {
    format!("Approve the change\n\n{}", trailers.join("\n"))
}

fn full_trailers() -> Vec<String> {
    vec![
        format!("Approved-Record-Id: {RECORD_ID}"),
        format!("Approved-Record-Digest: {DIGEST}"),
        "Signed-off-meaning: reviewed".to_owned(),
    ]
}

fn good_message() -> String {
    let lines = full_trailers();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    approval(&refs)
}

fn signed_tag(fixture: &Fixture, key: &str, name: &str, message: &str) {
    let public = fixture.public(key);
    git(
        &fixture.repo(),
        Some(public.to_str().expect("utf-8")),
        &["tag", "-s", "-m", message, name],
    );
}

fn request(fixture: &Fixture, tag: &str) -> Value {
    json!({
        "protocol": REQUEST_PROTOCOL,
        "repository": fixture.repo().display().to_string(),
        "tag": tag,
        "record_id": RECORD_ID,
        "record_digest": DIGEST,
        "allowed_signers": fixture.allowed().display().to_string(),
    })
}

fn run_command(subcommand: &str, body: &[u8], path: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"));
    command
        .args([subcommand, "--request", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = path {
        command.env("PATH", path);
    }
    let mut child = command.spawn().expect("the Cargo-built CLI must start");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(body)
        .expect("request");
    child.wait_with_output().expect("output")
}

/// The result of a request: exit 0, one JSON line, the result protocol.
fn result_on(request: &Value, path: Option<&Path>) -> Value {
    let output = run_command("approval-verify", request.to_string().as_bytes(), path);
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

fn result(request: &Value) -> Value {
    result_on(request, None)
}

fn status(fixture: &Fixture, tag: &str) -> Value {
    result(&request(fixture, tag))
}

/// Rewrites a tag object's bytes with `edit` and points the tag name at it.
fn rewrite_tag(fixture: &Fixture, name: &str, edit: impl Fn(&str) -> String) {
    let repo = fixture.repo();
    let object = git(&repo, None, &["cat-file", "tag", name]);
    let id = run_program(
        "git",
        &repo,
        &["hash-object", "-t", "tag", "-w", "--stdin"],
        Some(edit(&object).as_bytes()),
    );
    let id = String::from_utf8(id).expect("utf-8");
    git(
        &repo,
        None,
        &["update-ref", &format!("refs/tags/{name}"), id.trim()],
    );
}

#[test]
#[trace("TC-316", "FR-041-AC-1", "FR-041-CON-1", "FR-041-AC-11")]
fn tc_316_a_bound_signed_tag_with_a_meaning_is_approved() {
    let fixture = fixture();
    let message = approval(&[
        &format!("Approved-Record-Id: {RECORD_ID}"),
        &format!("Approved-Record-Digest: {DIGEST}"),
        "Signed-off-meaning: reviewed",
        "Signed-off-meaning: approved for release",
    ]);
    signed_tag(&fixture, "a", "v-approved", &message);
    for name in ["v-approved", "refs/tags/v-approved"] {
        let approved = status(&fixture, name);
        assert_eq!(approved["status"], "approved", "{approved}");
        assert_eq!(approved["approver"], ALICE);
        assert_eq!(
            approved["meaning"],
            json!(["reviewed", "approved for release"])
        );
        assert_eq!(approved["signature_kind"], "ssh");
        assert_eq!(approved["tag"], "v-approved");
        assert_eq!(approved["record_id"], RECORD_ID);
        assert_eq!(approved["record_digest"], DIGEST);
        assert_eq!(
            approved["binding"],
            json!({"record_id": "match", "record_digest": "match"})
        );
        assert_eq!(approved["signature"]["status"], "signed_valid");
        assert!(approved["signature"].get("protocol").is_none());
        assert!(approved.get("reason").is_none());
    }
}

#[test]
#[trace("TC-317", "FR-041-AC-11")]
fn tc_317_unrelated_trailers_keep_the_unchanged_signature_verdict() {
    let fixture = fixture();
    signed_tag(
        &fixture,
        "a",
        "v-other",
        &approval(&[
            "Approved-Record-Id: some-other-record",
            "Approved-Record-Digest: sha256:ffff",
            "Signed-off-meaning: reviewed",
        ]),
    );
    let approval = status(&fixture, "v-other");
    assert_eq!(approval["status"], "wrong_record");
    let body = json!({
        "protocol": "engineering-assurance.signature-verify-request/v1",
        "repository": fixture.repo().display().to_string(),
        "revision": "v-other",
        "kind": "tag",
        "allowed_signers": fixture.allowed().display().to_string(),
    });
    let output = run_command("signature-verify", body.to_string().as_bytes(), None);
    let mut plain: Value = serde_json::from_slice(&output.stdout).expect("json");
    plain.as_object_mut().expect("object").remove("protocol");
    assert_eq!(approval["signature"], plain);
}

#[test]
#[trace("TC-318", "FR-041-AC-2")]
fn tc_318_a_digest_differing_by_one_character_is_the_wrong_record() {
    let fixture = fixture();
    signed_tag(
        &fixture,
        "a",
        "v-digest",
        &good_message().replace(DIGEST, "sha256:0123456789abcdee"),
    );
    let wrong = status(&fixture, "v-digest");
    assert_eq!(wrong["status"], "wrong_record");
    assert_eq!(
        wrong["binding"],
        json!({"record_id": "match", "record_digest": "mismatch"})
    );
    assert!(wrong.get("approver").is_none());
    assert!(wrong.get("meaning").is_none());
    assert!(wrong.get("signature_kind").is_none());
    // A tag edited after signing is not the signed tag.
    signed_tag(&fixture, "a", "v-tampered", &good_message());
    rewrite_tag(&fixture, "v-tampered", |o| o.replace(DIGEST, "sha256:0"));
    let tampered = status(&fixture, "v-tampered");
    assert_eq!(tampered["status"], "signed_bad", "{tampered}");
    assert_eq!(tampered["binding"]["record_digest"], "mismatch");
}

#[test]
#[trace("TC-319", "FR-041-AC-3")]
fn tc_319_id_mismatch_missing_and_repeated_trailers_are_the_wrong_record() {
    let fixture = fixture();
    let id_line = format!("Approved-Record-Id: {RECORD_ID}");
    let digest_line = format!("Approved-Record-Digest: {DIGEST}");
    let digest_lower = digest_line.to_lowercase();
    let folded = format!("{digest_line}\n continued");
    let cases: [(&str, Vec<&str>, &str, &str); 10] = [
        (
            "v-id",
            vec!["Approved-Record-Id: other", &digest_line],
            "mismatch",
            "match",
        ),
        ("v-no-id", vec![&digest_line], "missing", "match"),
        ("v-no-digest", vec![&id_line], "match", "missing"),
        (
            "v-id-twice",
            vec![&id_line, &id_line, &digest_line],
            "repeated",
            "match",
        ),
        (
            "v-id-twice-one-wrong",
            vec![&id_line, "Approved-Record-Id: other", &digest_line],
            "repeated",
            "match",
        ),
        (
            "v-id-case",
            vec![&id_line, "approved-record-ID: other", &digest_line],
            "repeated",
            "match",
        ),
        (
            "v-digest-case",
            vec![&id_line, &digest_line, "APPROVED-RECORD-DIGEST: x"],
            "match",
            "repeated",
        ),
        (
            "v-digest-case-same",
            vec![&id_line, &digest_line, &digest_lower],
            "match",
            "repeated",
        ),
        ("v-folded", vec![&id_line, &folded], "match", "mismatch"),
        (
            "v-key-case-value-exact",
            vec!["approved-record-id: Change-Record-17", &digest_line],
            "mismatch",
            "match",
        ),
    ];
    for (name, lines, id, digest) in cases {
        let mut all = lines.clone();
        all.push("Signed-off-meaning: reviewed");
        signed_tag(&fixture, "a", name, &approval(&all));
        let wrong = status(&fixture, name);
        assert_eq!(wrong["status"], "wrong_record", "{name}: {wrong}");
        assert_eq!(
            wrong["binding"],
            json!({"record_id": id, "record_digest": digest}),
            "{name}"
        );
        assert!(wrong.get("approver").is_none());
    }
}

#[test]
#[trace("TC-320", "FR-041-AC-4")]
fn tc_320_a_good_signature_by_an_unlisted_key_is_an_untrusted_signer() {
    let fixture = fixture();
    signed_tag(&fixture, "b", "v-untrusted", &good_message());
    let untrusted = status(&fixture, "v-untrusted");
    assert_eq!(untrusted["status"], "untrusted_signer", "{untrusted}");
    assert!(untrusted.get("approver").is_none());
    assert_eq!(untrusted["signature"]["status"], "signed_untrusted");
    assert_eq!(untrusted["signature"]["signer"]["mapping"], "unmapped");
    assert_eq!(
        untrusted["binding"],
        json!({"record_id": "match", "record_digest": "match"})
    );
}

#[test]
#[trace("TC-321", "FR-041-AC-5")]
fn tc_321_a_missing_meaning_is_not_approved_and_an_empty_one_is() {
    let fixture = fixture();
    let id_line = format!("Approved-Record-Id: {RECORD_ID}");
    let digest_line = format!("Approved-Record-Digest: {DIGEST}");
    signed_tag(
        &fixture,
        "a",
        "v-no-meaning",
        &approval(&[&id_line, &digest_line]),
    );
    let missing = status(&fixture, "v-no-meaning");
    assert_eq!(missing["status"], "meaning_missing", "{missing}");
    assert!(missing.get("meaning").is_none());
    assert!(missing.get("approver").is_none());
    signed_tag(
        &fixture,
        "a",
        "v-empty-meaning",
        &approval(&[&id_line, &digest_line, "Signed-off-meaning:"]),
    );
    let empty = status(&fixture, "v-empty-meaning");
    assert_eq!(empty["status"], "approved", "{empty}");
    assert_eq!(empty["meaning"], json!([""]));
}

#[test]
#[trace("TC-322", "FR-041-AC-6")]
fn tc_322_bad_unsigned_lightweight_and_unresolved_tags_are_never_approved() {
    let fixture = fixture();
    let repo = fixture.repo();
    signed_tag(&fixture, "a", "v-bad", &good_message());
    rewrite_tag(&fixture, "v-bad", |o| o.replace("reviewed", "reviewes"));
    let bad = status(&fixture, "v-bad");
    assert_eq!(bad["status"], "signed_bad", "{bad}");
    assert_eq!(
        bad["binding"],
        json!({"record_id": "match", "record_digest": "match"})
    );

    git(
        &repo,
        None,
        &["tag", "-a", "-m", &good_message(), "v-plain"],
    );
    assert_eq!(status(&fixture, "v-plain")["status"], "unsigned");

    git(&repo, None, &["tag", "v-light"]);
    let light = status(&fixture, "v-light");
    assert_eq!(light["status"], "unsigned");
    assert_eq!(light["signature"]["reason"], "lightweight_tag");
    assert!(light.get("binding").is_none());

    for name in ["v-nothing", "main", "HEAD"] {
        let unresolved = status(&fixture, name);
        assert_eq!(unresolved["status"], "unverifiable", "{name}: {unresolved}");
        assert_eq!(unresolved["reason"], "revision_unresolved");
        assert_eq!(unresolved["signature"]["reason"], "revision_unresolved");
    }
    for name in ["v-bad", "v-plain", "v-light", "main"] {
        assert_ne!(status(&fixture, name)["status"], "approved");
    }
}

#[test]
#[trace("TC-323", "FR-041-AC-7")]
fn tc_323_trailers_on_the_commit_do_not_bind_and_a_commit_is_not_a_tag() {
    let fixture = fixture();
    let repo = fixture.repo();
    let public = fixture.public("a");
    let key = public.to_str().expect("utf-8");
    let commit_message = good_message();
    git(
        &repo,
        Some(key),
        &["commit", "-q", "-S", "--allow-empty", "-m", &commit_message],
    );
    signed_tag(&fixture, "a", "v-commit-trailers", "Approve the change");
    let tag = status(&fixture, "v-commit-trailers");
    assert_eq!(tag["status"], "wrong_record", "{tag}");
    assert_eq!(
        tag["binding"],
        json!({"record_id": "missing", "record_digest": "missing"})
    );
    // A signed commit and a signed merge commit, by branch name.
    git(&repo, None, &["branch", "side"]);
    git(&repo, None, &["checkout", "-q", "-b", "other", "HEAD~1"]);
    git(
        &repo,
        None,
        &["commit", "-q", "--allow-empty", "-m", "other"],
    );
    git(
        &repo,
        Some(key),
        &[
            "merge",
            "-q",
            "--no-ff",
            "-S",
            "-m",
            &commit_message,
            "side",
        ],
    );
    for name in ["main", "side", "other"] {
        let result = status(&fixture, name);
        assert_eq!(result["status"], "unverifiable", "{name}: {result}");
        assert_eq!(result["reason"], "revision_unresolved");
        assert_eq!(result["signature"]["reason"], "revision_unresolved");
    }
}

#[test]
#[trace("TC-324", "FR-041-AC-13")]
fn tc_324_a_message_of_only_trailer_lines_has_no_subject_and_binds_nothing() {
    let fixture = fixture();
    signed_tag(&fixture, "a", "v-only", &full_trailers().join("\n"));
    let only = status(&fixture, "v-only");
    assert_eq!(only["status"], "wrong_record", "{only}");
    assert_eq!(
        only["binding"],
        json!({"record_id": "missing", "record_digest": "missing"})
    );
    signed_tag(&fixture, "a", "v-subject", &good_message());
    let with_subject = status(&fixture, "v-subject");
    assert_eq!(with_subject["status"], "approved");
    assert_eq!(
        with_subject["binding"],
        json!({"record_id": "match", "record_digest": "match"})
    );
}

#[test]
#[trace("TC-325", "FR-041-AC-14")]
fn tc_325_the_trust_input_is_echoed_and_names_the_approver() {
    let fixture = fixture();
    // Mallory signs and the caller's own file maps her key.
    signed_tag(&fixture, "m", "v-mallory", &good_message());
    let by_default = status(&fixture, "v-mallory");
    assert_eq!(by_default["status"], "approved");
    assert_eq!(by_default["approver"], "mallory@example.test");
    assert_eq!(
        by_default["trust"],
        json!({"allowed_signers": fixture.allowed().display().to_string()})
    );
    // Another file names a different principal for the same key.
    let other = fixture.root().join("other_signers");
    let public = fs::read_to_string(fixture.public("m")).expect("key");
    fs::write(&other, format!("carol@example.test {public}")).expect("other file");
    let mut body = request(&fixture, "v-mallory");
    body["allowed_signers"] = json!(other.display().to_string());
    let remapped = result(&body);
    assert_eq!(remapped["status"], "approved");
    assert_eq!(remapped["approver"], "carol@example.test");
    assert_eq!(
        remapped["trust"],
        json!({"allowed_signers": other.display().to_string()})
    );
    // The trust input is echoed for statuses other than approved too.
    signed_tag(&fixture, "b", "v-untrusted", &good_message());
    let untrusted = status(&fixture, "v-untrusted");
    assert_eq!(
        untrusted["trust"]["allowed_signers"],
        json!(fixture.allowed().display().to_string())
    );
}

#[test]
#[trace("TC-326", "FR-041-AC-15")]
fn tc_326_a_message_git_cannot_read_is_unverifiable_not_a_wrong_record() {
    let fixture = fixture();
    signed_tag(&fixture, "a", "v-signed", &good_message());
    let shim = fixture.root().join("shim-bin");
    fs::create_dir(&shim).expect("shim dir");
    let real = std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|dir| dir.join("git"))
        .find(|candidate| candidate.is_file())
        .expect("git");
    let ssh_keygen = std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|dir| dir.join("ssh-keygen"))
        .find(|candidate| candidate.is_file())
        .expect("ssh-keygen");
    // Every git command but the trailer reader runs for real.
    let script = format!(
        "#!/bin/sh\nfor a in \"$@\"; do [ \"$a\" = interpret-trailers ] && exit 1; done\nexec {} \"$@\"\n",
        real.display()
    );
    fs::write(shim.join("git"), script).expect("shim");
    fs::set_permissions(shim.join("git"), fs::Permissions::from_mode(0o755)).expect("mode");
    std::os::unix::fs::symlink(ssh_keygen, shim.join("ssh-keygen")).expect("link");
    let unreadable = result_on(&request(&fixture, "v-signed"), Some(&shim));
    assert_eq!(unreadable["status"], "unverifiable", "{unreadable}");
    assert_eq!(unreadable["reason"], "trailer_unreadable");
    assert_eq!(unreadable["signature"]["status"], "signed_valid");
    assert!(unreadable.get("binding").is_none());
}

#[test]
#[trace("TC-327", "FR-041-AC-10")]
fn tc_327_invalid_requests_are_machine_errors_and_never_start_git() {
    let fixture = fixture();
    signed_tag(&fixture, "a", "v-signed", &good_message());
    let marker = fixture.root().join("git-was-started");
    let shim = fixture.root().join("shim-bin");
    fs::create_dir(&shim).expect("shim dir");
    fs::write(
        shim.join("git"),
        format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()),
    )
    .expect("shim");
    fs::set_permissions(shim.join("git"), fs::Permissions::from_mode(0o755)).expect("mode");
    let gitsign = |value: Value| json!({"gitsign": value});
    let long = "a".repeat(257);
    let mutations: Vec<(&str, Value)> = vec![
        ("unknown", json!({"extra": 1})),
        ("protocol", json!({"protocol": "wrong"})),
        ("empty id", json!({"record_id": ""})),
        ("long id", json!({"record_id": long})),
        ("dash id", json!({"record_id": "-id"})),
        ("control id", json!({"record_id": "a\nb"})),
        ("empty digest", json!({"record_digest": ""})),
        ("long digest", json!({"record_digest": long})),
        ("dash digest", json!({"record_digest": "-d"})),
        ("control digest", json!({"record_digest": "a\u{7}b"})),
        ("relative repo", json!({"repository": "repo"})),
        ("relative signers", json!({"allowed_signers": "signers"})),
        ("dash tag", json!({"tag": "--help"})),
        ("tag space", json!({"tag": "v 1"})),
        ("gitsign empty", gitsign(json!({}))),
        (
            "gitsign dash",
            gitsign(json!({
                "certificate_identity": "-x",
                "certificate_oidc_issuer": concat!("https:", "//issuer.example.test"),
                "principal": "p",
            })),
        ),
    ];
    for (label, mutation) in mutations {
        let mut body = request(&fixture, "v-signed");
        for (key, value) in mutation.as_object().expect("object") {
            body[key] = value.clone();
        }
        let output = run_command("approval-verify", body.to_string().as_bytes(), Some(&shim));
        assert_eq!(output.status.code(), Some(2), "{label}: {output:?}");
        let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
        assert_eq!(error["protocol"], "engineering-assurance.error/v1");
        assert_eq!(error["capability"], "approval-verify");
        assert_eq!(error["code"], "approval_verify_request_invalid", "{label}");
        assert!(!output.stderr.is_empty());
    }
    let output = run_command("approval-verify", b"{not json", Some(&shim));
    assert_eq!(output.status.code(), Some(2));
    let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
    assert_eq!(error["code"], "approval_verify_request_unreadable");
    let mut body = request(&fixture, "v-signed");
    body["allowed_signers"] = json!(fixture.root().join("missing").display().to_string());
    let output = run_command("approval-verify", body.to_string().as_bytes(), Some(&shim));
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
    assert_eq!(error["code"], "approval_verify_allowed_signers_invalid");
    assert!(!marker.exists(), "a refused request must not start git");
}

/// Signs `message` into an annotated tag object by hand (`ssh-keygen -Y sign`,
/// as git does), so the message may hold bytes `git tag -m` cannot take.
fn forged_signed_tag(fixture: &Fixture, key: &str, name: &str, message: &[u8]) {
    let repo = fixture.repo();
    let commit = git(&repo, None, &["rev-parse", "HEAD"]);
    let mut payload = format!(
        "object {}\ntype commit\ntag {name}\ntagger t <t@example.invalid> 1700000000 +0000\n\n",
        commit.trim()
    )
    .into_bytes();
    payload.extend_from_slice(message);
    let path = fixture.root().join(format!("{name}.payload"));
    fs::write(&path, &payload).expect("payload");
    let private = fixture.root().join(key);
    run_program(
        "ssh-keygen",
        fixture.root(),
        &[
            "-Y",
            "sign",
            "-n",
            "git",
            "-f",
            private.to_str().expect("utf-8"),
            path.to_str().expect("utf-8"),
        ],
        None,
    );
    let mut object = payload;
    object.extend(fs::read(fixture.root().join(format!("{name}.payload.sig"))).expect("signature"));
    let id = run_program(
        "git",
        &repo,
        &["hash-object", "-t", "tag", "-w", "--literally", "--stdin"],
        Some(&object),
    );
    let id = String::from_utf8(id).expect("utf-8");
    git(
        &repo,
        None,
        &["update-ref", &format!("refs/tags/{name}"), id.trim()],
    );
}

#[test]
#[trace("TC-330", "FR-041-AC-15", "FR-041-CON-1")]
fn tc_330_a_nul_or_non_utf8_message_is_unreadable_and_never_approves() {
    let fixture = fixture();
    // The digest a trusting reader would see before the NUL is the request's.
    let nul = format!(
        "Approve the change\n\nApproved-Record-Id: {RECORD_ID}\nApproved-Record-Digest: {DIGEST}\0EXTRA\nSigned-off-meaning: reviewed\n"
    );
    forged_signed_tag(&fixture, "a", "v-nul", nul.as_bytes());
    // A lossy decode would turn the byte 0xFF into the requested U+FFFD.
    let mut invalid =
        format!("Approve the change\n\nApproved-Record-Digest: {DIGEST}\nApproved-Record-Id: id-")
            .into_bytes();
    invalid.extend_from_slice(b"\xff\nSigned-off-meaning: reviewed\n");
    forged_signed_tag(&fixture, "a", "v-invalid", &invalid);
    for (name, id) in [("v-nul", RECORD_ID), ("v-invalid", "id-\u{fffd}")] {
        let mut body = request(&fixture, name);
        body["record_id"] = json!(id);
        let unreadable = result(&body);
        assert_eq!(unreadable["status"], "unverifiable", "{name}: {unreadable}");
        assert_eq!(unreadable["reason"], "trailer_unreadable");
        assert_eq!(unreadable["signature"]["status"], "signed_valid");
        assert!(unreadable.get("binding").is_none());
        assert!(unreadable.get("approver").is_none());
    }
}
