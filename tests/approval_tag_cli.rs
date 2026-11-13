// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary tests for `approval-tag` (FR-042) on real repositories. SSH
//! tags are signed with fictional ed25519 keys generated inside each test and
//! verified by `approval-verify`. gitsign tags are signed by a stand-in signing
//! program that emits a CMS-shaped block and verified with the FR-040 stand-in
//! `gitsign`; the identities are fictional.

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
const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const TAG_PROTOCOL: &str = "engineering-assurance.approval-tag-request/v1";
const TAG_RESULT: &str = "engineering-assurance.approval-tag-result/v1";
const VERIFY_PROTOCOL: &str = "engineering-assurance.approval-verify-request/v1";
const ALICE: &str = "alice@example.test";
const BOB: &str = "bob@example.test";
const IDENTITY: &str = "carol@example.test";
const ISSUER: &str = concat!("https:", "//issuer.example.test");
const PRINCIPAL: &str = "Carol Example";

/// Verifies as the FR-040 stand-in does: exits 0 unless `exit_exact_exact` says
/// otherwise.
const GITSIGN: &str = "#!/bin/sh\nexit 0\n";

/// A stand-in signing program for `gpg.x509.program`: records its arguments,
/// exits 1 when a `fail` file sits beside it, and otherwise reports a created
/// signature on the status descriptor and prints a CMS-shaped block. Git accepts
/// a signature only when `SIG_CREATED` follows an earlier status line, as it does
/// from gpg, so the stand-in states the key it considered first; without that
/// line git accepted the signature only when its read of the status pipe
/// happened to split the output, which made the tests fail under load. The
/// signer runs with a `PATH` holding only the fixture's `bin`, so it drains its
/// input with shell builtins: with no `cat` to run, it exited without reading,
/// and git failed with a broken pipe whenever it wrote the message after that.
const SIGNER: &str = r#"#!/bin/sh
d=${0%/*}
echo "$@" > "$d/sign_args"
while read -r line || [ -n "$line" ]; do :; done
if [ -e "$d/fail" ]; then exit 1; fi
echo "[GNUPG:] KEY_CONSIDERED AAAA 0" >&2
echo "[GNUPG:] SIG_CREATED D 1 8 00 1700000000 00 AAAA" >&2
printf -- '-----BEGIN SIGNED MESSAGE-----\nAAAA\n-----END SIGNED MESSAGE-----\n'
"#;

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

    fn bin(&self) -> PathBuf {
        self.root().join("bin")
    }

    fn global(&self) -> PathBuf {
        self.root().join("gitconfig")
    }

    fn record(&self) -> PathBuf {
        self.root().join("records").join(RECORD_ID).join("1.json")
    }

    fn public(&self, key: &str) -> PathBuf {
        self.root().join(format!("{key}.pub"))
    }

    /// Writes the caller's global git configuration: an identity and `body`.
    fn config(&self, body: &str) {
        fs::write(
            self.global(),
            format!("[user]\n\tname = t\n\temail = t@example.test\n{body}"),
        )
        .expect("global config");
    }

    fn ssh_config(&self, key: &str) {
        self.config(&format!(
            "[gpg]\n\tformat = ssh\n[user]\n\tsigningkey = {}\n",
            self.public(key).display()
        ));
    }

    fn x509_config(&self) {
        self.config(&format!(
            "[gpg]\n\tformat = x509\n[gpg \"x509\"]\n\tprogram = {}\n",
            self.bin().join("sign-standin").display()
        ));
    }

    fn executable(&self, name: &str, body: &str) {
        let path = self.bin().join(name);
        fs::write(&path, body).expect("executable");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("mode");
    }
}

/// A schema 2 change-assurance record as the store holds it: no digest member.
fn record_body() -> Value {
    json!({
        "schema_version": 2,
        "record_type": "change_assurance",
        "record_id": RECORD_ID,
        "revision": 1,
        "subject": {"repository": "example", "base_revision": "main"},
        "review_workflow": {
            "run_id": "run-1",
            "decision_event_kind": "change_assurance.review_decided",
            "signed_approval": {"required": false},
        },
    })
}

fn which(program: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
        .expect("program on PATH")
}

fn run_program(program: &str, directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new(program)
        .args(arguments)
        .current_dir(directory)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_DIR")
        .output()
        .expect("program must start");
    assert!(
        output.status.success(),
        "{program} {arguments:?}: {output:?}"
    );
    String::from_utf8(output.stdout).expect("utf-8")
}

fn git(repo: &Path, arguments: &[&str]) -> String {
    let mut all = vec!["-c", "user.name=t", "-c", "user.email=t@example.test"];
    all.extend_from_slice(arguments);
    run_program("git", repo, &all)
}

/// A repository with one commit and a bare remote, keys `a` (alice), `b` (bob),
/// a record file, an allowed-signers file mapping both keys, and a `bin`
/// directory with `git`, `ssh-keygen`, the stand-in `gitsign` and the stand-in
/// signing program. The global configuration starts empty of signing settings.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = Fixture { dir };
    let root = fixture.root();
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
        );
    }
    let public = |key: &str| fs::read_to_string(root.join(format!("{key}.pub"))).expect("key");
    fs::write(
        root.join("allowed_signers"),
        format!("{ALICE} {}{BOB} {}", public("a"), public("b")),
    )
    .expect("allowed signers");
    fs::create_dir(fixture.bin()).expect("bin");
    fixture.executable(
        "git",
        &format!("#!/bin/sh\nexec {} \"$@\"\n", which("git").display()),
    );
    fixture.executable(
        "ssh-keygen",
        &format!("#!/bin/sh\nexec {} \"$@\"\n", which("ssh-keygen").display()),
    );
    fixture.executable("gitsign", GITSIGN);
    fixture.executable("sign-standin", SIGNER);
    fs::create_dir_all(fixture.record().parent().expect("parent")).expect("records");
    fs::write(fixture.record(), record_body().to_string()).expect("record");
    let remote = root.join("remote.git");
    fs::create_dir(&remote).expect("remote dir");
    git(&remote, &["init", "-q", "--bare"]);
    let repo = fixture.repo();
    fs::create_dir(&repo).expect("repo dir");
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "base"]);
    git(
        &repo,
        &["remote", "add", "origin", remote.to_str().expect("utf-8")],
    );
    fixture.config("");
    fixture
}

fn tag_request(fixture: &Fixture, tag: &str, meaning: &str) -> Value {
    json!({
        "protocol": TAG_PROTOCOL,
        "repository": fixture.repo().display().to_string(),
        "tag": tag,
        "record": fixture.record().display().to_string(),
        "record_id": RECORD_ID,
        "record_digest": DIGEST,
        "meaning": meaning,
    })
}

/// Runs `subcommand` with the caller's environment: `path` as `PATH`, a home
/// directory, and the fixture's global git configuration.
fn run(fixture: &Fixture, subcommand: &str, body: &[u8], path: &Path) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args([subcommand, "--request", "-"])
        .current_dir(fixture.repo())
        .env_clear()
        .env("PATH", path)
        .env("HOME", fixture.root())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", fixture.global())
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

fn create(fixture: &Fixture, request: &Value) -> Output {
    run(
        fixture,
        "approval-tag",
        request.to_string().as_bytes(),
        &fixture.bin(),
    )
}

/// A created tag: exit 0 and one `created` result line.
fn created(fixture: &Fixture, request: &Value) -> Value {
    let output = create(fixture, request);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = std::str::from_utf8(&output.stdout).expect("utf-8");
    assert!(
        text.ends_with('\n') && text.matches('\n').count() == 1,
        "{text:?}"
    );
    let value: Value = serde_json::from_str(text).expect("json line");
    assert_eq!(value["protocol"], TAG_RESULT);
    assert_eq!(value["status"], "created");
    value
}

/// A refusal: exit 2, the machine error with `code`, a message, and no tag.
fn refused(fixture: &Fixture, request: &Value, code: &str) -> (Value, String) {
    let output = create(fixture, request);
    assert_eq!(output.status.code(), Some(2), "{code}: {output:?}");
    let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
    assert_eq!(error["protocol"], "engineering-assurance.error/v1");
    assert_eq!(error["capability"], "approval-tag");
    assert_eq!(error["code"], code, "{error}");
    let message = String::from_utf8(output.stderr).expect("utf-8");
    assert!(!message.is_empty());
    assert_eq!(tags(fixture), "", "a refusal must not write a tag");
    (error, message)
}

fn tags(fixture: &Fixture) -> String {
    git(&fixture.repo(), &["tag", "-l"])
}

fn verify_request(fixture: &Fixture, tag: &str, gitsign: bool) -> Value {
    let mut value = json!({
        "protocol": VERIFY_PROTOCOL,
        "repository": fixture.repo().display().to_string(),
        "tag": tag,
        "record_id": RECORD_ID,
        "record_digest": DIGEST,
        "allowed_signers": fixture.root().join("allowed_signers").display().to_string(),
    });
    if gitsign {
        value["gitsign"] = json!({
            "certificate_identity": IDENTITY,
            "certificate_oidc_issuer": ISSUER,
            "principal": PRINCIPAL,
        });
    }
    value
}

/// The `approval-verify` result for `tag`.
fn verified(fixture: &Fixture, tag: &str, gitsign: bool) -> Value {
    let request = verify_request(fixture, tag, gitsign);
    let output = run(
        fixture,
        "approval-verify",
        request.to_string().as_bytes(),
        &fixture.bin(),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    serde_json::from_slice(&output.stdout).expect("json line")
}

#[test]
#[trace("TC-341", "FR-042-AC-1", "FR-042-AC-3")]
fn tc_341_an_ssh_signed_tag_is_approved_by_approval_verify() {
    let fixture = fixture();
    fixture.ssh_config("a");
    let result = created(
        &fixture,
        &tag_request(&fixture, "approve-17", "reviewed for release"),
    );
    assert_eq!(result["tag"], "approve-17");
    assert_eq!(result["record_id"], RECORD_ID);
    assert_eq!(result["record_digest"], DIGEST);
    assert_eq!(result["meaning"], "reviewed for release");
    let head = git(&fixture.repo(), &["rev-parse", "HEAD"]);
    assert_eq!(result["target"], head.trim());
    let object = git(&fixture.repo(), &["rev-parse", "refs/tags/approve-17"]);
    assert_eq!(result["tag_object"], object.trim());
    assert_eq!(
        git(&fixture.repo(), &["cat-file", "-t", object.trim()]),
        "tag\n"
    );

    let approved = verified(&fixture, "approve-17", false);
    assert_eq!(approved["status"], "approved", "{approved}");
    assert_eq!(approved["approver"], ALICE);
    assert_eq!(approved["meaning"], json!(["reviewed for release"]));
    assert_eq!(approved["signature_kind"], "ssh");

    let message = git(&fixture.repo(), &["cat-file", "tag", object.trim()]);
    let body = message.split_once("\n\n").expect("headers then message").1;
    let body = body
        .split("-----BEGIN SSH SIGNATURE-----")
        .next()
        .expect("message");
    assert_eq!(
        body,
        format!(
            "Approve change-assurance record {RECORD_ID}\n\nApproved-Record-Id: {RECORD_ID}\nApproved-Record-Digest: {DIGEST}\nSigned-off-meaning: reviewed for release\n"
        )
    );
}

#[test]
#[trace("TC-342", "FR-042-AC-2")]
fn tc_342_a_gitsign_style_tag_is_approved_with_the_stand_in_gitsign() {
    let fixture = fixture();
    fixture.x509_config();
    created(&fixture, &tag_request(&fixture, "approve-17", "reviewed"));
    let arguments = fs::read_to_string(fixture.bin().join("sign_args")).expect("signer ran");
    assert!(arguments.contains("-bsau"), "{arguments}");
    let approved = verified(&fixture, "approve-17", true);
    assert_eq!(approved["status"], "approved", "{approved}");
    assert_eq!(approved["signature_kind"], "gitsign");
    assert_eq!(approved["approver"], PRINCIPAL);
    assert_eq!(approved["meaning"], json!(["reviewed"]));
}

#[test]
#[trace("TC-343", "FR-042-AC-4")]
fn tc_343_the_meaning_must_be_given_and_is_never_defaulted() {
    let fixture = fixture();
    fixture.ssh_config("a");
    let long = "m".repeat(257);
    for meaning in [
        "",
        " ",
        " reviewed",
        "reviewed ",
        "re\nviewed",
        "re\u{7}viewed",
        &long,
    ] {
        refused(
            &fixture,
            &tag_request(&fixture, "approve-17", meaning),
            "approval_tag_request_invalid",
        );
    }
    let mut missing = tag_request(&fixture, "approve-17", "x");
    missing.as_object_mut().expect("object").remove("meaning");
    refused(&fixture, &missing, "approval_tag_request_invalid");

    created(&fixture, &tag_request(&fixture, "approve-17", "reviewed"));
    let approved = verified(&fixture, "approve-17", false);
    assert_eq!(approved["meaning"], json!(["reviewed"]));
}

fn snapshot(fixture: &Fixture) -> (String, Vec<u8>, Vec<u8>, String) {
    let repo = fixture.repo();
    (
        git(&repo, &["for-each-ref"]),
        fs::read(fixture.global()).expect("global config"),
        fs::read(repo.join(".git").join("config")).expect("repository config"),
        git(&repo, &["rev-parse", "HEAD"]),
    )
}

#[test]
#[trace("TC-344", "FR-042-AC-5", "FR-042-CON-1")]
fn tc_344_the_configured_key_signs_and_nothing_else_changes() {
    let fixture = fixture();
    let before = snapshot(&fixture);
    fixture.ssh_config("a");
    let global_a = fs::read(fixture.global()).expect("global config");
    created(&fixture, &tag_request(&fixture, "by-alice", "reviewed"));
    assert_eq!(fs::read(fixture.global()).expect("global"), global_a);
    fixture.ssh_config("b");
    let global_b = fs::read(fixture.global()).expect("global config");
    created(&fixture, &tag_request(&fixture, "by-bob", "reviewed"));
    assert_eq!(fs::read(fixture.global()).expect("global"), global_b);
    assert_eq!(verified(&fixture, "by-alice", false)["approver"], ALICE);
    assert_eq!(verified(&fixture, "by-bob", false)["approver"], BOB);

    let after = snapshot(&fixture);
    assert_eq!(after.2, before.2, "repository configuration is unchanged");
    assert_eq!(after.3, before.3, "no commit is made");
    // Every earlier ref is unchanged; exactly the two new tag refs are added.
    for old in before.0.lines() {
        assert!(
            after.0.lines().any(|line| line == old),
            "ref changed: {old}"
        );
    }
    let new_refs: Vec<&str> = after
        .0
        .lines()
        .filter(|line| !before.0.lines().any(|old| old == *line))
        .collect();
    assert_eq!(new_refs.len(), 2, "{new_refs:?}");
    assert!(
        new_refs
            .iter()
            .all(|line| line.contains(" tag\trefs/tags/by-"))
    );
    let remote = fixture.root().join("remote.git");
    assert_eq!(git(&remote, &["for-each-ref"]), "", "nothing is pushed");
}

#[test]
#[trace("TC-345", "FR-042-AC-6")]
fn tc_345_missing_signing_configuration_is_refused_naming_the_setting() {
    let fixture = fixture();
    let request = tag_request(&fixture, "approve-17", "reviewed");
    let cases = [
        ("", "gpg.format"),
        ("[gpg]\n\tformat = openpgp\n", "gpg.format"),
        ("[gpg]\n\tformat = \n", "gpg.format"),
        ("[gpg]\n\tformat = ssh\n", "user.signingkey"),
        (
            "[gpg]\n\tformat = ssh\n[user]\n\tsigningkey = \n",
            "user.signingkey",
        ),
        ("[gpg]\n\tformat = x509\n", "gpg.x509.program"),
    ];
    for (body, setting) in cases {
        fixture.config(body);
        let (error, message) = refused(&fixture, &request, "approval_tag_signing_unconfigured");
        assert!(message.contains(setting), "{body:?}: {message}");
        assert!(
            error["message"]
                .as_str()
                .expect("message")
                .contains(setting)
        );
    }
    // A repository-local setting counts, as it does for `git tag -s`.
    fixture.config("");
    git(&fixture.repo(), &["config", "--local", "gpg.format", "ssh"]);
    refused(&fixture, &request, "approval_tag_signing_unconfigured");
    git(
        &fixture.repo(),
        &[
            "config",
            "--local",
            "user.signingkey",
            fixture.public("a").to_str().expect("utf-8"),
        ],
    );
    created(&fixture, &request);
}

#[test]
#[trace("TC-346", "FR-042-AC-7")]
fn tc_346_a_record_that_does_not_resolve_is_refused() {
    let fixture = fixture();
    fixture.ssh_config("a");
    let write = |body: &str| fs::write(fixture.record(), body).expect("record");
    let valid = |edit: &dyn Fn(&mut Value)| {
        let mut value = record_body();
        edit(&mut value);
        value.to_string()
    };
    let mut other_id = tag_request(&fixture, "approve-17", "reviewed");
    other_id["record_id"] = json!("another-record");
    refused(&fixture, &other_id, "approval_tag_record_unresolved");

    let bodies = [
        "{not json".to_owned(),
        "[]".to_owned(),
        "\"change_assurance\"".to_owned(),
        valid(&|v| v["schema_version"] = json!(1)),
        valid(&|v| v["schema_version"] = json!("2")),
        valid(&|v| {
            v.as_object_mut().expect("object").remove("schema_version");
        }),
        valid(&|v| v["record_type"] = json!("other")),
        valid(&|v| {
            v.as_object_mut().expect("object").remove("record_type");
        }),
        valid(&|v| v["record_id"] = json!("another-record")),
        valid(&|v| {
            v["review_workflow"]
                .as_object_mut()
                .expect("object")
                .remove("signed_approval");
        }),
        valid(&|v| {
            v.as_object_mut().expect("object").remove("review_workflow");
        }),
        format!("\u{feff}{}", valid(&|_| {})),
    ];
    for body in bodies {
        write(&body);
        refused(
            &fixture,
            &tag_request(&fixture, "approve-17", "reviewed"),
            "approval_tag_record_unresolved",
        );
    }
    write(&" ".repeat(9 * 1024 * 1024));
    refused(
        &fixture,
        &tag_request(&fixture, "approve-17", "reviewed"),
        "approval_tag_record_unresolved",
    );
    fs::remove_file(fixture.record()).expect("remove");
    refused(
        &fixture,
        &tag_request(&fixture, "approve-17", "reviewed"),
        "approval_tag_record_unresolved",
    );
    run_program(
        "mkfifo",
        fixture.root(),
        &[fixture.record().to_str().expect("utf-8")],
    );
    refused(
        &fixture,
        &tag_request(&fixture, "approve-17", "reviewed"),
        "approval_tag_record_unresolved",
    );
    fs::remove_file(fixture.record()).expect("remove fifo");
    fs::create_dir(fixture.record()).expect("directory");
    refused(
        &fixture,
        &tag_request(&fixture, "approve-17", "reviewed"),
        "approval_tag_record_unresolved",
    );
    fs::remove_dir(fixture.record()).expect("remove");
    write(&valid(&|_| {}));
    created(&fixture, &tag_request(&fixture, "approve-17", "reviewed"));
}

#[test]
#[trace("TC-347", "FR-042-AC-8")]
fn tc_347_bad_and_existing_tag_names_and_bad_revisions_are_refused() {
    let fixture = fixture();
    fixture.ssh_config("a");
    for name in [
        "a..b",
        "HEAD",
        "@",
        "heads/main",
        "main",
        "a~b",
        "a^b",
        "a:b",
        "a?b",
        "a*b",
        "a[b",
        "a.lock",
        "a@{b",
        "/a",
        "a/",
        "refs/tags/x",
        "refs/x",
    ] {
        refused(
            &fixture,
            &tag_request(&fixture, name, "reviewed"),
            "approval_tag_tag_invalid",
        );
    }
    let mut bad_revision = tag_request(&fixture, "approve-17", "reviewed");
    bad_revision["revision"] = json!("no-such-branch");
    refused(&fixture, &bad_revision, "approval_tag_revision_unresolved");

    git(&fixture.repo(), &["tag", "-a", "-m", "taken", "approve-17"]);
    let taken = git(&fixture.repo(), &["rev-parse", "refs/tags/approve-17"]);
    let output = create(&fixture, &tag_request(&fixture, "approve-17", "reviewed"));
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
    assert_eq!(error["code"], "approval_tag_tag_exists");
    assert_eq!(
        git(&fixture.repo(), &["rev-parse", "refs/tags/approve-17"]),
        taken
    );
    assert_eq!(tags(&fixture), "approve-17\n");

    let mut named = tag_request(&fixture, "approve-18", "reviewed");
    named["revision"] = json!("main");
    let result = created(&fixture, &named);
    assert_eq!(
        result["target"],
        git(&fixture.repo(), &["rev-parse", "main"]).trim()
    );
}

#[test]
#[trace("TC-348", "FR-042-AC-9")]
fn tc_348_invalid_requests_are_machine_errors_and_never_start_git() {
    let fixture = fixture();
    fixture.ssh_config("a");
    let marker = fixture.root().join("git-was-started");
    let shim = fixture.root().join("shim-bin");
    fs::create_dir(&shim).expect("shim dir");
    fs::write(
        shim.join("git"),
        format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()),
    )
    .expect("shim");
    fs::set_permissions(shim.join("git"), fs::Permissions::from_mode(0o755)).expect("mode");
    let long = "a".repeat(257);
    let mutations: Vec<(&str, Value)> = vec![
        ("unknown", json!({"extra": 1})),
        ("protocol", json!({"protocol": "wrong"})),
        ("relative repo", json!({"repository": "repo"})),
        ("relative record", json!({"record": "record.json"})),
        ("empty id", json!({"record_id": ""})),
        ("long id", json!({"record_id": long})),
        ("dash id", json!({"record_id": "-id"})),
        ("control id", json!({"record_id": "a\nb"})),
        ("padded id", json!({"record_id": " id"})),
        (
            "missing prefix",
            json!({"record_digest": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}),
        ),
        ("short digest", json!({"record_digest": "sha256:0123"})),
        (
            "upper digest",
            json!({"record_digest": format!("sha256:{}", "A".repeat(64))}),
        ),
        ("empty tag", json!({"tag": ""})),
        ("dash tag", json!({"tag": "--force"})),
        ("tag space", json!({"tag": "v 1"})),
        ("dash revision", json!({"revision": "--all"})),
        ("revision space", json!({"revision": "a b"})),
        ("number meaning", json!({"meaning": 1})),
    ];
    for (label, mutation) in mutations {
        let mut body = tag_request(&fixture, "approve-17", "reviewed");
        for (key, value) in mutation.as_object().expect("object") {
            body[key] = value.clone();
        }
        let output = run(&fixture, "approval-tag", body.to_string().as_bytes(), &shim);
        assert_eq!(output.status.code(), Some(2), "{label}: {output:?}");
        let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
        assert_eq!(error["capability"], "approval-tag");
        assert_eq!(error["code"], "approval_tag_request_invalid", "{label}");
        assert!(!output.stderr.is_empty());
    }
    let output = run(&fixture, "approval-tag", b"{not json", &shim);
    assert_eq!(output.status.code(), Some(2));
    let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
    assert_eq!(error["code"], "approval_tag_request_unreadable");
    assert!(!marker.exists(), "a refused request must not start git");
}

#[test]
#[trace("TC-349", "FR-042-AC-10")]
fn tc_349_a_relative_path_entry_in_the_repository_is_never_run() {
    let fixture = fixture();
    fixture.config("[gpg]\n\tformat = x509\n[gpg \"x509\"]\n\tprogram = sign-standin\n");
    let marker = fixture.root().join("shim-was-run");
    let relative = fixture.repo().join("relbin");
    fs::create_dir(&relative).expect("relbin");
    for name in ["git", "sign-standin"] {
        fs::write(
            relative.join(name),
            format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()),
        )
        .expect("shim");
        fs::set_permissions(relative.join(name), fs::Permissions::from_mode(0o755)).expect("mode");
    }
    let body = tag_request(&fixture, "approve-17", "reviewed").to_string();

    let only_relative = run(
        &fixture,
        "approval-tag",
        body.as_bytes(),
        Path::new("relbin"),
    );
    assert_eq!(only_relative.status.code(), Some(2), "{only_relative:?}");
    let error: Value = serde_json::from_slice(&only_relative.stdout).expect("error json");
    assert_eq!(error["code"], "approval_tag_git_unavailable");
    assert_eq!(tags(&fixture), "");

    let mixed = std::env::join_paths([Path::new("relbin"), fixture.bin().as_path()]).expect("path");
    let output = run(&fixture, "approval-tag", body.as_bytes(), Path::new(&mixed));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        fixture.bin().join("sign_args").exists(),
        "the absolute signer ran"
    );
    assert!(!marker.exists(), "nothing in the repository may run");
}

#[test]
#[trace("TC-350", "FR-042-AC-11")]
fn tc_350_a_failing_signer_leaves_no_tag_for_x509_and_for_ssh() {
    let fixture = fixture();
    fixture.x509_config();
    fs::write(fixture.bin().join("fail"), "").expect("fail flag");
    refused(
        &fixture,
        &tag_request(&fixture, "approve-17", "reviewed"),
        "approval_tag_create_failed",
    );
    fs::remove_file(fixture.bin().join("fail")).expect("remove flag");
    created(&fixture, &tag_request(&fixture, "approve-17", "reviewed"));

    // Git 2.34 exits 0 and writes an unsigned tag when an SSH signature fails:
    // a missing key file, and a passphrase-protected key with no agent.
    let fixture = self::fixture();
    let locked = fixture.root().join("locked");
    run_program(
        "ssh-keygen",
        fixture.root(),
        &[
            "-q",
            "-t",
            "ed25519",
            "-N",
            "secret",
            "-f",
            locked.to_str().expect("utf-8"),
        ],
    );
    for key in [
        fixture.root().join("missing.pub"),
        fixture.root().join("locked.pub"),
    ] {
        fixture.config(&format!(
            "[gpg]\n\tformat = ssh\n[user]\n\tsigningkey = {}\n",
            key.display()
        ));
        refused(
            &fixture,
            &tag_request(&fixture, "approve-17", "reviewed"),
            "approval_tag_create_failed",
        );
    }
    fixture.ssh_config("a");
    created(&fixture, &tag_request(&fixture, "approve-17", "reviewed"));
}
