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
const REGISTRY: &str = "HEAD:identity-registry.json";

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
        });
        value["identity_registry"] = json!(REGISTRY);
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
    // The registry that names the approver is committed in the repository.
    let registry = json!({
        "protocol": "engineering-assurance.gitsign-identity-registry/v1",
        "entries": [{
            "certificate_identity": IDENTITY,
            "certificate_oidc_issuer": ISSUER,
            "principal": PRINCIPAL,
        }],
    });
    fs::write(
        fixture.repo().join("identity-registry.json"),
        registry.to_string(),
    )
    .expect("registry");
    git(&fixture.repo(), &["add", "identity-registry.json"]);
    git(
        &fixture.repo(),
        &["commit", "-q", "-m", "identity registry"],
    );
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

// ---------------------------------------------------------------------------
// FR-085, FR-086: the declared gesture and the interactive signing run.
// ---------------------------------------------------------------------------

/// As [`run`], with extra environment variables for the CLI.
fn run_env(
    fixture: &Fixture,
    subcommand: &str,
    body: &[u8],
    path: &Path,
    extra: &[(&str, &Path)],
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"));
    command
        .args([subcommand, "--request", "-"])
        .current_dir(fixture.repo())
        .env_clear()
        .env("PATH", path)
        .env("HOME", fixture.root())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", fixture.global())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in extra {
        command.env(name, value);
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

fn gesture_request(fixture: &Fixture, tag: &str, gesture: &str) -> Value {
    let mut request = tag_request(fixture, tag, "reviewed");
    request["gesture"] = json!(gesture);
    request
}

fn tag_message(fixture: &Fixture, tag: &str) -> String {
    let object = git(&fixture.repo(), &["cat-file", "tag", tag]);
    let body = object.split_once("\n\n").expect("headers then message").1;
    body.split("-----BEGIN SSH SIGNATURE-----")
        .next()
        .expect("message")
        .to_owned()
}

#[test]
#[trace("TC-1149", "FR-085-AC-1", "FR-086-AC-1")]
fn tc_1149_a_request_without_gesture_keeps_three_trailers_and_declares_none() {
    let fixture = fixture();
    fixture.ssh_config("a");
    let result = created(&fixture, &tag_request(&fixture, "approve-17", "reviewed"));
    assert!(result.get("gesture").is_none(), "{result}");
    assert_eq!(
        tag_message(&fixture, "approve-17"),
        format!(
            "Approve change-assurance record {RECORD_ID}\n\nApproved-Record-Id: {RECORD_ID}\nApproved-Record-Digest: {DIGEST}\nSigned-off-meaning: reviewed\n"
        )
    );
    let approved = verified(&fixture, "approve-17", false);
    assert_eq!(approved["status"], "approved", "{approved}");
    assert_eq!(approved["gesture"], json!({"declared": "none"}));
    assert_eq!(
        approved["signature"]["gesture"],
        json!({"evidenced": "none"})
    );
}

#[test]
#[trace("TC-1150", "FR-085-AC-3", "FR-085-AC-8", "FR-086-AC-3", "FR-086-AC-8")]
fn tc_1150_a_declared_gesture_is_copied_verbatim_and_never_changes_the_status() {
    let fixture = fixture();
    fixture.ssh_config("a");
    created(&fixture, &tag_request(&fixture, "plain-17", "reviewed"));
    let plain = verified(&fixture, "plain-17", false);
    let result = created(
        &fixture,
        &gesture_request(&fixture, "touch-17", "device_touch"),
    );
    assert_eq!(result["gesture"], "device_touch");
    assert_eq!(
        tag_message(&fixture, "touch-17"),
        format!(
            "Approve change-assurance record {RECORD_ID}\n\nApproved-Record-Id: {RECORD_ID}\nApproved-Record-Digest: {DIGEST}\nSigned-off-meaning: reviewed\nSigning-Gesture: device_touch\n"
        )
    );
    let touched = verified(&fixture, "touch-17", false);
    assert_eq!(touched["status"], "approved", "{touched}");
    assert_eq!(touched["gesture"], json!({"declared": "device_touch"}));
    assert_eq!(
        touched["signature"]["gesture"],
        json!({"evidenced": "none"})
    );
    for member in ["status", "approver", "meaning", "signature_kind"] {
        assert_eq!(touched[member], plain[member], "{member}");
    }
    // No member scores, classes or accepts a gesture.
    assert_eq!(touched["gesture"].as_object().expect("object").len(), 1);
    assert_eq!(
        touched["signature"]["gesture"]
            .as_object()
            .expect("object")
            .len(),
        1
    );
}

#[test]
#[trace("TC-1151", "FR-085-AC-4")]
fn tc_1151_a_gesture_outside_the_three_values_is_refused_without_starting_git() {
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
    for bad in [
        json!("touch"),
        json!(""),
        json!("Device_Touch"),
        json!("DEVICE_TOUCH"),
        json!(5),
        json!(null),
        json!(["device_touch"]),
        json!(true),
    ] {
        let mut body = tag_request(&fixture, "approve-17", "reviewed");
        body["gesture"] = bad.clone();
        let output = run(&fixture, "approval-tag", body.to_string().as_bytes(), &shim);
        assert_eq!(output.status.code(), Some(2), "{bad}: {output:?}");
        let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
        assert_eq!(error["code"], "approval_tag_request_invalid", "{bad}");
    }
    assert!(!marker.exists(), "a refused request must not start git");
}

#[test]
#[trace("TC-1152", "FR-085-AC-10")]
fn tc_1152_the_signing_run_keeps_its_environment_and_passes_stderr_through() {
    let fixture = fixture();
    fixture.ssh_config("a");
    let record = fixture.root().join("git-run");
    fixture.executable(
        "git",
        &format!(
            "#!/bin/sh\ncase \"$*\" in *' tag -s '*|'tag -s '*)\n  {{ echo \"stdin=$(/bin/readlink /proc/$$/fd/0)\"; echo \"askpass=$SSH_ASKPASS\"; echo \"require=$SSH_ASKPASS_REQUIRE\"; echo \"display=$DISPLAY\"; echo \"sock=$SSH_AUTH_SOCK\"; }} > '{}'\n  echo STDERR-REACHED-THE-CALLER >&2 ;;\nesac\nexec {} \"$@\"\n",
            record.display(),
            which("git").display()
        ),
    );
    let askpass = fixture.root().join("askpass-program");
    let sock = fixture.root().join("agent-sock");
    let extra: [(&str, &Path); 4] = [
        ("SSH_ASKPASS", &askpass),
        ("SSH_ASKPASS_REQUIRE", Path::new("force")),
        ("DISPLAY", Path::new(":77")),
        ("SSH_AUTH_SOCK", &sock),
    ];
    let with = run_env(
        &fixture,
        "approval-tag",
        gesture_request(&fixture, "approve-17", "device_touch")
            .to_string()
            .as_bytes(),
        &fixture.bin(),
        &extra,
    );
    assert_eq!(with.status.code(), Some(0), "{with:?}");
    assert!(
        String::from_utf8_lossy(&with.stderr).contains("STDERR-REACHED-THE-CALLER"),
        "{with:?}"
    );
    let seen = fs::read_to_string(&record).expect("git run record");
    assert_eq!(
        seen,
        format!(
            "stdin=/dev/null\naskpass={}\nrequire=force\ndisplay=:77\nsock={}\n",
            askpass.display(),
            sock.display()
        )
    );
    // Without a gesture the same variables reach git and its stderr stays captured.
    fs::remove_file(&record).expect("reset");
    let without = run_env(
        &fixture,
        "approval-tag",
        tag_request(&fixture, "approve-18", "reviewed")
            .to_string()
            .as_bytes(),
        &fixture.bin(),
        &extra,
    );
    assert_eq!(without.status.code(), Some(0), "{without:?}");
    assert!(
        !String::from_utf8_lossy(&without.stderr).contains("STDERR-REACHED-THE-CALLER"),
        "{without:?}"
    );
    assert_eq!(fs::read_to_string(&record).expect("record"), seen);
}

#[test]
#[trace("TC-1153", "FR-086-AC-4", "FR-086-AC-5")]
fn tc_1153_an_unrecognized_or_repeated_declaration_is_unrecognized_and_other_statuses_have_none() {
    let fixture = fixture();
    fixture.ssh_config("a");
    let repo = fixture.repo();
    let public = fixture.public("a");
    let sign = |name: &str, message: &str| {
        git(
            &repo,
            &[
                "-c",
                "gpg.format=ssh",
                "-c",
                &format!("user.signingkey={}", public.display()),
                "tag",
                "-s",
                "--cleanup=verbatim",
                "-m",
                message,
                name,
            ],
        );
    };
    let base = format!(
        "Approve\n\nApproved-Record-Id: {RECORD_ID}\nApproved-Record-Digest: {DIGEST}\nSigned-off-meaning: reviewed\n"
    );
    sign("other", &format!("{base}Signing-Gesture: fingerprint\n"));
    sign(
        "repeated",
        &format!("{base}Signing-Gesture: device_touch\nSigning-Gesture: device_touch\n"),
    );
    sign("wrong-record", &base.replace(DIGEST, "sha256:ff"));
    for tag in ["other", "repeated"] {
        let result = verified(&fixture, tag, false);
        assert_eq!(result["status"], "approved", "{result}");
        assert_eq!(
            result["gesture"],
            json!({"declared": "unrecognized"}),
            "{tag}"
        );
    }
    let wrong = verified(&fixture, "wrong-record", false);
    assert_eq!(wrong["status"], "wrong_record", "{wrong}");
    assert!(wrong.get("gesture").is_none());
    git(&repo, &["tag", "-a", "-m", "plain", "unsigned"]);
    let unsigned = verified(&fixture, "unsigned", false);
    assert_eq!(unsigned["status"], "unsigned");
    assert!(unsigned.get("gesture").is_none());
    let missing = verified(&fixture, "no-such-tag", false);
    assert_eq!(missing["status"], "unverifiable");
    assert!(missing.get("gesture").is_none());
    // A key the trust input does not name: untrusted_signer carries none.
    fs::write(fixture.root().join("allowed_signers"), "").expect("empty trust");
    let untrusted = verified(&fixture, "other", false);
    assert_eq!(untrusted["status"], "untrusted_signer", "{untrusted}");
    assert!(untrusted.get("gesture").is_none());
}

#[test]
#[trace("TC-1154", "FR-086-AC-2")]
fn tc_1154_an_oidc_interactive_declaration_through_the_gitsign_stand_in_is_declared_only() {
    let fixture = fixture();
    fixture.x509_config();
    let committed = git(&fixture.repo(), &["rev-parse", "HEAD"]);
    assert!(!committed.is_empty());
    let registry = json!({
        "protocol": "engineering-assurance.gitsign-identity-registry/v1",
        "entries": [{
            "certificate_identity": IDENTITY,
            "certificate_oidc_issuer": ISSUER,
            "principal": PRINCIPAL,
        }],
    })
    .to_string();
    let repo = fixture.repo();
    fs::write(repo.join("identity-registry.json"), registry).expect("registry");
    git(&repo, &["add", "identity-registry.json"]);
    git(&repo, &["commit", "-q", "-m", "registry"]);
    let result = created(
        &fixture,
        &gesture_request(&fixture, "approve-17", "oidc_interactive"),
    );
    assert_eq!(result["gesture"], "oidc_interactive");
    let approved = verified(&fixture, "approve-17", true);
    assert_eq!(approved["status"], "approved", "{approved}");
    assert_eq!(approved["gesture"], json!({"declared": "oidc_interactive"}));
    assert_eq!(
        approved["signature"]["gesture"],
        json!({"evidenced": "none"})
    );
}

/// A real `ssh-agent` on a socket in the fixture, killed on drop.
struct Agent {
    child: std::process::Child,
    socket: PathBuf,
}

impl Drop for Agent {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Starts an `ssh-agent` whose confirmation program is `askpass`, and adds key
/// `a` of the fixture with `ssh-add -c`. The configured signing key is the
/// public key given literally, so git signs through the agent.
fn agent_with_confirmed_key(fixture: &Fixture, askpass: &Path) -> Agent {
    let socket = fixture.root().join("agent.sock");
    let child = Command::new("ssh-agent")
        .args(["-D", "-a"])
        .arg(&socket)
        .env("SSH_ASKPASS", askpass)
        .env("SSH_ASKPASS_REQUIRE", "force")
        .env("DISPLAY", ":0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("ssh-agent must start");
    let agent = Agent { child, socket };
    for _ in 0..200 {
        if agent.socket.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let added = Command::new("ssh-add")
        .arg("-c")
        .arg(fixture.root().join("a"))
        .env("SSH_AUTH_SOCK", &agent.socket)
        .env_remove("SSH_ASKPASS")
        .stdin(Stdio::null())
        .output()
        .expect("ssh-add must start");
    assert!(added.status.success(), "{added:?}");
    let public = fs::read_to_string(fixture.public("a")).expect("public key");
    let literal: Vec<&str> = public.split_whitespace().take(2).collect();
    fixture.config(&format!(
        "[gpg]\n\tformat = ssh\n[user]\n\tsigningkey = {}\n",
        literal.join(" ")
    ));
    agent
}

fn askpass_program(fixture: &Fixture) -> (PathBuf, PathBuf) {
    let log = fixture.root().join("askpass.log");
    let deny = fixture.root().join("askpass.deny");
    let path = fixture.root().join("askpass.sh");
    fs::write(
        &path,
        format!(
            "#!/bin/sh\necho call >> '{}'\n[ -e '{}' ] && exit 1\nexit 0\n",
            log.display(),
            deny.display()
        ),
    )
    .expect("askpass");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("mode");
    (path, log)
}

fn refs(fixture: &Fixture) -> String {
    git(&fixture.repo(), &["for-each-ref"])
}

#[test]
#[trace("TC-1155", "FR-085-AC-5", "FR-085-AC-6", "FR-086-AC-2")]
fn tc_1155_ssh_add_c_confirms_each_signature_and_a_denial_creates_no_tag() {
    let fixture = fixture();
    let (askpass, log) = askpass_program(&fixture);
    let agent = agent_with_confirmed_key(&fixture, &askpass);
    let socket = agent.socket.clone();
    let create_as = |tag: &str| {
        run_env(
            &fixture,
            "approval-tag",
            gesture_request(&fixture, tag, "agent_confirm")
                .to_string()
                .as_bytes(),
            &fixture.bin(),
            &[("SSH_AUTH_SOCK", &socket)],
        )
    };
    for tag in ["approve-1", "approve-2"] {
        let output = create_as(tag);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
    }
    let calls = fs::read_to_string(&log).expect("askpass log");
    assert_eq!(
        calls.lines().count(),
        2,
        "one confirmation per signature: {calls:?}"
    );
    let approved = verified(&fixture, "approve-1", false);
    assert_eq!(approved["status"], "approved", "{approved}");
    assert_eq!(approved["gesture"], json!({"declared": "agent_confirm"}));
    assert_eq!(
        approved["signature"]["gesture"],
        json!({"evidenced": "none"})
    );

    fs::write(fixture.root().join("askpass.deny"), "").expect("deny");
    let before = refs(&fixture);
    let denied = create_as("approve-3");
    assert_eq!(denied.status.code(), Some(2), "{denied:?}");
    let error: Value = serde_json::from_slice(&denied.stdout).expect("error json");
    assert_eq!(error["code"], "approval_tag_create_failed", "{error}");
    assert_eq!(refs(&fixture), before, "no ref may change");
    assert_eq!(
        fs::read_to_string(&log)
            .expect("askpass log")
            .lines()
            .count(),
        3
    );
}

/// A stand-in `gpg.x509.program` that talks to a person: it records its process
/// group and the terminal's foreground group, then (unless a `fail` file sits
/// beside it) writes a prompt to `/dev/tty`, reads one line from it, records the
/// line and emits a CMS-shaped block. Git feeds it the message on standard
/// input, which it drains with builtins first.
const PTY_SIGNER: &str = r#"#!/bin/sh
d=${0%/*}
while read -r line || [ -n "$line" ]; do :; done
read -r stat < /proc/$$/stat
set -- ${stat##*) }
echo "$3 $6" > "$d/groups"
if [ -e "$d/fail" ]; then exit 1; fi
printf 'enter the code: ' > /dev/tty
read -r answer < /dev/tty
echo "$answer" > "$d/answer"
echo "[GNUPG:] KEY_CONSIDERED AAAA 0" >&2
echo "[GNUPG:] SIG_CREATED D 1 8 00 1700000000 00 AAAA" >&2
printf -- '-----BEGIN SIGNED MESSAGE-----\nAAAA\n-----END SIGNED MESSAGE-----\n'
"#;

/// A non-interactive signer that records its inherited process-group boundary.
const BACKGROUND_PTY_SIGNER: &str = r#"#!/bin/sh
d=${0%/*}
while read -r line || [ -n "$line" ]; do :; done
read -r stat < /proc/$$/stat
set -- ${stat##*) }
echo "$3 $6" > "$d/groups"
echo "[GNUPG:] KEY_CONSIDERED AAAA 0" >&2
echo "[GNUPG:] SIG_CREATED D 1 8 00 1700000000 00 AAAA" >&2
printf -- '-----BEGIN SIGNED MESSAGE-----\nAAAA\n-----END SIGNED MESSAGE-----\n'
"#;

/// What one `approval-tag` run under a pseudo-terminal left behind.
struct PtyRun {
    /// The terminal's foreground process group before and after the run.
    before: i32,
    after: i32,
    /// Exit status of the CLI.
    code: i32,
    /// The CLI's standard output and standard error.
    stdout: String,
    stderr: String,
    /// The signer's process group and the foreground group it saw.
    signer_group: Option<(i32, i32)>,
    /// The line the signer read from the terminal.
    answer: Option<String>,
}

/// Runs `approval-tag --request -` as the session leader of a pseudo-terminal
/// made by `script`, with the request on standard input and standard output and
/// standard error in files, so that only `/dev/tty` is the terminal. `typed` is
/// what the person types.
fn approval_tag_under_pty(fixture: &Fixture, request: &Value, typed: &str) -> PtyRun {
    let root = fixture.root();
    let (request_file, out, stdout, stderr) = (
        root.join("pty-request.json"),
        root.join("pty-out"),
        root.join("pty-stdout"),
        root.join("pty-stderr"),
    );
    fs::write(&request_file, request.to_string()).expect("request file");
    let wrapper = root.join("pty-wrapper.sh");
    fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\ntp() {{ read -r s < /proc/$$/stat; set -- ${{s##*) }}; echo \"$6\"; }}\necho \"before=$(tp)\" > '{out}'\n'{cli}' approval-tag --request - < '{request}' > '{stdout}' 2> '{stderr}'\necho \"rc=$?\" >> '{out}'\necho \"after=$(tp)\" >> '{out}'\n",
            out = out.display(),
            cli = env!("CARGO_BIN_EXE_engineering-assurance"),
            request = request_file.display(),
            stdout = stdout.display(),
            stderr = stderr.display(),
        ),
    )
    .expect("wrapper");
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).expect("mode");
    let mut child = Command::new("/usr/bin/script")
        .args(["-q", "-e", "-c"])
        .arg(&wrapper)
        .arg("/dev/null")
        .current_dir(fixture.repo())
        .env_clear()
        .env("PATH", fixture.bin())
        .env("SHELL", "/bin/sh")
        .env("HOME", root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", fixture.global())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("script must start");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(typed.as_bytes())
        .expect("typed input");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    loop {
        if child.try_wait().expect("wait").is_some() {
            break;
        }
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("the run under the pseudo-terminal did not finish: a stopped signer?");
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let record = fs::read_to_string(&out).expect("wrapper record");
    let field = |name: &str| -> i32 {
        record
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .unwrap_or_else(|| panic!("{name} missing: {record:?}"))
            .parse()
            .expect("number")
    };
    let signer_group = fs::read_to_string(fixture.bin().join("groups"))
        .ok()
        .map(|text| {
            let mut parts = text
                .split_whitespace()
                .map(|p| p.parse::<i32>().expect("number"));
            (parts.next().expect("pgrp"), parts.next().expect("tpgid"))
        });
    PtyRun {
        before: field("before="),
        after: field("after="),
        code: field("rc="),
        stdout: fs::read_to_string(&stdout).expect("stdout"),
        stderr: fs::read_to_string(&stderr).expect("stderr"),
        signer_group,
        answer: fs::read_to_string(fixture.bin().join("answer"))
            .ok()
            .map(|text| text.trim_end().to_owned()),
    }
}

fn process_group_exists(group: i32) -> bool {
    let pid = rustix::process::Pid::from_raw(group).expect("group id");
    for _ in 0..100 {
        if rustix::process::test_kill_process_group(pid).is_err() {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    true
}

#[test]
#[trace("TC-1156", "FR-085-AC-2", "FR-085-AC-11")]
fn tc_1156_a_signer_reads_the_terminal_in_the_foreground_and_the_group_is_restored() {
    let fixture = fixture();
    fixture.x509_config();
    fixture.executable("sign-standin", PTY_SIGNER);
    let request = gesture_request(&fixture, "approve-17", "oidc_interactive");
    let run = approval_tag_under_pty(&fixture, &request, "typed-by-the-person\n");
    assert_eq!(run.code, 0, "{} / {}", run.stdout, run.stderr);
    // The line came from the terminal, not from the request on standard input.
    assert_eq!(run.answer.as_deref(), Some("typed-by-the-person"));
    // The signer was not stopped by SIGTTIN: its group was the foreground group.
    let (signer, foreground) = run.signer_group.expect("signer groups");
    assert_eq!(
        signer, foreground,
        "git's group is the terminal's foreground"
    );
    assert_ne!(signer, run.before);
    // Standard output is exactly one created line holding the declaration.
    assert!(run.stdout.ends_with('\n') && run.stdout.matches('\n').count() == 1);
    let line: Value = serde_json::from_str(&run.stdout).expect("json line");
    assert_eq!(line["status"], "created");
    assert_eq!(line["gesture"], "oidc_interactive");
    // The previous foreground group is back and git's group is gone.
    assert_eq!(run.after, run.before);
    assert!(
        !process_group_exists(signer),
        "git's group must not outlive the run"
    );
    assert!(tags(&fixture).contains("approve-17"));
}

#[test]
#[trace("TC-1157", "FR-085-AC-11")]
fn tc_1157_the_foreground_group_is_restored_after_a_signer_fails() {
    let fixture = fixture();
    fixture.x509_config();
    fixture.executable("sign-standin", PTY_SIGNER);
    fs::write(fixture.bin().join("fail"), "").expect("fail flag");
    let request = gesture_request(&fixture, "approve-17", "oidc_interactive");
    let run = approval_tag_under_pty(&fixture, &request, "unused\n");
    assert_eq!(run.code, 2, "{} / {}", run.stdout, run.stderr);
    let error: Value = serde_json::from_str(&run.stdout).expect("error json");
    assert_eq!(error["code"], "approval_tag_create_failed", "{error}");
    let (signer, foreground) = run.signer_group.expect("signer groups");
    assert_eq!(signer, foreground);
    assert_eq!(run.after, run.before);
    assert!(!process_group_exists(signer));
    assert_eq!(tags(&fixture), "");
}

#[test]
#[trace("TC-1160", "FR-085-AC-1")]
fn tc_1160_without_a_gesture_git_runs_outside_the_terminal_foreground_group() {
    let fixture = fixture();
    fixture.x509_config();
    fixture.executable("sign-standin", BACKGROUND_PTY_SIGNER);
    let request = tag_request(&fixture, "approve-17", "reviewed");
    let run = approval_tag_under_pty(&fixture, &request, "unused\n");

    assert_eq!(run.code, 0, "{} / {}", run.stdout, run.stderr);
    let (signer, foreground) = run.signer_group.expect("signer groups");
    assert_ne!(signer, foreground, "git's group is not foreground");
    assert_eq!(foreground, run.before, "the caller stays foreground");
    assert_eq!(run.after, run.before, "the foreground group is unchanged");
    let line: Value = serde_json::from_str(&run.stdout).expect("json line");
    assert_eq!(line["status"], "created");
    assert!(line.get("gesture").is_none(), "{line}");
    assert!(tags(&fixture).contains("approve-17"));
}
