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
const OTHER_ISSUER: &str = concat!("https:", "//other.example.test");
const PRINCIPAL: &str = "Alice Example";
const CMS_BLOCK: &str = "-----BEGIN SIGNED MESSAGE-----\nAAAA\n-----END SIGNED MESSAGE-----\n";
const RECORD_ID: &str = "change-record-17";
const DIGEST: &str = "sha256:0123456789abcdef";
/// The committed identity registry every gitsign request names.
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
    commit_registry(
        &repo,
        "registry",
        &registry_json(&[(IDENTITY, ISSUER, PRINCIPAL)]),
    );
    Fixture { dir }
}

/// The registry document with the given `(identity, issuer, principal)` entries.
fn registry_json(entries: &[(&str, &str, &str)]) -> String {
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
    json!({
        "protocol": "engineering-assurance.gitsign-identity-registry/v1",
        "entries": entries,
    })
    .to_string()
}

/// Commits `content` as `registry.json` in a root commit that `branch` names,
/// without touching HEAD, the index or the working tree. Returns the commit id.
fn commit_registry(repo: &Path, branch: &str, content: &str) -> String {
    let blob = store(repo, "blob", content);
    let tree = String::from_utf8(run_program(
        repo,
        "git",
        &["mktree"],
        Some(format!("100644 blob {blob}\tregistry.json\n").as_bytes()),
    ))
    .expect("utf-8")
    .trim()
    .to_owned();
    let commit = git(repo, &["commit-tree", &tree, "-m", "registry"])
        .trim()
        .to_owned();
    git(
        repo,
        &["update-ref", &format!("refs/heads/{branch}"), &commit],
    );
    commit
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
        });
        value["identity_registry"] = json!(REGISTRY);
    }
    value
}

fn result(fixture: &Fixture, request: &Value) -> Value {
    result_on(fixture, request, &fixture.bin())
}

fn result_on(fixture: &Fixture, request: &Value, path: &Path) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["approval-verify", "--request", "-"])
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
    let output: Output = child.wait_with_output().expect("output");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    serde_json::from_slice(&output.stdout).expect("json line")
}

#[test]
#[ignore = "needs real gitsign v0.18.0, available Sigstore trust services and a public approval tag named by FR_041_LIVE_*"]
#[trace("TC-1017", "TC-1018", "TC-1019", "TC-1020", "FR-041-AC-26")]
fn tc_1017_real_gitsign_verifies_and_binds_a_public_approval_tag() {
    let (Ok(repository), Ok(tag), Ok(identity), Ok(issuer), Ok(record_id), Ok(record_digest)) = (
        std::env::var("FR_041_LIVE_REPOSITORY"),
        std::env::var("FR_041_LIVE_TAG"),
        std::env::var("FR_041_LIVE_IDENTITY"),
        std::env::var("FR_041_LIVE_ISSUER"),
        std::env::var("FR_041_LIVE_RECORD_ID"),
        std::env::var("FR_041_LIVE_RECORD_DIGEST"),
    ) else {
        panic!(
            "UNAVAILABLE: set FR_041_LIVE_REPOSITORY, _TAG, _IDENTITY, _ISSUER, _RECORD_ID and _RECORD_DIGEST; this is not live interoperability evidence"
        );
    };
    let version = Command::new("gitsign")
        .arg("--version")
        .output()
        .unwrap_or_else(|error| {
            panic!("LIVE_CASE=UNAVAILABLE gitsign_not_started error={error}; no approval interoperability pass recorded")
        });
    let version_text = String::from_utf8_lossy(&version.stdout).into_owned();
    assert!(version.status.success(), "gitsign --version: {version:?}");
    assert_gitsign_018_version(&version_text);

    let live = tempfile::tempdir().expect("live checkout");
    let live_repository = live.path().join("repository");
    let clone = Command::new("git")
        .args(["clone", "--no-hardlinks", "--no-checkout", &repository])
        .arg(&live_repository)
        .output()
        .expect("clone public live repository");
    assert!(clone.status.success(), "git clone: {clone:?}");
    let tag_ref = format!("refs/tags/{tag}");
    let original_id = resolve_tag_object(&live_repository, &tag_ref);
    eprintln!(
        "LIVE_CASE=STARTED gitsign_version={:?} tag_object={original_id} identity={identity:?} issuer={issuer:?} trust_service=probe_pending",
        version_text.trim()
    );
    let live = LiveApproval {
        repository: live_repository,
        tag: tag.clone(),
        tag_ref,
        record_id,
        record_digest,
        identity,
        issuer,
        path: std::env::var_os("PATH").expect("PATH"),
    };
    let fixture = fixture();
    assert_live_approval(&fixture, &live, &original_id);
    eprintln!(
        "LIVE_CASE=APPROVAL_PASS gitsign_version={:?} tag_object={original_id} identity={:?} issuer={:?} trust_service=available",
        live.identity,
        live.issuer,
        version_text.trim()
    );
    assert_live_identity_mismatches(&fixture, &live);
    assert_live_trust_restriction(&fixture, &live);
    assert_live_changed_tag(&fixture, &live, &original_id);
}

