// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary tests for `approval-trail` on gitsign-signed tags (FR-043).
//! `gitsign` is not installed on the build machines, so the tests run a
//! stand-in executable named `gitsign` that exits with a scripted status (the
//! FR-040 stand-in). The CMS signature block is forged and the identities are
//! fictional.

use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
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

fn forged_tag(fixture: &Fixture, name: &str, epoch: u64, meaning: &str) {
    let repo = fixture.repo();
    let commit = git(&repo, &["rev-parse", "HEAD"]);
    let message = format!(
        "Approve the change\n\nApproved-Record-Id: {RECORD_ID}\nApproved-Record-Digest: {DIGEST}\nSigned-off-meaning: {meaning}"
    );
    let object = format!(
        "object {}\ntype commit\ntag {name}\ntagger t <t@example.test> {epoch} +0000\n\n{message}\n{CMS_BLOCK}",
        commit.trim()
    );
    let id = run_program(
        &repo,
        "git",
        &["hash-object", "-t", "tag", "-w", "--stdin"],
        Some(object.as_bytes()),
    );
    let id = String::from_utf8(id).expect("utf-8");
    git(
        &repo,
        &["update-ref", &format!("refs/tags/{name}"), id.trim()],
    );
}

fn trail(fixture: &Fixture) -> Value {
    let request = json!({
        "protocol": "engineering-assurance.approval-trail-request/v1",
        "repository": fixture.repo().display().to_string(),
        "record_id": RECORD_ID,
        "record_digest": DIGEST,
        "allowed_signers": fixture.root().join("allowed_signers").display().to_string(),
        "gitsign": {
            "certificate_identity": IDENTITY,
            "certificate_oidc_issuer": ISSUER,
            "principal": PRINCIPAL,
        },
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["approval-trail", "--request", "-"])
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
    let output = child.wait_with_output().expect("output");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    serde_json::from_slice(&output.stdout).expect("json line")
}

#[test]
#[trace("TC-362", "FR-043-AC-11")]
fn tc_362_gitsign_tags_are_listed_with_the_stand_in_verdict() {
    let fixture = fixture();
    forged_tag(&fixture, "v-1", 1_700_000_000, "approved");
    forged_tag(&fixture, "v-2", 1_700_000_100, "revoked");
    let approved = trail(&fixture);
    let entries = approved["entries"].as_array().expect("entries");
    assert_eq!(entries.len(), 2, "{approved}");
    for (listed, tag, meaning) in [
        (&entries[0], "v-1", "approved"),
        (&entries[1], "v-2", "revoked"),
    ] {
        assert_eq!(listed["tag"], tag);
        assert_eq!(listed["status"], "approved", "{listed}");
        assert_eq!(listed["signature_kind"], "gitsign");
        assert_eq!(listed["approver"], PRINCIPAL);
        assert_eq!(listed["meaning"], json!([meaning]));
    }
    fs::write(fixture.bin().join("exit_exact_exact"), "1\n").expect("script");
    let rejected = trail(&fixture);
    for listed in rejected["entries"].as_array().expect("entries") {
        assert_eq!(listed["status"], "untrusted_signer", "{listed}");
        assert!(listed.get("approver").is_none());
    }
}
