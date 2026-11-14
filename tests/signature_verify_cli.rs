// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary tests for `signature-verify` (FR-036) on real repositories
//! signed with fictional ed25519 keys generated inside each test.

use std::{
    env, fs,
    io::Write,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use ix_trace_rs::trace;
use serde_json::{Value, json};

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

    /// Fingerprint of `key` as `ssh-keygen` prints it.
    fn fingerprint(&self, key: &str) -> String {
        let output = Command::new("ssh-keygen")
            .arg("-lf")
            .arg(self.public(key))
            .output()
            .expect("ssh-keygen must start");
        let text = String::from_utf8(output.stdout).expect("utf-8");
        text.split_whitespace()
            .nth(1)
            .expect("fingerprint")
            .to_owned()
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

/// A repository, keys `a` and `b`, and an allowed-signers file naming only `a`.
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
    let public = fs::read_to_string(root.join("a.pub")).expect("public key");
    fs::write(
        root.join("allowed_signers"),
        format!("signer@example.invalid {public}"),
    )
    .expect("allowed signers");
    let repo = root.join("repo");
    fs::create_dir(&repo).expect("repo dir");
    git(&repo, None, &["init", "-q", "-b", "main"]);
    fs::write(repo.join("file"), "content\n").expect("file");
    git(&repo, None, &["add", "file"]);
    Fixture { dir }
}

fn signed_commit(fixture: &Fixture, key: &str, message: &str) -> String {
    let public = fixture.public(key);
    let repo = fixture.repo();
    git(
        &repo,
        Some(public.to_str().expect("utf-8")),
        &["commit", "-q", "-S", "--allow-empty", "-m", message],
    );
    git(&repo, None, &["rev-parse", "HEAD"]).trim().to_owned()
}

fn plain_commit(fixture: &Fixture, message: &str) -> String {
    let repo = fixture.repo();
    git(
        &repo,
        None,
        &["commit", "-q", "--allow-empty", "-m", message],
    );
    git(&repo, None, &["rev-parse", "HEAD"]).trim().to_owned()
}

fn signed_tag(fixture: &Fixture, key: &str, name: &str) {
    let public = fixture.public(key);
    git(
        &fixture.repo(),
        Some(public.to_str().expect("utf-8")),
        &["tag", "-s", "-m", "tag message", name],
    );
}

/// Rewrites an object's bytes with `edit` and stores the result.
fn rewrite(fixture: &Fixture, kind: &str, revision: &str, edit: impl Fn(&str) -> String) -> String {
    let repo = fixture.repo();
    let object = git(&repo, None, &["cat-file", kind, revision]);
    let edited = edit(&object);
    let id = run_program(
        "git",
        &repo,
        &["hash-object", "-t", kind, "-w", "--stdin"],
        Some(edited.as_bytes()),
    );
    String::from_utf8(id).expect("utf-8").trim().to_owned()
}

const FORGED_PGP: &str = "-----BEGIN PGP SIGNATURE-----\nAAAA\n-----END PGP SIGNATURE-----\n";

fn request(fixture: &Fixture, revision: &str, kind: &str) -> Value {
    json!({
        "protocol": "engineering-assurance.signature-verify-request/v1",
        "repository": fixture.repo().display().to_string(),
        "revision": revision,
        "kind": kind,
        "allowed_signers": fixture.allowed().display().to_string(),
    })
}

fn run_with(body: &[u8], path: Option<&Path>) -> Output {
    run_with_temp(body, path, None)
}

fn run_with_temp(body: &[u8], path: Option<&Path>, temp: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"));
    command
        .args(["signature-verify", "--request", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = path {
        command.env("PATH", path);
    }
    if let Some(temp) = temp {
        command.env("TMPDIR", temp);
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

fn verdict_with(request: &Value, path: Option<&Path>) -> Value {
    let output = run_with(request.to_string().as_bytes(), path);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = std::str::from_utf8(&output.stdout).expect("utf-8");
    assert!(
        text.ends_with('\n') && text.matches('\n').count() == 1,
        "{text:?}"
    );
    let value: Value = serde_json::from_str(text).expect("json line");
    assert_eq!(
        value["protocol"],
        "engineering-assurance.signature-verify-result/v1"
    );
    value
}

fn verdict(request: &Value) -> Value {
    verdict_with(request, None)
}

fn status(fixture: &Fixture, revision: &str, kind: &str) -> Value {
    verdict(&request(fixture, revision, kind))
}

/// A directory holding only the named programs, found on the real `PATH`.
fn path_with(fixture: &Fixture, name: &str, programs: &[&str]) -> PathBuf {
    let directory = fixture.root().join(name);
    fs::create_dir(&directory).expect("bin dir");
    let search = env::var_os("PATH").expect("PATH");
    for program in programs {
        let found = env::split_paths(&search)
            .map(|dir| dir.join(program))
            .find(|candidate| candidate.is_file())
            .expect("program on PATH");
        symlink(found, directory.join(program)).expect("symlink");
    }
    directory
}

fn git_recording_path(fixture: &Fixture, name: &str, log: &Path, fail_cat_file: bool) -> PathBuf {
    let directory = fixture.root().join(name);
    fs::create_dir(&directory).expect("bin dir");
    let real_git = env::split_paths(&env::var_os("PATH").expect("PATH"))
        .map(|dir| dir.join("git"))
        .find(|candidate| candidate.is_file())
        .expect("real git on PATH");
    let script = directory.join("git");
    let fail = if fail_cat_file {
        "if [ \"$1\" = cat-file ]; then exit 1; fi\n"
    } else {
        ""
    };
    fs::write(
        &script,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n{fail}exec '{}' \"$@\"\n",
            log.display(),
            real_git.display()
        ),
    )
    .expect("git recorder");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("mode");
    directory
}

#[test]
#[trace("TC-250", "FR-036-AC-1", "FR-036-CON-1")]
fn tc_250_valid_commit_and_tag_are_signed_valid() {
    let fixture = fixture();
    let id = signed_commit(&fixture, "a", "signed by a");
    signed_tag(&fixture, "a", "v-valid");
    let commit = status(&fixture, &id, "commit");
    assert_eq!(commit["status"], "signed_valid");
    assert_eq!(commit["kind"], "commit");
    assert_eq!(commit["key_fingerprint"], fixture.fingerprint("a"));
    assert!(commit.get("reason").is_none());
    let tag = status(&fixture, "v-valid", "tag");
    assert_eq!(tag["status"], "signed_valid");
    assert_eq!(tag["kind"], "tag");
    assert!(tag.get("key_fingerprint").is_none());
    // A branch name resolves like an id.
    assert_eq!(status(&fixture, "main", "commit")["status"], "signed_valid");
}

#[test]
#[trace("TC-251", "FR-036-AC-2")]
fn tc_251_unsigned_commit_and_tag_are_unsigned() {
    let fixture = fixture();
    let id = plain_commit(&fixture, "plain");
    git(
        &fixture.repo(),
        None,
        &["tag", "-a", "-m", "plain tag", "v-plain"],
    );
    let commit = status(&fixture, &id, "commit");
    assert_eq!(commit["status"], "unsigned");
    assert!(commit.get("reason").is_none());
    assert!(commit.get("key_fingerprint").is_none());
    let tag = status(&fixture, "v-plain", "tag");
    assert_eq!(tag["status"], "unsigned");
    assert!(tag.get("reason").is_none());
}

#[test]
#[trace("TC-252", "FR-036-AC-3")]
fn tc_252_unlisted_key_is_untrusted_and_altered_objects_are_bad() {
    let fixture = fixture();
    let untrusted = signed_commit(&fixture, "b", "signed by b");
    signed_tag(&fixture, "b", "v-untrusted");
    let commit = status(&fixture, &untrusted, "commit");
    assert_eq!(commit["status"], "signed_untrusted");
    assert_eq!(commit["key_fingerprint"], fixture.fingerprint("b"));
    assert_eq!(
        status(&fixture, "v-untrusted", "tag")["status"],
        "signed_untrusted"
    );

    let good = signed_commit(&fixture, "a", "signed by a");
    let altered = rewrite(&fixture, "commit", &good, |o| {
        o.replace("signed by a", "signed by x")
    });
    assert_eq!(status(&fixture, &altered, "commit")["status"], "signed_bad");
    signed_tag(&fixture, "a", "v-altered");
    let altered_tag = rewrite(&fixture, "tag", "v-altered", |o| {
        o.replace("tag message", "tag messagf")
    });
    assert_eq!(
        status(&fixture, &altered_tag, "tag")["status"],
        "signed_bad"
    );
}

#[test]
#[trace("TC-253", "FR-036-AC-4")]
fn tc_253_lightweight_tag_is_unsigned_with_its_reason() {
    let fixture = fixture();
    signed_commit(&fixture, "a", "signed by a");
    git(&fixture.repo(), None, &["tag", "v-light"]);
    let tag = status(&fixture, "v-light", "tag");
    assert_eq!(tag["status"], "unsigned");
    assert_eq!(tag["reason"], "lightweight_tag");
}

#[test]
#[trace("TC-254", "FR-036-AC-5")]
fn tc_254_unavailable_git_repository_or_revision_is_unverifiable() {
    let fixture = fixture();
    signed_commit(&fixture, "a", "signed by a");
    for kind in ["commit", "tag"] {
        let unknown = status(&fixture, "no-such-revision", kind);
        assert_eq!(unknown["status"], "unverifiable", "{kind}");
        assert_eq!(unknown["reason"], "revision_unresolved");
        if kind == "commit" {
            assert_eq!(unknown["verified_commit_id"], Value::Null);
        }
    }
    let plain = tempfile::tempdir().expect("tempdir");
    let mut elsewhere = request(&fixture, "HEAD", "commit");
    elsewhere["repository"] = json!(plain.path().display().to_string());
    let not_repository = verdict(&elsewhere);
    assert_eq!(not_repository["status"], "unverifiable");
    assert_eq!(not_repository["reason"], "repository_unavailable");
    elsewhere["repository"] = json!(fixture.root().join("missing").display().to_string());
    assert_eq!(verdict(&elsewhere)["reason"], "repository_unavailable");

    let empty = path_with(&fixture, "empty-bin", &[]);
    let no_git = verdict_with(&request(&fixture, "HEAD", "commit"), Some(&empty));
    assert_eq!(no_git["status"], "unverifiable");
    assert_eq!(no_git["reason"], "git_not_started");
}

#[test]
#[trace("TC-278", "FR-036-AC-13", "FR-036-AC-15")]
fn tc_278_commit_alias_and_id_return_one_identity_used_by_git_verifiers() {
    let fixture = fixture();
    let id = plain_commit(&fixture, "unsigned commit");
    let log = fixture.root().join("git-args");
    let path = git_recording_path(&fixture, "recording-bin", &log, false);

    let alias = verdict_with(&request(&fixture, "HEAD", "commit"), Some(&path));
    let alias_calls = fs::read_to_string(&log).expect("recorded git calls");
    assert_eq!(alias["status"], "unsigned");
    assert_eq!(alias["verified_commit_id"], id);
    assert!(alias_calls.lines().any(|call| call == format!("verify-commit {id}")));
    assert!(alias_calls.lines().any(|call| {
        call == format!("log -1 --no-show-signature --format=%G?%x00%GF%x00%GS {id}")
    }));

    fs::write(&log, "").expect("clear git call log");
    let by_id = verdict_with(&request(&fixture, &id, "commit"), Some(&path));
    assert_eq!(by_id["verified_commit_id"], id);
    assert_eq!(by_id["verified_commit_id"], alias["verified_commit_id"]);
}

#[test]
#[trace("TC-279", "FR-036-AC-15")]
fn tc_279_commit_id_survives_failure_after_resolution() {
    let fixture = fixture();
    let id = plain_commit(&fixture, "resolved before cat-file fails");
    let log = fixture.root().join("git-args");
    let path = git_recording_path(&fixture, "failing-bin", &log, true);
    let result = verdict_with(&request(&fixture, "HEAD", "commit"), Some(&path));
    assert_eq!(result["status"], "unverifiable");
    assert_eq!(result["verified_commit_id"], id);
}

#[test]
#[trace("TC-280", "FR-036-AC-14", "FR-036-AC-15")]
fn tc_280_tag_and_pre_resolution_verdicts_have_null_commit_identity() {
    let fixture = fixture();
    plain_commit(&fixture, "tag target");
    git(&fixture.repo(), None, &["tag", "v-light"]);
    let lightweight = status(&fixture, "v-light", "tag");
    assert_eq!(lightweight["verified_commit_id"], Value::Null);
    assert_eq!(lightweight["reason"], "lightweight_tag");

    signed_tag(&fixture, "a", "v-signed");
    assert_eq!(status(&fixture, "v-signed", "tag")["verified_commit_id"], Value::Null);
    assert_eq!(status(&fixture, "missing", "commit")["verified_commit_id"], Value::Null);
}

#[test]
#[trace("TC-256", "FR-036-AC-7")]
fn tc_256_invalid_revisions_are_refused_before_git_starts() {
    let fixture = fixture();
    let marker = fixture.root().join("git-was-started");
    let shim = fixture.root().join("shim-bin");
    fs::create_dir(&shim).expect("shim dir");
    let script = shim.join("git");
    fs::write(
        &script,
        format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()),
    )
    .expect("shim");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("mode");
    let long = "a".repeat(257);
    for revision in [
        "",
        long.as_str(),
        "-x",
        "--output=x",
        "a b",
        "a\tb",
        "a\nb",
        "a\u{7}b",
    ] {
        let output = run_with(
            request(&fixture, revision, "commit").to_string().as_bytes(),
            Some(&shim),
        );
        assert_eq!(output.status.code(), Some(2), "{revision:?}: {output:?}");
        let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
        assert_eq!(error["protocol"], "engineering-assurance.error/v1");
        assert_eq!(error["capability"], "signature-verify");
        assert_eq!(error["code"], "signature_verify_revision_invalid");
        assert!(!output.stderr.is_empty());
    }
    assert!(!marker.exists(), "an invalid revision must not start git");
}

#[test]
#[trace("TC-257", "FR-036-AC-8")]
fn tc_257_unreadable_and_nonconforming_requests_are_machine_errors() {
    let fixture = fixture();
    let valid = request(&fixture, "HEAD", "commit");
    let mut cases: Vec<(Vec<u8>, &str)> = vec![
        (b"not json".to_vec(), "signature_verify_request_unreadable"),
        (b"[]".to_vec(), "signature_verify_request_invalid"),
    ];
    let mut mutate = |edit: &dyn Fn(&mut Value)| {
        let mut body = valid.clone();
        edit(&mut body);
        cases.push((
            body.to_string().into_bytes(),
            "signature_verify_request_invalid",
        ));
    };
    mutate(&|v| {
        v.as_object_mut().expect("object").remove("allowed_signers");
    });
    mutate(&|v| v["allowed_signers"] = json!("relative/allowed"));
    mutate(&|v| v["repository"] = json!("relative/repo"));
    mutate(&|v| v["protocol"] = json!("engineering-assurance.signature-verify-request/v2"));
    mutate(&|v| v["kind"] = json!("branch"));
    mutate(&|v| v["extra"] = json!(1));
    mutate(&|v| {
        v.as_object_mut().expect("object").remove("revision");
    });
    for (body, code) in cases {
        let output = run_with(&body, None);
        assert_eq!(output.status.code(), Some(2), "{code}: {output:?}");
        let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
        assert_eq!(error["protocol"], "engineering-assurance.error/v1");
        assert_eq!(error["capability"], "signature-verify");
        assert_eq!(error["code"], code);
        assert!(!output.stderr.is_empty());
    }
    let output = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["signature-verify", "--request", "not json"])
        .output()
        .expect("start");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
#[trace("TC-258", "FR-036-AC-10")]
fn tc_258_openpgp_commit_and_tag_are_unsupported() {
    let fixture = fixture();
    let base = plain_commit(&fixture, "plain");
    let forged = rewrite(&fixture, "commit", &base, |o| {
        o.replacen(
            "\n\n",
            "\ngpgsig -----BEGIN PGP SIGNATURE-----\n AAAA\n -----END PGP SIGNATURE-----\n\n",
            1,
        )
    });
    let commit = status(&fixture, &forged, "commit");
    assert_eq!(commit["status"], "unverifiable");
    assert_eq!(commit["reason"], "unsupported_signature_format");
    git(
        &fixture.repo(),
        None,
        &["tag", "-a", "-m", "plain tag", "v-pgp"],
    );
    let tag_id = rewrite(&fixture, "tag", "v-pgp", |o| format!("{o}{FORGED_PGP}"));
    let tag = status(&fixture, &tag_id, "tag");
    assert_eq!(tag["status"], "unverifiable");
    assert_eq!(tag["reason"], "unsupported_signature_format");
}

#[test]
#[trace("TC-259", "FR-036-AC-11")]
fn tc_259_missing_ssh_keygen_makes_a_rejected_tag_unverifiable_only() {
    let fixture = fixture();
    signed_commit(&fixture, "a", "signed by a");
    signed_tag(&fixture, "b", "v-untrusted");
    git(
        &fixture.repo(),
        None,
        &["tag", "-a", "-m", "plain tag", "v-plain"],
    );
    let pgp = rewrite(&fixture, "tag", "v-plain", |o| format!("{o}{FORGED_PGP}"));
    let git_only = path_with(&fixture, "git-only", &["git"]);

    let rejected = verdict_with(&request(&fixture, "v-untrusted", "tag"), Some(&git_only));
    assert_eq!(rejected["status"], "unverifiable");
    assert_eq!(rejected["reason"], "ssh_keygen_not_started");
    let unsigned = verdict_with(&request(&fixture, "v-plain", "tag"), Some(&git_only));
    assert_eq!(unsigned["status"], "unsigned");
    let unsupported = verdict_with(&request(&fixture, &pgp, "tag"), Some(&git_only));
    assert_eq!(unsupported["reason"], "unsupported_signature_format");
}

/// Makes the repository's own config name a `gpg` that reports a trusted good
/// signature for anything.
fn install_lying_gpg(fixture: &Fixture) {
    let script = fixture.root().join("lying-gpg");
    fs::write(
        &script,
        "#!/bin/sh\necho '[GNUPG:] NEWSIG'\necho '[GNUPG:] GOODSIG ABCDEF0123456789 signer'\necho '[GNUPG:] VALIDSIG ABCDEF0123456789 2026-01-01 1 0 4 0 1 10 00 ABCDEF0123456789'\necho '[GNUPG:] TRUST_ULTIMATE 0 pgp'\nexit 0\n",
    )
    .expect("script");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("mode");
    let path = script.to_str().expect("utf-8").to_owned();
    for key in ["gpg.program", "gpg.openpgp.program", "gpg.x509.program"] {
        git(&fixture.repo(), None, &["config", key, &path]);
    }
}

#[test]
#[trace("TC-260", "FR-036-AC-10", "FR-036-CON-1")]
fn tc_260_a_tag_whose_last_header_line_is_not_ssh_never_verifies_as_valid() {
    let fixture = fixture();
    signed_commit(&fixture, "b", "signed by b");
    signed_tag(&fixture, "b", "v-crafted");
    install_lying_gpg(&fixture);
    // A prefix-only PGP header after the SSH block is the signature to git.
    let crafted = rewrite(&fixture, "tag", "v-crafted", |o| {
        format!("{o}-----BEGIN PGP SIGNATURE----- \nAAAA\n-----END PGP SIGNATURE-----\n")
    });
    let tag = status(&fixture, &crafted, "tag");
    assert_eq!(tag["status"], "unverifiable");
    assert_eq!(tag["reason"], "unsupported_signature_format");
}

#[test]
#[trace("TC-261", "FR-036-AC-10", "FR-036-CON-1")]
fn tc_261_a_commit_with_any_non_ssh_signature_header_never_verifies_as_valid() {
    let fixture = fixture();
    let base = plain_commit(&fixture, "plain");
    install_lying_gpg(&fixture);
    let crafted = rewrite(&fixture, "commit", &base, |o| {
        o.replacen(
            "\n\n",
            "\ngpgsig-sha256 -----BEGIN SSH SIGNATURE-----\n AAAA\n -----END SSH SIGNATURE-----\ngpgsig -----BEGIN PGP SIGNATURE-----\n AAAA\n -----END PGP SIGNATURE-----\n\n",
            1,
        )
    });
    let commit = status(&fixture, &crafted, "commit");
    assert_eq!(commit["status"], "unverifiable");
    assert_eq!(commit["reason"], "unsupported_signature_format");
    assert!(commit.get("key_fingerprint").is_none());
}

#[test]
#[trace("TC-262", "FR-036-AC-1")]
fn tc_262_repository_signature_display_config_does_not_change_the_grade() {
    let fixture = fixture();
    let id = signed_commit(&fixture, "a", "signed by a");
    git(
        &fixture.repo(),
        None,
        &["config", "log.showSignature", "true"],
    );
    let commit = status(&fixture, &id, "commit");
    assert_eq!(commit["status"], "signed_valid");
    assert_eq!(commit["key_fingerprint"], fixture.fingerprint("a"));
}

#[test]
#[trace("TC-263", "FR-036-AC-4")]
fn tc_263_only_a_tag_ref_to_a_commit_is_a_lightweight_tag() {
    let fixture = fixture();
    let id = signed_commit(&fixture, "a", "signed by a");
    git(&fixture.repo(), None, &["tag", "v-light"]);
    assert_eq!(
        status(&fixture, "v-light", "tag")["reason"],
        "lightweight_tag"
    );
    assert_eq!(
        status(&fixture, "refs/tags/v-light", "tag")["reason"],
        "lightweight_tag"
    );
    for revision in [id.as_str(), "main", "HEAD"] {
        let verdict = status(&fixture, revision, "tag");
        assert_eq!(verdict["status"], "unverifiable", "{revision}");
        assert_eq!(verdict["reason"], "revision_unresolved", "{revision}");
    }
}

const PRINCIPAL: &str = "signer@example.invalid";

fn public_line(fixture: &Fixture, key: &str) -> String {
    fs::read_to_string(fixture.public(key)).expect("public key")
}

fn set_allowed(fixture: &Fixture, contents: &str) {
    fs::write(fixture.allowed(), contents).expect("allowed signers");
}

fn signed_commit_with(fixture: &Fixture, key: &str, messages: &[&str]) -> String {
    let public = fixture.public(key);
    let repo = fixture.repo();
    let mut arguments = vec!["commit", "-q", "-S", "--allow-empty"];
    for message in messages {
        arguments.extend(["-m", message]);
    }
    git(&repo, Some(public.to_str().expect("utf-8")), &arguments);
    git(&repo, None, &["rev-parse", "HEAD"]).trim().to_owned()
}

fn signed_tag_with(fixture: &Fixture, key: &str, name: &str, message: &str) {
    let public = fixture.public(key);
    git(
        &fixture.repo(),
        Some(public.to_str().expect("utf-8")),
        &["tag", "-s", "-m", message, name],
    );
}

fn meaning(verdict: &Value) -> (&str, Vec<&str>) {
    let meaning = &verdict["signed_off_meaning"];
    let values = meaning
        .get("values")
        .and_then(Value::as_array)
        .map(|values| values.iter().map(|v| v.as_str().expect("string")).collect())
        .unwrap_or_default();
    (meaning["presence"].as_str().expect("presence"), values)
}

#[test]
#[trace("TC-264", "FR-037-AC-1", "FR-037-CON-1")]
fn tc_264_valid_commit_and_tag_are_mapped_to_the_principal_in_the_file() {
    let fixture = fixture();
    let id = signed_commit(&fixture, "a", "signed by a");
    signed_tag(&fixture, "a", "v-valid");
    let commit = status(&fixture, &id, "commit");
    assert_eq!(commit["status"], "signed_valid");
    assert_eq!(commit["signer"]["mapping"], "mapped");
    assert_eq!(commit["signer"]["principal"], PRINCIPAL);
    assert_eq!(commit["key_fingerprint"], fixture.fingerprint("a"));
    let tag = status(&fixture, "v-valid", "tag");
    assert_eq!(tag["status"], "signed_valid");
    assert_eq!(tag["signer"]["mapping"], "mapped");
    assert_eq!(tag["signer"]["principal"], PRINCIPAL);
    assert!(tag.get("key_fingerprint").is_none());
}

#[test]
#[trace("TC-265", "FR-037-AC-2", "FR-037-CON-1")]
fn tc_265_a_key_the_file_does_not_name_is_unmapped_without_a_principal() {
    let fixture = fixture();
    // The committer identity matches the principal; it must not be used.
    let repo = fixture.repo();
    let public = fixture.public("b");
    git(
        &repo,
        Some(public.to_str().expect("utf-8")),
        &[
            "-c",
            &format!("user.email={PRINCIPAL}"),
            "commit",
            "-q",
            "-S",
            "--allow-empty",
            "-m",
            "signed by b",
        ],
    );
    let id = git(&repo, None, &["rev-parse", "HEAD"]).trim().to_owned();
    let commit = status(&fixture, &id, "commit");
    assert_eq!(commit["status"], "signed_untrusted");
    assert_eq!(commit["signer"]["mapping"], "unmapped");
    assert!(commit["signer"].get("principal").is_none());
    signed_tag(&fixture, "b", "v-untrusted");
    let tag = status(&fixture, "v-untrusted", "tag");
    assert_eq!(tag["status"], "signed_untrusted");
    assert_eq!(tag["signer"]["mapping"], "unmapped");
    assert!(tag["signer"].get("principal").is_none());
    let bad_or_unsigned = plain_commit(&fixture, "plain");
    assert!(
        status(&fixture, &bad_or_unsigned, "commit")
            .get("signer")
            .is_none()
    );
}

/// Runs the command on another thread and fails the test if it hangs.
fn run_guarded(body: Vec<u8>, path: Option<PathBuf>) -> Output {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(run_with(&body, path.as_deref()));
    });
    receiver
        .recv_timeout(std::time::Duration::from_secs(30))
        .expect("the command must not hang")
}

#[test]
#[trace("TC-266", "FR-037-AC-3")]
fn tc_266_an_invalid_allowed_signers_file_is_refused_before_git_starts() {
    let fixture = fixture();
    signed_commit(&fixture, "a", "signed by a");
    let marker = fixture.root().join("git-was-started");
    let shim = fixture.root().join("shim-bin");
    fs::create_dir(&shim).expect("shim dir");
    let script = shim.join("git");
    fs::write(
        &script,
        format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()),
    )
    .expect("shim");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("mode");
    let oversized = fixture.root().join("oversized");
    fs::write(&oversized, vec![b'#'; 1024 * 1024 + 1]).expect("oversized");
    let not_utf8 = fixture.root().join("not-utf8");
    fs::write(&not_utf8, [0xff, 0xfe, b'\n']).expect("not utf-8");
    let directory = fixture.root().join("a-directory");
    fs::create_dir(&directory).expect("directory");
    let missing = fixture.root().join("missing");
    let fifo = fixture.root().join("a-fifo");
    run_program(
        "mkfifo",
        fixture.root(),
        &[fifo.to_str().expect("utf-8")],
        None,
    );
    for path in [&missing, &directory, &oversized, &not_utf8, &fifo] {
        let mut body = request(&fixture, "HEAD", "commit");
        body["allowed_signers"] = json!(path.display().to_string());
        let output = run_guarded(body.to_string().into_bytes(), Some(shim.clone()));
        assert_eq!(output.status.code(), Some(2), "{path:?}: {output:?}");
        let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
        assert_eq!(error["protocol"], "engineering-assurance.error/v1");
        assert_eq!(error["capability"], "signature-verify");
        assert_eq!(error["code"], "signature_verify_allowed_signers_invalid");
        assert!(!output.stderr.is_empty());
    }
    assert!(!marker.exists(), "an invalid file must not start git");
    // A file of exactly 1 MiB is accepted.
    let limit = fixture.root().join("limit");
    let mut contents = format!("{PRINCIPAL} {}", public_line(&fixture, "a")).into_bytes();
    contents.resize(1024 * 1024, b'\n');
    fs::write(&limit, contents).expect("limit file");
    let mut body = request(&fixture, "HEAD", "commit");
    body["allowed_signers"] = json!(limit.display().to_string());
    assert_eq!(verdict(&body)["status"], "signed_valid");
}

