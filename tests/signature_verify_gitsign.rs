// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary tests for `signature-verify` on gitsign-signed objects
//! (FR-040). `gitsign` is not installed on the build machines, so every test
//! but the ignored live one runs a stand-in executable named `gitsign` that
//! records its arguments, working directory and environment and exits with a
//! scripted status. The CMS signature blocks are forged with `git hash-object`
//! and the identities are fictional.

use std::{
    fs,
    io::Write,
    os::unix::fs::{PermissionsExt, symlink},
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

/// Records every call as `args_N`, `cwd_N` and `env_N` (the environment the
/// process was started with) in its own directory, then exits with the status
/// scripted for the identity and issuer flavor of the call.
const STAND_IN: &str = r#"#!/bin/sh
d=${0%/*}
n=0
[ -e "$d/count" ] && read -r n < "$d/count"
n=$((n + 1))
echo "$n" > "$d/count"
for a in "$@"; do printf '%s\n' "$a"; done > "$d/args_$n"
pwd > "$d/cwd_$n"
/usr/bin/tr '\0' '\n' < /proc/$$/environ > "$d/env_$n"
id=exact
iss=exact
for a in "$@"; do
  case "$a" in
    --certificate-identity-regexp) id=any ;;
    --certificate-oidc-issuer-regexp) iss=any ;;
  esac
done
[ -e "$d/flood" ] && /usr/bin/head -c 9000000 /dev/zero
status=0
if [ -e "$d/exit_${id}_${iss}" ]; then read -r status < "$d/exit_${id}_${iss}"
elif [ -e "$d/exit_default" ]; then read -r status < "$d/exit_default"; fi
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

    fn home(&self) -> PathBuf {
        self.root().join("home")
    }

    fn allowed(&self) -> PathBuf {
        self.root().join("allowed_signers")
    }

    /// Scripts the stand-in's exit status for one flavor, such as `exact_exact`.
    fn script(&self, flavor: &str, status: i32) {
        fs::write(
            self.bin().join(format!("exit_{flavor}")),
            format!("{status}\n"),
        )
        .expect("script");
    }

    fn calls(&self) -> usize {
        fs::read_to_string(self.bin().join("count"))
            .map_or(0, |text| text.trim().parse().expect("count"))
    }

    fn lines(&self, name: &str, call: usize) -> Vec<String> {
        fs::read_to_string(self.bin().join(format!("{name}_{call}")))
            .expect("record")
            .lines()
            .map(str::to_owned)
            .collect()
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

/// A repository with one plain commit, an empty allowed-signers file and a
/// `bin` directory holding `git` and the stand-in `gitsign`.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir(root.join("home")).expect("home");
    fs::write(root.join("allowed_signers"), "").expect("allowed signers");
    let bin = root.join("bin");
    fs::create_dir(&bin).expect("bin");
    // A `git` that logs its arguments, then runs the real one.
    let wrapper = format!(
        "#!/bin/sh\necho \"$*\" >> \"${{0%/*}}/gitlog\"\nexec {} \"$@\"\n",
        which("git").display()
    );
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

/// A continued header value: every line after the first starts with a space.
fn header_block(block: &str) -> String {
    block.trim_end_matches('\n').replace('\n', "\n ")
}

/// A commit whose message is `message` and whose `gpgsig` header is `block`.
fn forged_commit(fixture: &Fixture, message: &str, block: &str) -> String {
    let repo = fixture.repo();
    let tree = git(&repo, &["rev-parse", "HEAD^{tree}"]).trim().to_owned();
    let parent = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    let who = "t <t@example.test> 1700000000 +0000";
    let object = format!(
        "tree {tree}\nparent {parent}\nauthor {who}\ncommitter {who}\ngpgsig {}\n\n{message}\n",
        header_block(block)
    );
    let id = store(&repo, "commit", &object);
    git(&repo, &["update-ref", "refs/heads/signed", &id]);
    id
}

/// An annotated tag named `name` whose message ends in `block`.
fn forged_tag(fixture: &Fixture, name: &str, message: &str, block: &str) -> String {
    let repo = fixture.repo();
    let commit = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    let object = format!(
        "object {commit}\ntype commit\ntag {name}\ntagger t <t@example.test> 1700000000 +0000\n\n{message}\n{block}"
    );
    let id = store(&repo, "tag", &object);
    git(&repo, &["update-ref", &format!("refs/tags/{name}"), &id]);
    id
}

fn request(fixture: &Fixture, revision: &str, kind: &str, gitsign: bool) -> Value {
    let mut value = json!({
        "protocol": "engineering-assurance.signature-verify-request/v1",
        "repository": fixture.repo().display().to_string(),
        "revision": revision,
        "kind": kind,
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

fn run(fixture: &Fixture, body: &[u8], path: &Path) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["signature-verify", "--request", "-"])
        .env_clear()
        .env("PATH", path)
        .env("HOME", fixture.home())
        .env("SIGSTORE_ROOT_FILE", "no-such-root")
        .env("GITSIGN_FULCIO_ROOT", "no-such-fulcio")
        .env("TUF_ROOT", "no-such-tuf")
        .env("GIT_DIR", "no-such-git-dir")
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

fn verdict(fixture: &Fixture, request: &Value) -> Value {
    let bin = fixture.bin();
    verdict_on(fixture, request, &bin)
}

fn verdict_on(fixture: &Fixture, request: &Value, path: &Path) -> Value {
    let output = run(fixture, request.to_string().as_bytes(), path);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    serde_json::from_slice(&output.stdout).expect("json line")
}

#[test]
#[trace("TC-278", "FR-040-AC-1", "FR-040-CON-1")]
fn tc_278_an_accepted_commit_and_tag_are_signed_valid_with_the_request_identity() {
    let fixture = fixture();
    let id = forged_commit(&fixture, "signed", CMS_BLOCK);
    forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    for (revision, kind) in [(id.as_str(), "commit"), ("v-cms", "tag")] {
        let result = verdict(&fixture, &request(&fixture, revision, kind, true));
        assert_eq!(result["status"], "signed_valid", "{result}");
        assert_eq!(result["signature_kind"], "gitsign");
        assert_eq!(
            result["certificate"],
            json!({"identity": IDENTITY, "oidc_issuer": ISSUER})
        );
        assert_eq!(
            result["signer"],
            json!({"source": "request", "mapping": "mapped", "principal": PRINCIPAL})
        );
        assert!(result.get("reason").is_none());
        assert!(result.get("key_fingerprint").is_none());
    }
    assert_eq!(fixture.calls(), 2);
}

#[test]
#[trace("TC-279", "FR-040-AC-2")]
fn tc_279_gitsign_gets_the_resolved_commit_id_or_bare_tag_name_and_runs_in_the_repository() {
    let fixture = fixture();
    let id = forged_commit(&fixture, "signed", CMS_BLOCK);
    forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    // A branch name resolves to the id; the request text is never passed.
    verdict(&fixture, &request(&fixture, "signed", "commit", true));
    verdict(&fixture, &request(&fixture, "v-cms", "tag", true));
    verdict(&fixture, &request(&fixture, "refs/tags/v-cms", "tag", true));
    assert_eq!(fixture.calls(), 3);
    let flags = |target: &str, sub: &str| {
        vec![
            sub.to_owned(),
            "--certificate-identity".to_owned(),
            IDENTITY.to_owned(),
            "--certificate-oidc-issuer".to_owned(),
            ISSUER.to_owned(),
            target.to_owned(),
        ]
    };
    assert_eq!(fixture.lines("args", 1), flags(&id, "verify"));
    assert_eq!(fixture.lines("args", 2), flags("v-cms", "verify-tag"));
    assert_eq!(fixture.lines("args", 3), flags("v-cms", "verify-tag"));
    let repo = fs::canonicalize(fixture.repo()).expect("repo");
    for call in 1..=3 {
        let cwd = fs::canonicalize(&fixture.lines("cwd", call)[0]).expect("cwd");
        assert_eq!(cwd, repo);
    }
    // Git is never asked to verify the CMS object.
    let log = fs::read_to_string(fixture.bin().join("gitlog")).expect("git log");
    assert!(
        !log.contains("verify-commit") && !log.contains("verify-tag"),
        "{log}"
    );
}

#[test]
#[trace("TC-280", "FR-040-AC-3")]
fn tc_280_a_wildcard_only_acceptance_is_untrusted_with_the_mismatch() {
    let cases = [
        // (exact+exact, exact id/any issuer, any id/exact issuer) -> mismatch
        ("exact_any", 0, "any_exact", 1, "issuer", 3),
        ("exact_any", 1, "any_exact", 0, "identity", 4),
        ("exact_any", 1, "any_exact", 1, "identity_and_issuer", 4),
    ];
    for (issuer_flavor, issuer_status, identity_flavor, identity_status, mismatch, calls) in cases {
        let fixture = fixture_with_commit();
        fixture.script("exact_exact", 1);
        fixture.script("any_any", 0);
        fixture.script(issuer_flavor, issuer_status);
        fixture.script(identity_flavor, identity_status);
        let result = verdict(&fixture, &request(&fixture, "signed", "commit", true));
        assert_eq!(result["status"], "signed_untrusted", "{result}");
        assert_eq!(result["mismatch"], mismatch);
        assert_eq!(
            result["signer"],
            json!({"source": "request", "mapping": "unmapped"})
        );
        assert!(result.get("certificate").is_none());
        assert_eq!(result["signature_kind"], "gitsign");
        assert_eq!(fixture.calls(), calls);
    }
}

fn fixture_with_commit() -> Fixture {
    let fixture = fixture();
    forged_commit(&fixture, "signed", CMS_BLOCK);
    fixture
}

#[test]
#[trace("TC-281", "FR-040-AC-4")]
fn tc_281_failures_are_unverifiable_and_never_bad_or_untrusted() {
    let fixture = fixture_with_commit();
    fixture.script("default", 1);
    let result = verdict(&fixture, &request(&fixture, "signed", "commit", true));
    assert_eq!(result["status"], "unverifiable", "{result}");
    assert_eq!(result["reason"], "gitsign_verification_failed");
    assert_eq!(fixture.calls(), 2);

    // Exit 2, or any other status, at each step of the procedure.
    for status in [2, 3, 127] {
        for step in ["exact_exact", "any_any", "exact_any", "any_exact"] {
            let fixture = fixture_with_commit();
            fixture.script("exact_exact", 1);
            fixture.script("any_any", 0);
            fixture.script("exact_any", 1);
            fixture.script("any_exact", 1);
            fixture.script(step, status);
            let result = verdict(&fixture, &request(&fixture, "signed", "commit", true));
            assert_eq!(
                result["status"], "unverifiable",
                "{step} {status}: {result}"
            );
            assert_eq!(result["reason"], "gitsign_verification_failed");
        }
    }
}

#[test]
#[trace("TC-282", "FR-040-AC-5")]
fn tc_282_a_cms_object_without_a_gitsign_member_is_missing_its_identity() {
    let fixture = fixture_with_commit();
    forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    for (revision, kind) in [("signed", "commit"), ("v-cms", "tag")] {
        let result = verdict(&fixture, &request(&fixture, revision, kind, false));
        assert_eq!(result["status"], "unverifiable", "{result}");
        assert_eq!(result["reason"], "gitsign_identity_missing");
        assert_eq!(result["signature_kind"], "gitsign");
    }
    assert_eq!(fixture.calls(), 0);
}

#[test]
#[trace("TC-283", "FR-040-AC-6")]
fn tc_283_a_missing_gitsign_executable_is_not_started() {
    let fixture = fixture_with_commit();
    forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    let bare = fixture.root().join("bare");
    fs::create_dir(&bare).expect("bare");
    symlink(which("git"), bare.join("git")).expect("git link");
    for (revision, kind) in [("signed", "commit"), ("v-cms", "tag")] {
        let result = verdict_on(&fixture, &request(&fixture, revision, kind, true), &bare);
        assert_eq!(result["status"], "unverifiable", "{result}");
        assert_eq!(result["reason"], "gitsign_not_started");
    }
}

#[test]
#[trace("TC-284", "FR-040-AC-7")]
fn tc_284_output_above_the_capture_bound_is_output_exceeded() {
    let fixture = fixture_with_commit();
    fs::write(fixture.bin().join("flood"), "").expect("flood");
    let result = verdict(&fixture, &request(&fixture, "signed", "commit", true));
    assert_eq!(result["status"], "unverifiable", "{result}");
    assert_eq!(result["reason"], "output_exceeded");
}

#[test]
#[trace("TC-285", "FR-040-AC-8")]
fn tc_285_unsigned_and_openpgp_objects_are_unchanged_with_or_without_gitsign() {
    let fixture = fixture();
    git(&fixture.repo(), &["tag", "-a", "-m", "plain", "v-plain"]);
    let pgp = "-----BEGIN PGP SIGNATURE-----\nAAAA\n-----END PGP SIGNATURE-----\n";
    forged_commit(&fixture, "pgp", pgp);
    forged_tag(&fixture, "v-pgp", "tag message", pgp);
    for gitsign in [false, true] {
        for (revision, kind) in [("main", "commit"), ("v-plain", "tag")] {
            let result = verdict(&fixture, &request(&fixture, revision, kind, gitsign));
            assert_eq!(result["status"], "unsigned", "{result}");
            assert!(result.get("signature_kind").is_none());
            assert!(result.get("reason").is_none());
        }
        for (revision, kind) in [("signed", "commit"), ("v-pgp", "tag")] {
            let result = verdict(&fixture, &request(&fixture, revision, kind, gitsign));
            assert_eq!(result["status"], "unverifiable", "{result}");
            assert_eq!(result["reason"], "unsupported_signature_format");
            assert!(result.get("signature_kind").is_none());
        }
    }
    assert_eq!(fixture.calls(), 0);
}

#[test]
#[trace("TC-286", "FR-040-AC-11")]
fn tc_286_a_malformed_gitsign_member_is_refused_before_git_starts() {
    let fixture = fixture_with_commit();
    let empty = fixture.root().join("empty");
    fs::create_dir(&empty).expect("empty");
    let long = "x".repeat(257);
    let bad: Vec<Value> = vec![
        json!({"certificate_oidc_issuer": ISSUER, "principal": PRINCIPAL}),
        json!({"certificate_identity": IDENTITY, "principal": PRINCIPAL}),
        json!({"certificate_identity": IDENTITY, "certificate_oidc_issuer": ISSUER}),
        json!({"certificate_identity": "", "certificate_oidc_issuer": ISSUER, "principal": PRINCIPAL}),
        json!({"certificate_identity": IDENTITY, "certificate_oidc_issuer": ISSUER, "principal": ""}),
        json!({"certificate_identity": long, "certificate_oidc_issuer": ISSUER, "principal": PRINCIPAL}),
        json!({"certificate_identity": IDENTITY, "certificate_oidc_issuer": long, "principal": PRINCIPAL}),
        json!({"certificate_identity": IDENTITY, "certificate_oidc_issuer": ISSUER, "principal": long}),
        json!({"certificate_identity": "-alice@example.test", "certificate_oidc_issuer": ISSUER, "principal": PRINCIPAL}),
        json!({"certificate_identity": IDENTITY, "certificate_oidc_issuer": concat!("-https:", "//x.test"), "principal": PRINCIPAL}),
        json!({"certificate_identity": "alice\n@example.test", "certificate_oidc_issuer": ISSUER, "principal": PRINCIPAL}),
        json!({"certificate_identity": IDENTITY, "certificate_oidc_issuer": ISSUER, "principal": "A\u{7}"}),
        json!({"certificate_identity": IDENTITY, "certificate_oidc_issuer": ISSUER, "principal": PRINCIPAL, "extra": 1}),
        json!("alice"),
    ];
    for gitsign in bad {
        let mut body = request(&fixture, "signed", "commit", false);
        body["gitsign"] = gitsign.clone();
        // An empty PATH could not start git: exit 2 proves git was not started.
        let output = run(&fixture, body.to_string().as_bytes(), &empty);
        assert_eq!(output.status.code(), Some(2), "{gitsign}: {output:?}");
        let error: Value = serde_json::from_slice(&output.stdout).expect("machine error");
        assert_eq!(
            error["code"], "signature_verify_request_invalid",
            "{gitsign}"
        );
    }
    assert_eq!(fixture.calls(), 0);
}

fn ssh_fixture() -> (Fixture, String) {
    let fixture = fixture();
    let root = fixture.root();
    let key = root.join("key");
    run_program(
        root,
        "ssh-keygen",
        &[
            "-q",
            "-t",
            "ed25519",
            "-N",
            "",
            "-C",
            "k",
            "-f",
            key.to_str().expect("utf-8"),
        ],
        None,
    );
    let public = fs::read_to_string(root.join("key.pub")).expect("public key");
    fs::write(fixture.allowed(), format!("signer@example.test {public}")).expect("allowed");
    let key = root.join("key.pub").display().to_string();
    (fixture, key)
}

#[test]
#[trace("TC-287", "FR-040-AC-12")]
fn tc_287_ssh_objects_keep_the_ssh_verdict_and_never_start_gitsign() {
    let (fixture, key) = ssh_fixture();
    let repo = fixture.repo();
    let sign = |extra: &[&str]| {
        let mut all = vec!["-c", "gpg.format=ssh", "-c"];
        let setting = format!("user.signingkey={key}");
        all.push(&setting);
        all.extend_from_slice(extra);
        git(&repo, &all);
    };
    sign(&["commit", "-q", "-S", "--allow-empty", "-m", "ssh signed"]);
    sign(&["tag", "-s", "-m", "ssh tag", "v-ssh"]);
    // The real git and ssh-keygen are needed for the SSH path.
    let path = fixture.root().join("ssh-bin");
    fs::create_dir(&path).expect("ssh-bin");
    symlink(which("git"), path.join("git")).expect("git link");
    symlink(which("ssh-keygen"), path.join("ssh-keygen")).expect("ssh-keygen link");
    symlink(fixture.bin().join("gitsign"), path.join("gitsign")).expect("gitsign link");
    for (revision, kind) in [("HEAD", "commit"), ("v-ssh", "tag")] {
        let result = verdict_on(&fixture, &request(&fixture, revision, kind, true), &path);
        assert_eq!(result["status"], "signed_valid", "{result}");
        assert_eq!(result["signature_kind"], "ssh");
        assert_eq!(
            result["signer"],
            json!({"source": "allowed_signers", "mapping": "mapped", "principal": "signer@example.test"})
        );
        assert!(result.get("certificate").is_none());
        assert!(result.get("mismatch").is_none());
    }
    assert_eq!(fixture.calls(), 0);
}

#[test]
#[trace("TC-288", "FR-040-AC-13")]
fn tc_288_the_signed_off_meaning_is_reported_for_every_gitsign_status() {
    let fixture = fixture();
    let message = "work\n\nSigned-off-meaning: reviewed";
    forged_commit(&fixture, message, CMS_BLOCK);
    forged_tag(&fixture, "v-cms", message, CMS_BLOCK);
    let expect = json!({"presence": "present", "values": ["reviewed"]});
    for (revision, kind) in [("signed", "commit"), ("v-cms", "tag")] {
        let valid = verdict(&fixture, &request(&fixture, revision, kind, true));
        assert_eq!(valid["status"], "signed_valid");
        assert_eq!(valid["signed_off_meaning"], expect);
        let missing = verdict(&fixture, &request(&fixture, revision, kind, false));
        assert_eq!(missing["status"], "unverifiable");
        assert_eq!(missing["signed_off_meaning"], expect);
    }
    fixture.script("default", 1);
    let failed = verdict(&fixture, &request(&fixture, "signed", "commit", true));
    assert_eq!(failed["reason"], "gitsign_verification_failed");
    assert_eq!(failed["signed_off_meaning"], expect);
}

#[test]
#[trace("TC-289", "FR-040-AC-17", "FR-040-CON-1")]
fn tc_289_a_gitsign_key_in_any_repository_configuration_stops_the_run() {
    let local = |f: &Fixture| {
        git(&f.repo(), &["config", "gitsign.fulcioRoot", "/x"]);
    };
    let included = |f: &Fixture| {
        let file = f.root().join("included.cfg");
        fs::write(&file, "[gitsign]\n\trekorURL = rekor-log\n").expect("include");
        git(
            &f.repo(),
            &["config", "include.path", file.to_str().expect("utf-8")],
        );
    };
    let worktree = |f: &Fixture| {
        git(&f.repo(), &["config", "extensions.worktreeConfig", "true"]);
        git(
            &f.repo(),
            &["config", "--worktree", "gitsign.connectorID", "x"],
        );
    };
    let setters: [&dyn Fn(&Fixture); 3] = [&local, &included, &worktree];
    for set in setters {
        let fixture = fixture_with_commit();
        forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
        set(&fixture);
        for (revision, kind) in [("signed", "commit"), ("v-cms", "tag")] {
            let result = verdict(&fixture, &request(&fixture, revision, kind, true));
            assert_eq!(result["status"], "unverifiable", "{result}");
            assert_eq!(result["reason"], "gitsign_repository_config");
        }
        assert_eq!(fixture.calls(), 0);
    }
    let clean = fixture_with_commit();
    git(&clean.repo(), &["config", "gitsignfoo.other", "unrelated"]);
    let result = verdict(&clean, &request(&clean, "signed", "commit", true));
    assert_eq!(result["status"], "signed_valid", "{result}");
    assert_eq!(clean.calls(), 1);
}

#[test]
#[trace("TC-290", "FR-040-AC-18")]
fn tc_290_gitsign_gets_exactly_four_environment_variables() {
    let fixture = fixture_with_commit();
    verdict(&fixture, &request(&fixture, "signed", "commit", true));
    let mut names: Vec<String> = fixture
        .lines("env", 1)
        .iter()
        .filter_map(|line| line.split_once('=').map(|(name, _)| name.to_owned()))
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["GIT_CONFIG_GLOBAL", "GIT_CONFIG_NOSYSTEM", "HOME", "PATH"]
    );
    let env = fixture.lines("env", 1);
    assert!(env.contains(&"GIT_CONFIG_NOSYSTEM=1".to_owned()));
    assert!(env.contains(&"GIT_CONFIG_GLOBAL=/dev/null".to_owned()));
    assert!(env.contains(&format!("HOME={}", fixture.home().display())));
}

#[test]
#[trace("TC-291", "FR-040-AC-19")]
fn tc_291_a_tag_that_is_not_an_annotated_tag_ref_never_starts_gitsign() {
    let fixture = fixture_with_commit();
    git(&fixture.repo(), &["tag", "light"]);
    let id = forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    let light = verdict(&fixture, &request(&fixture, "light", "tag", true));
    assert_eq!(light["status"], "unsigned", "{light}");
    assert_eq!(light["reason"], "lightweight_tag");
    // The tag object's own id, a branch and an unknown name are not tag refs.
    for revision in [id.as_str(), "main", "refs/tags/nope", "v-cms^{tag}"] {
        let result = verdict(&fixture, &request(&fixture, revision, "tag", true));
        assert_eq!(result["status"], "unverifiable", "{revision}: {result}");
        assert_eq!(result["reason"], "revision_unresolved", "{revision}");
    }
    assert_eq!(fixture.calls(), 0);
}

#[test]
#[ignore = "needs the real gitsign, network access and a public gitsign-signed commit and tag named by FR_040_LIVE_*"]
#[trace("TC-292", "FR-040-AC-15")]
fn tc_292_real_gitsign_verifies_a_public_signed_commit_and_tag() {
    let (Ok(repository), Ok(commit), Ok(tag), Ok(identity), Ok(issuer)) = (
        std::env::var("FR_040_LIVE_REPOSITORY"),
        std::env::var("FR_040_LIVE_COMMIT"),
        std::env::var("FR_040_LIVE_TAG"),
        std::env::var("FR_040_LIVE_IDENTITY"),
        std::env::var("FR_040_LIVE_ISSUER"),
    ) else {
        panic!("set FR_040_LIVE_REPOSITORY, _COMMIT, _TAG, _IDENTITY and _ISSUER");
    };
    let fixture = fixture();
    fs::write(fixture.allowed(), "").expect("allowed");
    let path = std::env::var_os("PATH").expect("PATH");
    let live = |revision: &str, kind: &str, identity: &str, issuer: &str| {
        let mut body = request(&fixture, revision, kind, true);
        body["repository"] = json!(repository);
        body["gitsign"]["certificate_identity"] = json!(identity);
        body["gitsign"]["certificate_oidc_issuer"] = json!(issuer);
        verdict_on(&fixture, &body, Path::new(&path))
    };
    for (revision, kind) in [(&commit, "commit"), (&tag, "tag")] {
        assert_eq!(
            live(revision, kind, &identity, &issuer)["status"],
            "signed_valid"
        );
        let wrong = live(revision, kind, &identity, OTHER_ISSUER);
        assert_eq!(wrong["status"], "signed_untrusted");
        assert_eq!(wrong["mismatch"], "issuer");
    }
}
