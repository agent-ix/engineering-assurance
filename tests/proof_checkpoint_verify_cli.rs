// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary tests for the FR-080 exact-object proof checkpoint verifier.

use std::{
    fmt::Write as _,
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const CERTIFICATE_IDENTITY: &str = "alice@example.test";
const OIDC_ISSUER: &str = concat!("https:", "//issuer.example.test");
const GITSIGN_STAND_IN: &str = r#"#!/bin/sh
d=${0%/*}
identity_any=0
issuer_any=0
for a in "$@"; do
  case "$a" in
    --certificate-identity-regexp) identity_any=1 ;;
    --certificate-oidc-issuer-regexp) issuer_any=1 ;;
  esac
done
if [ "$identity_any" -eq 1 ] && [ "$issuer_any" -eq 1 ]; then exit 0; fi
if [ -e "$d/mode_identity" ] && [ "$identity_any" -eq 1 ] && [ "$issuer_any" -eq 0 ]; then exit 0; fi
if [ -e "$d/mode_issuer" ] && [ "$identity_any" -eq 0 ] && [ "$issuer_any" -eq 1 ]; then exit 0; fi
if [ ! -e "$d/mode_identity" ] && [ ! -e "$d/mode_issuer" ] && [ "$identity_any" -eq 0 ] && [ "$issuer_any" -eq 0 ]; then exit 0; fi
exit 1
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
    fn allowed(&self) -> PathBuf {
        self.root().join("allowed_signers")
    }
    fn key(&self) -> PathBuf {
        self.root().join("key")
    }
}

fn run(program: &str, cwd: &Path, args: &[&str], input: Option<&[u8]>) -> Output {
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("process starts");
    if let Some(input) = input {
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(input)
            .expect("write input");
    }
    child.wait_with_output().expect("process exits")
}

fn git(fixture: &Fixture, args: &[&str]) -> String {
    git_input(fixture, args, None)
}

fn git_input(fixture: &Fixture, args: &[&str], input: Option<&[u8]>) -> String {
    let output = run("git", &fixture.repo(), args, input);
    assert!(output.status.success(), "git {args:?}: {output:?}");
    String::from_utf8(output.stdout)
        .expect("git output is UTF-8")
        .trim()
        .to_owned()
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = Fixture { dir };
    fs::create_dir(fixture.repo()).expect("repository directory");
    fs::write(fixture.allowed(), "").expect("allowed-signers file");
    let key = fixture.key();
    let key_text = key.to_str().expect("key path UTF-8");
    let output = run(
        "ssh-keygen",
        fixture.root(),
        &[
            "-q",
            "-t",
            "ed25519",
            "-N",
            "",
            "-C",
            "fictional",
            "-f",
            key_text,
        ],
        None,
    );
    assert!(output.status.success(), "ssh-keygen: {output:?}");
    let public = fs::read_to_string(key.with_extension("pub")).expect("public key");
    fs::write(fixture.allowed(), format!("alice@example.test {public}")).expect("trust file");
    git(&fixture, &["init", "-q", "-b", "main"]);
    git(
        &fixture,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.test",
            "commit",
            "--allow-empty",
            "-q",
            "-m",
            "base",
        ],
    );
    fixture
}

fn signed_tag(fixture: &Fixture, name: &str, message: &str) -> String {
    let key = fixture.key();
    let key_text = key.to_str().expect("key path UTF-8");
    let message_path = fixture.root().join("tag-message");
    fs::write(&message_path, message).expect("tag message");
    let message_text = message_path.to_str().expect("message path UTF-8");
    git(
        fixture,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.test",
            "-c",
            "gpg.format=ssh",
            "-c",
            &format!("user.signingkey={key_text}"),
            "tag",
            "-s",
            "-F",
            message_text,
            name,
        ],
    );
    git(fixture, &["rev-parse", &format!("refs/tags/{name}")])
}

fn cms_tag(fixture: &Fixture, name: &str) -> String {
    let commit = git(fixture, &["rev-parse", "HEAD"]);
    let tag = format!(
        "object {commit}\ntype commit\ntag {name}\ntagger Test <test@example.test> 1 +0000\n\n{}-----BEGIN SIGNED MESSAGE-----\nAAAA\n-----END SIGNED MESSAGE-----\n",
        checkpoint_message(&[])
    );
    let id = git_input(
        fixture,
        &["hash-object", "-t", "tag", "-w", "--stdin"],
        Some(tag.as_bytes()),
    );
    git(fixture, &["update-ref", &format!("refs/tags/{name}"), &id]);
    id
}

fn gitsign_fixture() -> (Fixture, PathBuf) {
    let fixture = fixture();
    let registry = json!({
        "protocol":"engineering-assurance.gitsign-identity-registry/v1",
        "entries":[{"certificate_identity":CERTIFICATE_IDENTITY,"certificate_oidc_issuer":OIDC_ISSUER,"principal":"Alice Example"}]
    });
    fs::write(fixture.repo().join("registry.json"), registry.to_string()).expect("registry");
    git(&fixture, &["add", "registry.json"]);
    git(
        &fixture,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.test",
            "commit",
            "-q",
            "-m",
            "registry",
        ],
    );
    let bin = fixture.root().join("bin");
    fs::create_dir(&bin).expect("gitsign directory");
    fs::write(bin.join("gitsign"), GITSIGN_STAND_IN).expect("gitsign stand-in");
    fs::set_permissions(bin.join("gitsign"), fs::Permissions::from_mode(0o755))
        .expect("executable");
    cms_tag(&fixture, "cms-proof");
    (fixture, bin.join("gitsign"))
}

fn with_gitsign(mut request: Value) -> Value {
    request["gitsign"] =
        json!({"certificate_identity":CERTIFICATE_IDENTITY,"certificate_oidc_issuer":OIDC_ISSUER});
    request["identity_registry"] = json!("main:registry.json");
    request
}

fn checkpoint_message(overrides: &[(&str, &str)]) -> String {
    let mut fields = [
        ("Proof-Checkpoint-Purpose", "proof-integrity-statement/v1"),
        ("Proof-Statement-Digest", DIGEST),
        ("Proof-Record-Id", "record-42"),
        ("Proof-Record-Revision", "3"),
        ("Proof-Candidate-Revision", "candidate-9"),
        ("Proof-Id", "proof-17"),
        ("Proof-Attestation-Id", "attestation-5"),
    ];
    for (key, value) in overrides {
        if let Some(field) = fields.iter_mut().find(|field| field.0 == *key) {
            field.1 = value;
        }
    }
    let mut trailers = String::new();
    for (key, value) in &fields {
        writeln!(trailers, "{key}: {value}").expect("write trailer to string");
    }
    format!("Proof integrity checkpoint.\n\n{trailers}")
}

fn request(fixture: &Fixture, tag: &str, object_id: &str) -> Value {
    json!({
        "protocol":"engineering-assurance.proof-checkpoint-verify-request/v1",
        "repository":fixture.repo(),
        "tag":tag,
        "expected":{
            "purpose":"proof-integrity-statement/v1",
            "statement_digest":DIGEST,
            "record_id":"record-42",
            "record_revision":3,
            "candidate_revision":"candidate-9",
            "proof_id":"proof-17",
            "attestation_id":"attestation-5",
            "tag_object_id":object_id,
        },
        "allowed_signers":fixture.allowed(),
    })
}

fn run_request(request: &Value) -> Output {
    run_request_with_path(request, None)
}

fn run_request_with_path(request: &Value, path: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"));
    command
        .args(["proof-checkpoint-verify", "--request", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = path {
        command.env("PATH", path);
    }
    let mut child = command.spawn().expect("CLI starts");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(request.to_string().as_bytes())
        .expect("request");
    child.wait_with_output().expect("CLI exits")
}

fn result(output: Output) -> Value {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = String::from_utf8(output.stdout).expect("JSON output");
    assert_eq!(text.matches('\n').count(), 1, "one result line: {text:?}");
    let value: Value = serde_json::from_str(&text).expect("result JSON");
    assert_eq!(
        value["protocol"],
        "engineering-assurance.proof-checkpoint-verify-result/v1"
    );
    value
}

#[test]
#[trace(
    "TC-1022",
    "FR-080-AC-1",
    "FR-080-CON-2",
    "FR-080-CON-3",
    "FR-080-AC-8"
)]
fn tc_1022_a_matching_signed_tag_verifies_the_exact_statement_scope_and_object() {
    let fixture = fixture();
    let object_id = signed_tag(&fixture, "proof", &checkpoint_message(&[]));
    let output = result(run_request(&request(&fixture, "proof", &object_id)));
    assert_eq!(output["status"], "verified");
    assert_eq!(output["tag_object_id"], object_id);
    assert_eq!(output["checkpoint"]["statement_digest"]["status"], "match");
    assert_eq!(output["signer"]["principal"], "alice@example.test");
    assert_eq!(output["trust"]["allowed_signers"], json!(fixture.allowed()));
    for forbidden in [
        "approved",
        "role_authorization",
        "proof_valid",
        "qualification",
    ] {
        assert!(
            output.get(forbidden).is_none(),
            "unexpected claim {forbidden}"
        );
    }
}

#[test]
#[trace("TC-1024", "TC-1025", "FR-080-AC-3", "FR-080-AC-4")]
fn tc_1024_changed_or_missing_checkpoint_fields_never_verify() {
    for (name, message, field) in [
        (
            "digest",
            checkpoint_message(&[(
                "Proof-Statement-Digest",
                "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            )]),
            "statement_digest",
        ),
        (
            "scope",
            checkpoint_message(&[("Proof-Record-Id", "record-other")]),
            "record_id",
        ),
        (
            "purpose",
            checkpoint_message(&[("Proof-Checkpoint-Purpose", "other-purpose")]),
            "purpose",
        ),
    ] {
        let fixture = fixture();
        let object_id = signed_tag(&fixture, name, &message);
        let output = result(run_request(&request(&fixture, name, &object_id)));
        assert_eq!(output["status"], "wrong_checkpoint", "{name}");
        assert_eq!(output["checkpoint"][field]["status"], "mismatch", "{name}");
        assert!(output["signer"].is_null());
    }
}

#[test]
#[trace("TC-1024", "TC-1025", "FR-080-AC-3", "FR-080-AC-4")]
fn tc_1024_absent_and_repeated_trailers_are_classified_without_verifying() {
    for (name, message, field, expected_status) in [
        (
            "absent",
            checkpoint_message(&[]).replace(&format!("Proof-Statement-Digest: {DIGEST}\n"), ""),
            "statement_digest",
            "missing",
        ),
        (
            "repeated",
            format!(
                "{}Proof-Statement-Digest: {DIGEST}\n",
                checkpoint_message(&[])
            ),
            "statement_digest",
            "repeated",
        ),
    ] {
        let fixture = fixture();
        let object_id = signed_tag(&fixture, name, &message);
        let output = result(run_request(&request(&fixture, name, &object_id)));
        assert_eq!(output["status"], "wrong_checkpoint", "{name}");
        assert_eq!(
            output["checkpoint"][field]["status"], expected_status,
            "{name}"
        );
        assert!(output["signer"].is_null());
    }
}

#[test]
#[trace("TC-1026", "TC-1031", "FR-080-AC-5", "FR-080-AC-9")]
fn tc_1026_untrusted_bad_unsigned_and_unavailable_verifier_never_verify() {
    let fixture = fixture();
    let untrusted_id = signed_tag(&fixture, "untrusted", &checkpoint_message(&[]));
    fs::write(fixture.allowed(), "").expect("remove trusted signer mapping");
    let untrusted = result(run_request(&request(&fixture, "untrusted", &untrusted_id)));
    assert_eq!(untrusted["status"], "untrusted_signer");
    assert!(untrusted["signer"].is_null());

    let mut bad_object = git(&fixture, &["cat-file", "tag", &untrusted_id]);
    bad_object = bad_object.replace(
        "Proof-Record-Id: record-42",
        "Proof-Record-Id: record-altered",
    );
    let bad_id = git_input(
        &fixture,
        &["hash-object", "-t", "tag", "-w", "--stdin"],
        Some(bad_object.as_bytes()),
    );
    git(&fixture, &["update-ref", "refs/tags/bad", &bad_id]);
    fs::write(
        fixture.allowed(),
        format!(
            "alice@example.test {}\n",
            fs::read_to_string(fixture.key().with_extension("pub")).expect("public key")
        ),
    )
    .expect("restore trust");
    let bad = result(run_request(&request(&fixture, "bad", &bad_id)));
    assert_eq!(bad["status"], "signed_bad");

    fs::write(fixture.allowed(), "alice@example.test PLACEHOLDER\n")
        .expect("restore malformed trust");
    let unsigned_name = "unsigned";
    let message_path = fixture.root().join("unsigned-message");
    fs::write(&message_path, checkpoint_message(&[])).expect("message");
    git(
        &fixture,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.test",
            "tag",
            "-a",
            "-F",
            message_path.to_str().expect("path"),
            unsigned_name,
        ],
    );
    let unsigned_id = git(&fixture, &["rev-parse", "refs/tags/unsigned"]);
    let unsigned = result(run_request(&request(&fixture, unsigned_name, &unsigned_id)));
    assert_eq!(unsigned["status"], "unsigned");

    let signed_name = "unavailable";
    let signed_id = signed_tag(&fixture, signed_name, &checkpoint_message(&[]));
    fs::write(
        fixture.allowed(),
        format!(
            "alice@example.test {}\n",
            fs::read_to_string(fixture.key().with_extension("pub")).expect("public key")
        ),
    )
    .expect("restore trust");
    let bin = fixture.root().join("git-only");
    fs::create_dir(&bin).expect("isolated PATH directory");
    let git_path = std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|directory| directory.join("git"))
        .find(|path| path.is_file())
        .expect("git on PATH");
    std::os::unix::fs::symlink(git_path, bin.join("git")).expect("git symlink");
    let unavailable = result(run_request_with_path(
        &request(&fixture, signed_name, &signed_id),
        Some(&bin),
    ));
    assert_eq!(unavailable["status"], "unverifiable");
    assert!(unavailable["signer"].is_null());
}

#[test]
#[trace("TC-1023", "TC-1026", "FR-080-AC-2", "FR-080-AC-5")]
fn tc_1023_gitsign_trust_maps_the_identity_and_refuses_identity_or_issuer_mismatch() {
    let (fixture, executable) = gitsign_fixture();
    let object_id = git(&fixture, &["rev-parse", "refs/tags/cms-proof"]);
    let original_path = std::env::var_os("PATH").expect("PATH");
    let bin = executable.parent().expect("bin directory");
    let path = std::env::join_paths(
        std::iter::once(bin.to_path_buf()).chain(std::env::split_paths(&original_path)),
    )
    .expect("joined PATH");
    let matched = with_gitsign(request(&fixture, "cms-proof", &object_id));
    let output = result(run_request_with_path(&matched, Some(Path::new(&path))));
    assert_eq!(output["status"], "verified");
    assert_eq!(output["signer"]["principal"], "Alice Example");
    assert_eq!(
        output["signature"]["certificate"]["oidc_issuer"],
        OIDC_ISSUER
    );
    assert_eq!(
        output["trust"]["identity_registry_commit"],
        git(&fixture, &["rev-parse", "main"])
    );

    let no_gitsign = fixture.root().join("git-only");
    fs::create_dir(&no_gitsign).expect("git-only directory");
    let real_git = std::env::split_paths(&original_path)
        .map(|directory| directory.join("git"))
        .find(|path| path.is_file())
        .expect("git on PATH");
    std::os::unix::fs::symlink(real_git, no_gitsign.join("git")).expect("git symlink");
    let unavailable = result(run_request_with_path(&matched, Some(&no_gitsign)));
    assert_eq!(unavailable["status"], "unverifiable");
    assert!(unavailable["signer"].is_null());

    fs::write(bin.join("mode_identity"), "").expect("identity mismatch mode");
    let identity_mismatch = with_gitsign(request(&fixture, "cms-proof", &object_id));
    let output = result(run_request_with_path(
        &identity_mismatch,
        Some(Path::new(&path)),
    ));
    assert_eq!(output["status"], "untrusted_signer");
    assert_eq!(output["signature"]["mismatch"], "identity");
    assert!(output["signer"].is_null());
    fs::remove_file(bin.join("mode_identity")).expect("clear identity mode");

    fs::write(bin.join("mode_issuer"), "").expect("issuer mismatch mode");
    let issuer_mismatch = with_gitsign(request(&fixture, "cms-proof", &object_id));
    let output = result(run_request_with_path(
        &issuer_mismatch,
        Some(Path::new(&path)),
    ));
    assert_eq!(output["status"], "untrusted_signer");
    assert_eq!(output["signature"]["mismatch"], "issuer");
    assert!(output["signer"].is_null());
}

#[test]
#[trace("TC-1030", "TC-1031", "FR-080-AC-9", "FR-080-CON-3")]
fn tc_1030_same_name_retarget_is_refused_before_signature_verification() {
    let fixture = fixture();
    let pinned = signed_tag(&fixture, "proof", &checkpoint_message(&[]));
    git(
        &fixture,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.test",
            "-c",
            "gpg.format=ssh",
            "-c",
            &format!("user.signingkey={}", fixture.key().display()),
            "tag",
            "-s",
            "-f",
            "-m",
            &checkpoint_message(&[]).replace(
                "Proof integrity checkpoint.",
                "Retargeted proof checkpoint.",
            ),
            "proof",
        ],
    );
    let replacement = git(&fixture, &["rev-parse", "refs/tags/proof"]);
    assert_ne!(
        pinned, replacement,
        "the replacement must be another immutable object"
    );
    let marker = fixture.root().join("verifier-started");
    let path = verifier_monitor_path(&fixture, &marker);
    assert_retarget_refused(&fixture, &pinned, &marker, Some(&path));
    exercise_unsigned_retarget(&fixture, &pinned, &marker, &path);
    exercise_untrusted_retarget(&fixture, &pinned, &marker, &path);
    exercise_malformed_signed_retarget(&fixture, &pinned, &marker, &path);
}

fn verifier_monitor_path(fixture: &Fixture, marker: &Path) -> PathBuf {
    let bin = fixture.root().join("no-verifier");
    fs::create_dir(&bin).expect("verifier directory");
    let real_ssh_keygen = std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|directory| directory.join("ssh-keygen"))
        .find(|path| path.is_file())
        .expect("ssh-keygen on PATH");
    fs::write(
        bin.join("ssh-keygen"),
        format!(
            "#!/bin/sh\ntouch {}\nexec {} \"$@\"\n",
            marker.display(),
            real_ssh_keygen.display()
        ),
    )
    .expect("ssh-keygen monitor");
    fs::set_permissions(bin.join("ssh-keygen"), fs::Permissions::from_mode(0o755))
        .expect("executable");
    let original_path = std::env::var_os("PATH").expect("PATH");
    std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&original_path)))
        .expect("joined PATH")
        .into()
}

fn assert_retarget_refused(fixture: &Fixture, pinned: &str, marker: &Path, path: Option<&Path>) {
    let current = git(fixture, &["rev-parse", "refs/tags/proof"]);
    assert_ne!(current, pinned, "each control points at another object");
    let output = result(run_request_with_path(
        &request(fixture, "proof", pinned),
        path,
    ));
    assert_eq!(output["status"], "wrong_checkpoint");
    assert_eq!(output["tag_object_id"], current);
    assert_eq!(output["checkpoint"]["tag_object_id"], "mismatch");
    assert_eq!(
        output["signature"]["reason"],
        "expected_tag_object_id_mismatch"
    );
    assert_eq!(output["signature"]["delegated"], false);
    assert!(output["signer"].is_null());
    assert!(
        !marker.exists(),
        "mismatched object must not start ssh-keygen"
    );
}

fn exercise_unsigned_retarget(fixture: &Fixture, pinned: &str, marker: &Path, path: &Path) {
    let message_path = fixture.root().join("retarget-message");
    let retarget_message = checkpoint_message(&[]).replace(
        "Proof integrity checkpoint.",
        "Retargeted proof checkpoint.",
    );
    fs::write(&message_path, retarget_message).expect("retarget message");
    git(
        fixture,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.test",
            "tag",
            "-a",
            "-f",
            "-F",
            message_path.to_str().expect("message path"),
            "proof",
        ],
    );
    assert_retarget_refused(fixture, pinned, marker, Some(path));
}

fn exercise_untrusted_retarget(fixture: &Fixture, pinned: &str, marker: &Path, path: &Path) {
    let trusted = fs::read_to_string(fixture.key().with_extension("pub")).expect("public key");
    let message_path = fixture.root().join("retarget-message");
    let sign_current_tag = || {
        git(
            fixture,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.test",
                "-c",
                "gpg.format=ssh",
                "-c",
                &format!("user.signingkey={}", fixture.key().display()),
                "tag",
                "-s",
                "-f",
                "-F",
                message_path.to_str().expect("message path"),
                "proof",
            ],
        );
    };
    sign_current_tag();
    fs::write(fixture.allowed(), "").expect("remove signer trust");
    assert_retarget_refused(fixture, pinned, marker, Some(path));
    fs::write(fixture.allowed(), format!("alice@example.test {trusted}"))
        .expect("restore signer trust");
}

fn exercise_malformed_signed_retarget(fixture: &Fixture, pinned: &str, marker: &Path, path: &Path) {
    let message_path = fixture.root().join("retarget-message");
    let sign_current_tag = || {
        git(
            fixture,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.test",
                "-c",
                "gpg.format=ssh",
                "-c",
                &format!("user.signingkey={}", fixture.key().display()),
                "tag",
                "-s",
                "-f",
                "-F",
                message_path.to_str().expect("message path"),
                "proof",
            ],
        );
    };
    sign_current_tag();
    let signed = git(fixture, &["rev-parse", "refs/tags/proof"]);
    let mut bad_object = git(fixture, &["cat-file", "tag", &signed]);
    bad_object = bad_object.replace("Proof-Record-Id: record-42", "Proof-Record-Id: changed");
    let bad_id = git_input(
        fixture,
        &["hash-object", "-t", "tag", "-w", "--stdin"],
        Some(bad_object.as_bytes()),
    );
    git(fixture, &["update-ref", "refs/tags/proof", &bad_id]);
    assert_retarget_refused(fixture, pinned, marker, Some(path));

    let no_verifier = fixture.root().join("git-only");
    fs::create_dir(&no_verifier).expect("git-only directory");
    let git_path = std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|directory| directory.join("git"))
        .find(|path| path.is_file())
        .expect("git on PATH");
    std::os::unix::fs::symlink(git_path, no_verifier.join("git")).expect("git symlink");
    sign_current_tag();
    assert_retarget_refused(fixture, pinned, marker, Some(&no_verifier));
}

#[test]
#[trace("TC-1028", "FR-080-AC-7", "FR-080-CON-1")]
fn tc_1028_a_ref_move_after_object_load_keeps_verification_on_the_loaded_object() {
    let fixture = fixture();
    let pinned = signed_tag(&fixture, "proof", &checkpoint_message(&[]));
    let replacement = signed_tag(&fixture, "replacement", &checkpoint_message(&[]));
    let bin = fixture.root().join("bin");
    fs::create_dir(&bin).expect("wrapper directory");
    let real_git = std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|directory| directory.join("git"))
        .find(|path| path.is_file())
        .expect("git on PATH");
    fs::write(
        bin.join("git"),
        format!(
            "#!/bin/sh\n{} \"$@\"\nresult=$?\ncase \" $* \" in *\" cat-file tag \"*) {} update-ref refs/tags/proof {} ;; esac\nexit $result\n",
            real_git.display(), real_git.display(), replacement
        ),
    )
    .expect("git wrapper");
    fs::set_permissions(bin.join("git"), fs::Permissions::from_mode(0o755)).expect("executable");
    let original_path = std::env::var_os("PATH").expect("PATH");
    let path =
        std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&original_path)))
            .expect("joined PATH");
    let output = result(run_request_with_path(
        &request(&fixture, "proof", &pinned),
        Some(Path::new(&path)),
    ));
    assert_eq!(output["status"], "verified");
    assert_eq!(output["tag_object_id"], pinned);
    assert_eq!(
        git(&fixture, &["rev-parse", "refs/tags/proof"]),
        replacement
    );
}

#[test]
#[trace("TC-1032", "FR-080-AC-10", "FR-080-AC-6")]
fn tc_1032_malformed_expected_object_id_is_rejected_before_git() {
    let fixture = fixture();
    let mut bad_object_request = request(&fixture, "missing-tag", &"A".repeat(40));
    bad_object_request["expected"]["tag_object_id"] = json!("ABC");
    let output = run_request(&bad_object_request);
    assert_eq!(output.status.code(), Some(2));
    let error: Value = serde_json::from_slice(&output.stdout).expect("machine error stdout");
    assert_eq!(error["code"], "proof_checkpoint_verify_request_invalid");

    let mut missing_digest_request = request(&fixture, "missing-tag", &"a".repeat(40));
    missing_digest_request["expected"]
        .as_object_mut()
        .expect("expected object")
        .remove("statement_digest");
    let output = run_request(&missing_digest_request);
    assert_eq!(output.status.code(), Some(2));
    let error: Value = serde_json::from_slice(&output.stdout).expect("machine error stdout");
    assert_eq!(error["code"], "proof_checkpoint_verify_request_invalid");

    let object_id = signed_tag(&fixture, "missing-trust", &checkpoint_message(&[]));
    let mut request = request(&fixture, "missing-trust", &object_id);
    request["allowed_signers"] = json!(fixture.root().join("no-such-allowed-signers"));
    let output = run_request(&request);
    assert_eq!(output.status.code(), Some(2));
    let error: Value = serde_json::from_slice(&output.stdout).expect("machine error stdout");
    assert_eq!(error["code"], "proof_checkpoint_verify_request_invalid");
}