#[test]
#[trace("TC-267", "FR-037-AC-4")]
fn tc_267_unparsable_lines_authorize_nothing_and_the_rest_still_grades() {
    let fixture = fixture();
    let id = signed_commit(&fixture, "a", "signed by a");
    set_allowed(&fixture, "this is not an allowed signers line\n");
    let only = status(&fixture, &id, "commit");
    assert_eq!(only["status"], "signed_untrusted");
    assert_eq!(only["signer"]["mapping"], "unmapped");
    set_allowed(
        &fixture,
        &format!(
            "this is not an allowed signers line\n{PRINCIPAL} {}",
            public_line(&fixture, "a")
        ),
    );
    let mixed = status(&fixture, &id, "commit");
    assert_eq!(mixed["status"], "signed_valid");
    assert_eq!(mixed["signer"]["principal"], PRINCIPAL);
}

#[test]
#[trace("TC-268", "FR-037-AC-9")]
fn tc_268_an_expired_entry_is_untrusted_and_unmapped() {
    let fixture = fixture();
    let id = signed_commit(&fixture, "a", "signed by a");
    signed_tag(&fixture, "a", "v-expired");
    set_allowed(
        &fixture,
        &format!(
            "{PRINCIPAL} valid-before=\"20200101\" {}",
            public_line(&fixture, "a")
        ),
    );
    for (revision, kind) in [(id.as_str(), "commit"), ("v-expired", "tag")] {
        let verdict = status(&fixture, revision, kind);
        assert_eq!(verdict["status"], "signed_untrusted", "{kind}");
        assert_eq!(verdict["signer"]["mapping"], "unmapped", "{kind}");
        assert!(verdict["signer"].get("principal").is_none(), "{kind}");
    }
}