struct LiveApproval {
    repository: PathBuf,
    tag: String,
    tag_ref: String,
    record_id: String,
    record_digest: String,
    identity: String,
    issuer: String,
    path: std::ffi::OsString,
}

impl LiveApproval {
    fn request(&self, fixture: &Fixture, identity: &str, issuer: &str) -> Value {
        let mut body = request(fixture, &self.tag, true);
        body["repository"] = json!(self.repository.display().to_string());
        body["record_id"] = json!(self.record_id);
        body["record_digest"] = json!(self.record_digest);
        body["gitsign"]["certificate_identity"] = json!(identity);
        body["gitsign"]["certificate_oidc_issuer"] = json!(issuer);
        commit_registry(
            &self.repository,
            "registry",
            &registry_json(&[(&self.identity, &self.issuer, PRINCIPAL)]),
        );
        body
    }

    fn result(&self, fixture: &Fixture, identity: &str, issuer: &str) -> Value {
        result_on(
            fixture,
            &self.request(fixture, identity, issuer),
            Path::new(&self.path),
        )
    }
}

fn assert_gitsign_018_version(version: &str) {
    let token = version
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .last()
        .unwrap_or_default();
    assert!(
        matches!(token, "v0.18.0" | "0.18.0"),
        "expected exact gitsign v0.18.0 version token, got {version:?}"
    );
}

fn resolve_tag_object(repository: &Path, tag_ref: &str) -> String {
    git(repository, &["rev-parse", &format!("{tag_ref}^{{tag}}")])
        .trim()
        .to_owned()
}

fn assert_live_approval(fixture: &Fixture, live: &LiveApproval, original_id: &str) {
    let accepted = live.result(fixture, &live.identity, &live.issuer);
    assert_ne!(
        accepted["status"], "unverifiable",
        "LIVE_CASE=UNAVAILABLE reason={} trust_service=unavailable_or_unverifiable; no approval interoperability pass recorded",
        accepted["reason"]
    );
    assert_eq!(accepted["status"], "approved", "{accepted}");
    assert_eq!(accepted["tag_object_id"], original_id, "{accepted}");
    assert_eq!(
        accepted["signature"]["status"], "signed_valid",
        "{accepted}"
    );
    assert_eq!(
        accepted["signature"]["signature_kind"], "gitsign",
        "{accepted}"
    );
}

fn assert_live_identity_mismatches(fixture: &Fixture, live: &LiveApproval) {
    let wrong_identity = live.result(fixture, "wrong-identity@example.test", &live.issuer);
    assert_live_untrusted(&wrong_identity, "identity");
    let wrong_issuer = live.result(fixture, &live.identity, OTHER_ISSUER);
    assert_live_untrusted(&wrong_issuer, "issuer");
}

fn assert_live_untrusted(result: &Value, mismatch: &str) {
    assert_eq!(result["status"], "untrusted_signer", "{result}");
    assert_eq!(result["signature"]["mismatch"], mismatch, "{result}");
    assert!(result.get("approver").is_none(), "{result}");
    assert!(result.get("meaning").is_none(), "{result}");
    assert!(result.get("signed_at").is_none(), "{result}");
}

