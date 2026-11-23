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
const REGISTRY: &str = "registry:registry.json";

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
    commit_registry(&repo, &[(IDENTITY, ISSUER, PRINCIPAL)]);
    Fixture { dir }
}

/// Commits a registry holding `entries` as `registry.json` in a root commit that
/// branch `registry` names. Returns the commit id.
fn commit_registry(repo: &Path, entries: &[(&str, &str, &str)]) -> String {
    let entries: Vec<Value> = entries
        .iter()
        .map(|(identity, issuer, principal)| {
            json!({
                "certificate_identity": identity,
                "certificate_oidc_issuer": issuer,
                "principal": principal,
            })
        })
        .collect();
    let document = json!({
        "protocol": "engineering-assurance.gitsign-identity-registry/v1",
        "entries": entries,
    })
    .to_string();
    let blob = run_program(
        repo,
        "git",
        &["hash-object", "-t", "blob", "-w", "--stdin"],
        Some(document.as_bytes()),
    );
    let blob = String::from_utf8(blob).expect("utf-8").trim().to_owned();
    let tree = run_program(
        repo,
        "git",
        &["mktree"],
        Some(format!("100644 blob {blob}\tregistry.json\n").as_bytes()),
    );
    let tree = String::from_utf8(tree).expect("utf-8").trim().to_owned();
    let commit = git(repo, &["commit-tree", &tree, "-m", "registry"])
        .trim()
        .to_owned();
    git(repo, &["update-ref", "refs/heads/registry", &commit]);
    commit
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

fn request(fixture: &Fixture) -> Value {
    json!({
        "protocol": "engineering-assurance.approval-trail-request/v1",
        "repository": fixture.repo().display().to_string(),
        "record_id": RECORD_ID,
        "record_digest": DIGEST,
        "allowed_signers": fixture.root().join("allowed_signers").display().to_string(),
        "gitsign": {
            "certificate_identity": IDENTITY,
            "certificate_oidc_issuer": ISSUER,
        },
        "identity_registry": REGISTRY,
    })
}

fn trail(fixture: &Fixture) -> Value {
    let output = run_trail(fixture, &request(fixture), &fixture.bin());
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    serde_json::from_slice(&output.stdout).expect("json line")
}

fn run_trail(fixture: &Fixture, request: &Value, path: &Path) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["approval-trail", "--request", "-"])
        .env_clear()
        .env("PATH", path)
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
    child.wait_with_output().expect("output")
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

#[test]
#[trace("TC-362", "FR-043-AC-11", "FR-043-AC-4")]
fn tc_362_an_identity_the_registry_does_not_name_is_listed_untrusted() {
    let fixture = fixture();
    forged_tag(&fixture, "v-1", 1_700_000_000, "approved");
    let registry_commit = commit_registry(&fixture.repo(), &[("bob@example.test", ISSUER, "Bob")]);
    let listed = trail(&fixture);
    let entries = listed["entries"].as_array().expect("entries");
    assert_eq!(entries.len(), 1, "{listed}");
    assert_eq!(entries[0]["status"], "untrusted_signer", "{listed}");
    assert_eq!(entries[0]["signature"]["status"], "signed_valid");
    assert!(entries[0].get("approver").is_none());
    // The trust carries the registry and the commit it resolved to.
    assert_eq!(listed["trust"]["identity_registry"], REGISTRY);
    assert_eq!(listed["trust"]["identity_registry_commit"], registry_commit);
    assert_eq!(
        entries[0]["signature"]["identity_registry_commit"],
        registry_commit
    );
}

#[test]
#[trace("TC-361", "FR-043-AC-10")]
fn tc_361_the_gitsign_inputs_are_refused_with_the_trail_codes() {
    let fixture = fixture();
    forged_tag(&fixture, "v-1", 1_700_000_000, "approved");
    let no_git = fixture.root().join("no-git");
    fs::create_dir(&no_git).expect("empty PATH directory");
    let code = |request: &Value, path: &Path| {
        let output = run_trail(&fixture, request, path);
        assert_eq!(output.status.code(), Some(2), "{request}: {output:?}");
        let error: Value = serde_json::from_slice(&output.stdout).expect("machine error");
        error["code"].clone()
    };

    let mut principal = request(&fixture);
    principal["gitsign"]["principal"] = json!("Mallory");
    assert_eq!(
        code(&principal, &no_git),
        "approval_trail_gitsign_principal_refused"
    );
    let mut without_registry = request(&fixture);
    without_registry
        .as_object_mut()
        .expect("object")
        .remove("identity_registry");
    assert_eq!(
        code(&without_registry, &no_git),
        "approval_trail_request_invalid"
    );

    let mut unresolved = request(&fixture);
    unresolved["identity_registry"] = json!("no-such-branch:registry.json");
    assert_eq!(
        code(&unresolved, &fixture.bin()),
        "approval_trail_identity_registry_invalid"
    );

    // Not a repository: the repository code, with or without `gitsign`.
    let plain = fixture.root().join("plain");
    fs::create_dir(&plain).expect("plain directory");
    let mut with_gitsign = request(&fixture);
    with_gitsign["repository"] = json!(plain.display().to_string());
    assert_eq!(
        code(&with_gitsign, &fixture.bin()),
        "approval_trail_repository_unavailable"
    );
    // A git that cannot start while the registry is read is the same refusal.
    assert_eq!(
        code(&request(&fixture), &no_git),
        "approval_trail_repository_unavailable"
    );
}