#[test]
#[trace("TC-269", "FR-037-AC-5")]
fn tc_269_trailers_are_read_in_order_from_the_trailer_block_only() {
    let fixture = fixture();
    let one = signed_commit_with(&fixture, "a", &["subject", "Signed-off-meaning: reviewed"]);
    assert_eq!(
        meaning(&status(&fixture, &one, "commit")),
        ("present", vec!["reviewed"])
    );
    let two = signed_commit_with(
        &fixture,
        "a",
        &[
            "subject",
            "Signed-off-meaning: reviewed\nSigned-off-meaning: approved for release",
        ],
    );
    assert_eq!(
        meaning(&status(&fixture, &two, "commit")),
        ("present", vec!["reviewed", "approved for release"])
    );
    let divided = signed_commit_with(
        &fixture,
        "a",
        &["subject", "---", "Signed-off-meaning: after a divider line"],
    );
    assert_eq!(
        meaning(&status(&fixture, &divided, "commit")),
        ("present", vec!["after a divider line"])
    );
    let outside = signed_commit_with(
        &fixture,
        "a",
        &[
            "subject",
            "Signed-off-meaning: not a trailer",
            "closing text",
        ],
    );
    assert_eq!(
        meaning(&status(&fixture, &outside, "commit")),
        ("missing", vec![])
    );
}