fn assert_live_trust_restriction(fixture: &Fixture, live: &LiveApproval) {
    let set = git(
        &live.repository,
        &["config", "gitsign.fulcioRoot", "/dev/null"],
    );
    assert!(set.is_empty(), "set config: {set}");
    let restricted = live.result(fixture, &live.identity, &live.issuer);
    assert_eq!(restricted["status"], "unverifiable", "{restricted}");
    assert_eq!(
        restricted["reason"], "gitsign_repository_config",
        "{restricted}"
    );
    assert!(restricted.get("approver").is_none(), "{restricted}");
    assert!(restricted.get("meaning").is_none(), "{restricted}");
    let remove = git(
        &live.repository,
        &["config", "--unset-all", "gitsign.fulcioRoot"],
    );
    assert!(remove.is_empty(), "remove config: {remove}");
    let check = Command::new("git")
        .args([
            "-C",
            live.repository.to_str().expect("repository path"),
            "config",
            "--get",
            "gitsign.fulcioRoot",
        ])
        .output()
        .expect("check cleared gitsign config");
    assert_eq!(
        check.status.code(),
        Some(1),
        "gitsign config remains: {check:?}"
    );
}

fn assert_live_changed_tag(fixture: &Fixture, live: &LiveApproval, original_id: &str) {
    let raw = git(&live.repository, &["cat-file", "tag", original_id]);
    let (headers, signed_message) = raw.split_once("\n\n").expect("annotated tag headers");
    let signature_at = signed_message
        .find("-----BEGIN SIGNED MESSAGE-----")
        .expect("gitsign CMS block in live tag");
    let (_, signature) = signed_message.split_at(signature_at);
    let changed = format!("{headers}\n\nchanged-by-ea-129\n{signature}");
    let changed_id = git_store_tag(&live.repository, &changed);
    eprintln!(
        "LIVE_CASE=TAG_MUTATION original_tag_object={original_id} changed_tag_object={changed_id}"
    );
    let update = git(
        &live.repository,
        &["update-ref", &live.tag_ref, &changed_id],
    );
    assert!(update.is_empty(), "update-ref: {update}");
    let changed = live.result(fixture, &live.identity, &live.issuer);
    assert_eq!(changed["status"], "unverifiable", "{changed}");
    assert_eq!(
        changed["reason"], "gitsign_verification_failed",
        "{changed}"
    );
    assert_eq!(changed["tag_object_id"], changed_id, "{changed}");
    assert!(changed.get("approver").is_none(), "{changed}");
    assert!(changed.get("meaning").is_none(), "{changed}");
    assert!(changed.get("signed_at").is_none(), "{changed}");
}

fn git_store_tag(repository: &Path, object: &str) -> String {
    let mut child = Command::new("git")
        .args([
            "-C",
            repository.to_str().expect("repository path"),
            "hash-object",
            "-t",
            "tag",
            "-w",
            "--stdin",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("git hash-object");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(object.as_bytes())
        .expect("tag object");
    let output = child.wait_with_output().expect("hash-object output");
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout)
        .expect("object id")
        .trim()
        .to_owned()
}

#[test]
#[trace("TC-1020", "FR-041-AC-26")]
fn tc_1020_missing_gitsign_is_unverifiable_not_an_approval() {
    let fixture = fixture();
    forged_tag(&fixture, "v-cms", &good_message(), CMS_BLOCK);
    fs::remove_file(fixture.bin().join("gitsign")).expect("remove stand-in gitsign");
    let missing = result(&fixture, &request(&fixture, "v-cms", true));
    assert_eq!(missing["status"], "unverifiable", "{missing}");
    assert_eq!(missing["reason"], "gitsign_not_started", "{missing}");
    assert!(missing.get("approver").is_none(), "{missing}");
    assert!(missing.get("meaning").is_none(), "{missing}");
}

#[test]
#[trace("TC-328", "FR-041-AC-8")]
fn tc_328_a_gitsign_tag_is_approved_by_the_registry_principal() {
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
        })
    );
    assert_eq!(approved["trust"]["identity_registry"], REGISTRY);
    assert_eq!(
        approved["signature"]["signer"]["source"],
        "gitsign_registry"
    );
}

