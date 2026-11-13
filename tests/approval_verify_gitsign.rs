// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary tests for `approval-verify` on gitsign-signed tags (FR-041).
//! `gitsign` is not installed on the build machines, so the tests run a
//! stand-in executable named `gitsign` that exits with a scripted status (the
//! FR-040 stand-in). The CMS signature block is forged with `git hash-object`
//! and the identities are fictional.

use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const IDENTITY: &str = "alice@example.test";
const ISSUER: &str = concat!("https:", "//issuer.example.test");
const PRINCIPAL: &str = "Alice Example";
const CMS_BLOCK: &str = "-----BEGIN SIGNED MESSAGE-----\nAAAA\n-----END SIGNED MESSAGE-----\n";
const RECORD_ID: &str = "change-record-17";
const DIGEST: &str = "sha256:0123456789abcdef";

/// Scripted by flavor: `exit_<identity>_<issuer>` holds the exit status.
const STAND_IN: &str = r#"#!/bin/sh
d=${0%/*}
id=exact
iss=exact
for a in "$@"; do
  case "$a" in
    --certificate-identity-regexp) id=any ;;
    --certificate-oidc-issuer-regexp) iss=any ;;
  esac
done
echo run >> "$d/invocations"
status=0
if [ -e "$d/exit_${id}_${iss}" ]; then read -r status < "$d/exit_${id}_${iss}"; fi
exit "$status"
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

    fn allowed(&self) -> PathBuf {
        self.root().join("allowed_signers")
    }

    fn script(&self, flavor: &str, status: i32) {
        fs::write(
            self.bin().join(format!("exit_{flavor}")),
            format!("{status}\n"),
        )
        .expect("script");
    }
}

fn run_program(
    directory: &Path,
    program: &str,
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
    let mut all = vec!["-c", "user.name=t", "-c", "user.email=t@example.test"];
    all.extend_from_slice(arguments);
    String::from_utf8(run_program(repo, "git", &all, None)).expect("utf-8")
}

fn which(program: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
        .expect("program on PATH")
}

/// A repository with one commit, an empty allowed-signers file and a `bin`
/// directory holding `git` and the stand-in `gitsign`.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir(root.join("home")).expect("home");
    fs::write(root.join("allowed_signers"), "").expect("allowed signers");
    let bin = root.join("bin");
    fs::create_dir(&bin).expect("bin");
    let wrapper = format!("#!/bin/sh\nexec {} \"$@\"\n", which("git").display());
    fs::write(bin.join("git"), wrapper).expect("git wrapper");
    fs::set_permissions(bin.join("git"), fs::Permissions::from_mode(0o755)).expect("mode");
    fs::write(bin.join("gitsign"), STAND_IN).expect("stand-in");
    fs::set_permissions(bin.join("gitsign"), fs::Permissions::from_mode(0o755)).expect("mode");
    let repo = root.join("repo");
    fs::create_dir(&repo).expect("repo dir");
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "base"]);
    Fixture { dir }
}

fn store(repo: &Path, kind: &str, object: &str) -> String {
    let id = run_program(
        repo,
        "git",
        &["hash-object", "-t", kind, "-w", "--stdin"],
        Some(object.as_bytes()),
    );
    String::from_utf8(id).expect("utf-8").trim().to_owned()
}

/// An annotated tag named `name` whose message ends in `block`.
fn forged_tag(fixture: &Fixture, name: &str, message: &str, block: &str) -> String {
    forged_tag_with_tagger(
        fixture,
        name,
        Some("t <t@example.test> 1700000000 +0000"),
        message,
        block,
    )
}

/// As [`forged_tag`], with the given tagger line (none when `None`).
fn forged_tag_with_tagger(
    fixture: &Fixture,
    name: &str,
    tagger: Option<&str>,
    message: &str,
    block: &str,
) -> String {
    let repo = fixture.repo();
    let commit = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    let tagger = tagger.map_or_else(String::new, |line| format!("tagger {line}\n"));
    let object = format!("object {commit}\ntype commit\ntag {name}\n{tagger}\n{message}\n{block}");
    let id = store(&repo, "tag", &object);
    git(&repo, &["update-ref", &format!("refs/tags/{name}"), &id]);
    id
}

fn good_message() -> String {
    format!(
        "Approve the change\n\nApproved-Record-Id: {RECORD_ID}\nApproved-Record-Digest: {DIGEST}\nSigned-off-meaning: reviewed"
    )
}

fn request(fixture: &Fixture, tag: &str, gitsign: bool) -> Value {
    let mut value = json!({
        "protocol": "engineering-assurance.approval-verify-request/v1",
        "repository": fixture.repo().display().to_string(),
        "tag": tag,
        "record_id": RECORD_ID,
        "record_digest": DIGEST,
        "allowed_signers": fixture.allowed().display().to_string(),
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

fn result(fixture: &Fixture, request: &Value) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["approval-verify", "--request", "-"])
        .env_clear()
        .env("PATH", fixture.bin())
        .env("HOME", fixture.root().join("home"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the Cargo-built CLI must start");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(request.to_string().as_bytes())
        .expect("request");
    let output: Output = child.wait_with_output().expect("output");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    serde_json::from_slice(&output.stdout).expect("json line")
}

#[test]
#[trace("TC-328", "FR-041-AC-8")]
fn tc_328_a_gitsign_tag_is_approved_by_the_request_principal() {
    let fixture = fixture();
    forged_tag(&fixture, "v-cms", &good_message(), CMS_BLOCK);
    let approved = result(&fixture, &request(&fixture, "v-cms", true));
    assert_eq!(approved["status"], "approved", "{approved}");
    assert_eq!(approved["signature_kind"], "gitsign");
    assert_eq!(approved["approver"], PRINCIPAL);
    assert_eq!(approved["meaning"], json!(["reviewed"]));
    assert_eq!(
        approved["trust"]["gitsign"],
        json!({
            "certificate_identity": IDENTITY,
            "certificate_oidc_issuer": ISSUER,
            "principal": PRINCIPAL,
        })
    );
}

#[test]
#[trace("TC-329", "FR-041-AC-8")]
fn tc_329_an_identity_gitsign_does_not_accept_is_untrusted_and_a_missing_one_is_unverifiable() {
    let fixture = fixture();
    forged_tag(&fixture, "v-cms", &good_message(), CMS_BLOCK);
    fixture.script("exact_exact", 1);
    let untrusted = result(&fixture, &request(&fixture, "v-cms", true));
    assert_eq!(untrusted["status"], "untrusted_signer", "{untrusted}");
    assert_eq!(untrusted["signature"]["status"], "signed_untrusted");
    assert!(untrusted.get("approver").is_none());

    let missing = result(&fixture, &request(&fixture, "v-cms", false));
    assert_eq!(missing["status"], "unverifiable", "{missing}");
    assert_eq!(missing["reason"], "gitsign_identity_missing");
    assert!(missing["trust"].get("gitsign").is_none());
}

#[test]
#[trace("TC-336", "FR-041-AC-20")]
fn tc_336_a_gitsign_tag_reports_its_tagger_date_and_gitsign_never_changes_the_status() {
    let fixture = fixture();
    forged_tag_with_tagger(
        &fixture,
        "v-cms",
        Some("t <t@example.test> 1700000000 -0700"),
        &good_message(),
        CMS_BLOCK,
    );
    let approved = result(&fixture, &request(&fixture, "v-cms", true));
    assert_eq!(approved["status"], "approved", "{approved}");
    assert_eq!(approved["approver"], PRINCIPAL);
    assert_eq!(approved["signed_at"], "2023-11-14T22:13:20Z");
    assert_eq!(approved["signed_at_source"], "tagger_date");
    // Reading the time starts no gitsign run: one approval ran it once.
    let runs = fs::read_to_string(fixture.bin().join("invocations")).expect("log");
    assert_eq!(runs.lines().count(), 1, "{runs:?}");
    // A rejected identity is untrusted and carries no time.
    fixture.script("exact_exact", 1);
    let untrusted = result(&fixture, &request(&fixture, "v-cms", true));
    assert_eq!(untrusted["status"], "untrusted_signer", "{untrusted}");
    assert!(untrusted.get("signed_at").is_none());
}

#[test]
#[trace("TC-337", "FR-041-AC-18", "FR-041-CON-3")]
fn tc_337_an_approval_with_no_readable_tagger_date_reports_the_time_missing() {
    let fixture = fixture();
    forged_tag_with_tagger(&fixture, "v-none", None, &good_message(), CMS_BLOCK);
    forged_tag_with_tagger(
        &fixture,
        "v-bad",
        Some("t <t@example.test> yesterday +0000"),
        &good_message(),
        CMS_BLOCK,
    );
    for tag in ["v-none", "v-bad"] {
        let approved = result(&fixture, &request(&fixture, tag, true));
        assert_eq!(approved["status"], "approved", "{approved}");
        assert_eq!(approved["approver"], PRINCIPAL);
        assert_eq!(approved["meaning"], json!(["reviewed"]));
        assert_eq!(approved["signed_at_missing"], "tagger_date_unreadable");
        assert!(approved.get("signed_at").is_none());
        assert!(approved.get("signed_at_source").is_none());
    }
}