#[test]
#[trace("TC-270", "FR-037-AC-6", "FR-037-CON-2")]
fn tc_270_a_missing_trailer_is_missing_and_an_empty_one_is_present() {
    let fixture = fixture();
    let none = signed_commit(&fixture, "a", "no trailer here");
    let verdict = status(&fixture, &none, "commit");
    assert_eq!(meaning(&verdict), ("missing", vec![]));
    assert!(verdict["signed_off_meaning"].get("values").is_none());
    let empty = signed_commit_with(&fixture, "a", &["subject", "Signed-off-meaning:"]);
    assert_eq!(
        meaning(&status(&fixture, &empty, "commit")),
        ("present", vec![""])
    );
}

#[test]
#[trace("TC-271", "FR-037-AC-7")]
fn tc_271_a_bad_and_an_unsigned_commit_report_the_trailer_and_keep_their_status() {
    let fixture = fixture();
    let good = signed_commit_with(&fixture, "a", &["subject", "Signed-off-meaning: reviewed"]);
    let altered = rewrite(&fixture, "commit", &good, |o| {
        o.replace("subject", "subjecu")
    });
    let bad = status(&fixture, &altered, "commit");
    assert_eq!(bad["status"], "signed_bad");
    assert_eq!(meaning(&bad), ("present", vec!["reviewed"]));
    assert!(bad.get("signer").is_none());
    let unsigned = plain_commit_with(&fixture, &["subject", "Signed-off-meaning: authored"]);
    let verdict = status(&fixture, &unsigned, "commit");
    assert_eq!(verdict["status"], "unsigned");
    assert_eq!(meaning(&verdict), ("present", vec!["authored"]));
}