#[test]
#[trace("TC-325", "FR-041-AC-14")]
fn tc_325_the_trust_echoes_the_registry_and_the_commit_it_resolved_to() {
    let fixture = fixture();
    forged_tag(&fixture, "v-cms", &good_message(), CMS_BLOCK);
    let repo = fixture.repo();
    let first = git(&repo, &["rev-parse", "refs/heads/registry"])
        .trim()
        .to_owned();
    let approved = result(&fixture, &request(&fixture, "v-cms", true));
    assert_eq!(
        approved["trust"]["identity_registry_commit"], first,
        "{approved}"
    );
    assert_eq!(approved["signature"]["identity_registry_commit"], first);
    assert_eq!(approved["approver"], PRINCIPAL);

    // Another committed registry names a different person for the same identity:
    // a different approver under a different trust.
    let second = commit_registry(
        &repo,
        "other",
        &registry_json(&[(IDENTITY, ISSUER, "Other Person")]),
    );
    let mut other = request(&fixture, "v-cms", true);
    other["identity_registry"] = json!("other:registry.json");
    let changed = result(&fixture, &other);
    assert_eq!(changed["approver"], "Other Person", "{changed}");
    assert_eq!(changed["trust"]["identity_registry"], "other:registry.json");
    assert_eq!(changed["trust"]["identity_registry_commit"], second);
    assert_ne!(first, second);

    // The echo is on every status, an untrusted signer included.
    fixture.script("exact_exact", 1);
    let untrusted = result(&fixture, &request(&fixture, "v-cms", true));
    assert_eq!(untrusted["status"], "untrusted_signer");
    assert_eq!(untrusted["trust"]["identity_registry_commit"], first);

    // A request with no `gitsign` carries neither member.
    let plain = result(&fixture, &request(&fixture, "v-cms", false));
    assert!(plain["trust"].get("identity_registry").is_none());
    assert!(plain["trust"].get("identity_registry_commit").is_none());

    // A git failure while the registry is read yields no commit: not a repository.
    let not_a_repository = fixture.root().join("plain");
    fs::create_dir(&not_a_repository).expect("plain directory");
    let mut body = request(&fixture, "v-cms", true);
    body["repository"] = json!(not_a_repository.display().to_string());
    let failed = result(&fixture, &body);
    assert_eq!(failed["status"], "unverifiable", "{failed}");
    assert_eq!(failed["reason"], "repository_unavailable");
    assert!(failed["tag_object_id"].is_null());
    assert_eq!(failed["trust"]["identity_registry"], REGISTRY);
    assert!(failed["trust"].get("identity_registry_commit").is_none());
    assert!(failed.get("approver").is_none());
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
    fixture.script("exact_exact", 0);

    // An identity gitsign accepts but no registry entry names is not an approval.
    commit_registry(
        &fixture.repo(),
        "registry",
        &registry_json(&[("bob@example.test", ISSUER, "Bob")]),
    );
    let unnamed = result(&fixture, &request(&fixture, "v-cms", true));
    assert_eq!(unnamed["status"], "untrusted_signer", "{unnamed}");
    assert_eq!(unnamed["signature"]["status"], "signed_valid");
    assert_eq!(unnamed["signature"]["signer"]["mapping"], "unmapped");
    for member in ["approver", "meaning", "signed_at", "signature_kind"] {
        assert!(unnamed.get(member).is_none(), "{member}: {unnamed}");
    }

    let missing = result(&fixture, &request(&fixture, "v-cms", false));
    assert_eq!(missing["status"], "unverifiable", "{missing}");
    assert_eq!(missing["reason"], "gitsign_identity_missing");
    assert!(missing["trust"].get("gitsign").is_none());
}

/// The machine error of a request that yields no verdict, asserting exit 2.
fn refusal(fixture: &Fixture, request: &Value, path: &Path) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["approval-verify", "--request", &request.to_string()])
        .env_clear()
        .env("PATH", path)
        .env("HOME", fixture.root().join("home"))
        .output()
        .expect("the Cargo-built CLI must start");
    assert_eq!(output.status.code(), Some(2), "{request}: {output:?}");
    serde_json::from_slice(&output.stdout).expect("machine error")
}