fn plain_commit_with(fixture: &Fixture, messages: &[&str]) -> String {
    let repo = fixture.repo();
    let mut arguments = vec!["commit", "-q", "--allow-empty"];
    for message in messages {
        arguments.extend(["-m", message]);
    }
    git(&repo, None, &arguments);
    git(&repo, None, &["rev-parse", "HEAD"]).trim().to_owned()
}

#[test]
#[trace("TC-272", "FR-037-AC-8")]
fn tc_272_a_signed_tags_trailer_comes_from_the_tag_message() {
    let fixture = fixture();
    signed_commit_with(
        &fixture,
        "a",
        &["subject", "Signed-off-meaning: from commit"],
    );
    signed_tag_with(
        &fixture,
        "a",
        "v-meaning",
        "release\n\nSigned-off-meaning: from tag",
    );
    let tag = status(&fixture, "v-meaning", "tag");
    assert_eq!(tag["status"], "signed_valid");
    assert_eq!(meaning(&tag), ("present", vec!["from tag"]));
    signed_tag_with(&fixture, "a", "v-no-meaning", "release");
    assert_eq!(
        meaning(&status(&fixture, "v-no-meaning", "tag")),
        ("missing", vec![])
    );
}

#[test]
#[trace("TC-276", "FR-037-AC-6", "FR-037-CON-2")]
fn tc_276_repository_trailer_config_cannot_inject_or_hide_the_trailer() {
    let fixture = fixture();
    let repo = fixture.repo();
    let injected = signed_commit_with(&fixture, "a", &["subject", "Fake: injected"]);
    let real = signed_commit_with(&fixture, "a", &["subject", "Signed-off-meaning: real"]);
    git(
        &repo,
        None,
        &["config", "trailer.fake.key", "Signed-off-meaning"],
    );
    git(&repo, None, &["config", "core.commentChar", "S"]);
    assert_eq!(
        meaning(&status(&fixture, &injected, "commit")),
        ("missing", vec![])
    );
    assert_eq!(
        meaning(&status(&fixture, &real, "commit")),
        ("present", vec!["real"])
    );
}

#[test]
#[trace("TC-277", "FR-037-AC-6", "FR-037-CON-2")]
fn tc_277_a_temp_dir_inside_the_verified_repository_cannot_inject_the_trailer() {
    let fixture = fixture();
    let repo = fixture.repo();
    let injected = signed_commit_with(&fixture, "a", &["subject", "fake: injected"]);
    git(
        &repo,
        None,
        &["config", "trailer.fake.key", "Signed-off-meaning"],
    );
    let inside = repo.join("scratch");
    fs::create_dir(&inside).expect("scratch dir");
    let body = request(&fixture, &injected, "commit");
    let output = run_with_temp(body.to_string().as_bytes(), None, Some(&inside));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let verdict: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(verdict["status"], "signed_valid");
    assert_eq!(meaning(&verdict), ("missing", vec![]));
}