#[test]
#[trace("TC-327", "FR-041-AC-10")]
fn tc_327_the_gitsign_inputs_are_refused_with_their_own_codes() {
    let fixture = fixture();
    forged_tag(&fixture, "v-cms", &good_message(), CMS_BLOCK);
    let no_git = fixture.root().join("no-git");
    fs::create_dir(&no_git).expect("empty PATH directory");
    let code = |request: &Value, path: &Path| refusal(&fixture, request, path)["code"].clone();

    // A request error: registry absent with `gitsign`, present without it, malformed.
    let mut without_registry = request(&fixture, "v-cms", true);
    without_registry
        .as_object_mut()
        .expect("object")
        .remove("identity_registry");
    let mut without_gitsign = request(&fixture, "v-cms", false);
    without_gitsign["identity_registry"] = json!(REGISTRY);
    let mut malformed = request(&fixture, "v-cms", true);
    malformed["identity_registry"] = json!("registry:../registry.json");
    let mut with_principal_no_registry = without_registry.clone();
    with_principal_no_registry["gitsign"]["principal"] = json!("p");
    for body in [
        &without_registry,
        &without_gitsign,
        &malformed,
        &with_principal_no_registry,
    ] {
        assert_eq!(
            code(body, &no_git),
            "approval_verify_request_invalid",
            "{body}"
        );
    }

    // A principal in a request that otherwise validates: refused before git starts.
    let mut principal = request(&fixture, "v-cms", true);
    principal["gitsign"]["principal"] = json!("Mallory");
    let error = refusal(&fixture, &principal, &no_git);
    assert_eq!(error["code"], "approval_verify_gitsign_principal_refused");
    assert!(!error.to_string().contains("Mallory"));

    // An unusable registry starts git (it reads the repository) and no verifier.
    let mut unresolved = request(&fixture, "v-cms", true);
    unresolved["identity_registry"] = json!("no-such-branch:registry.json");
    let mut absent = request(&fixture, "v-cms", true);
    absent["identity_registry"] = json!("registry:missing.json");
    commit_registry(
        &fixture.repo(),
        "broken",
        "{\"protocol\":\"wrong\",\"entries\":[]}",
    );
    let mut wrong_protocol = request(&fixture, "v-cms", true);
    wrong_protocol["identity_registry"] = json!("broken:registry.json");
    for body in [&unresolved, &absent, &wrong_protocol] {
        assert_eq!(
            code(body, &fixture.bin()),
            "approval_verify_identity_registry_invalid",
            "{body}"
        );
    }
    assert!(!fixture.bin().join("invocations").exists());
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

#[test]
#[trace("TC-1011", "FR-041-AC-25")]
fn tc_1011_the_registry_captured_before_verification_survives_a_moved_branch_and_an_edited_worktree()
 {
    let fixture = fixture();
    forged_tag(&fixture, "v-cms", &good_message(), CMS_BLOCK);
    let repo = fixture.repo();
    // A second registry, on its own branch, that names nobody for the identity.
    let empty = commit_registry(&repo, "empty", &registry_json(&[]));
    // The same registry as a tracked file on main, for the working-tree edit.
    fs::write(
        repo.join("registry.json"),
        registry_json(&[(IDENTITY, ISSUER, PRINCIPAL)]),
    )
    .expect("registry file");
    git(&repo, &["add", "registry.json"]);
    git(&repo, &["commit", "-q", "-m", "registry on main"]);
    let main_commit = git(&repo, &["rev-parse", "main"]).trim().to_owned();
    let registry_commit = git(&repo, &["rev-parse", "refs/heads/registry"])
        .trim()
        .to_owned();

    let real_git = which("git");
    let during = format!(
        "{git} -C '{repo}' update-ref refs/heads/registry '{empty}'\nprintf '%s' '{blank}' > '{repo}/registry.json'\nexit \"$status\"",
        git = real_git.display(),
        repo = repo.display(),
        blank = registry_json(&[]),
    );
    fs::write(
        fixture.bin().join("gitsign"),
        STAND_IN.replace("exit \"$status\"", &during),
    )
    .expect("stand-in");

    // The branch the revision names moves while gitsign runs.
    let moved = result(&fixture, &request(&fixture, "v-cms", true));
    assert_eq!(moved["status"], "approved", "{moved}");
    assert_eq!(moved["approver"], PRINCIPAL);
    assert_eq!(moved["trust"]["identity_registry_commit"], registry_commit);
    assert_eq!(
        git(&repo, &["rev-parse", "refs/heads/registry"]).trim(),
        empty,
        "the stand-in moved the branch"
    );

    // The working-tree registry is edited while gitsign runs.
    let mut worktree = request(&fixture, "v-cms", true);
    worktree["identity_registry"] = json!("main:registry.json");
    let edited = result(&fixture, &worktree);
    assert_eq!(edited["status"], "approved", "{edited}");
    assert_eq!(edited["approver"], PRINCIPAL);
    assert_eq!(edited["trust"]["identity_registry_commit"], main_commit);
    assert_eq!(
        fs::read_to_string(repo.join("registry.json")).expect("file"),
        registry_json(&[]),
        "the stand-in edited the working tree"
    );
}
