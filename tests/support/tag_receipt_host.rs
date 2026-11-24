// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Adapter tests for tag receipts over real git repositories, with the signer,
//! the visibility lookup and the Sigstore verifier replaced by doubles
//! (FR-081, FR-082).

use std::{
    cell::Cell,
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use engineering_assurance::{
    attestation_signing::{
        AuthenticatedWorkflowClaims, BundleVerifier, VerificationFailure, WorkflowIdentityDocument,
        WorkflowIdentityPolicy,
    },
    receipt::ReceiptBundleSet,
    record_sink::{
        config::Visibility,
        leak::{LookupFailed, VisibilityLookup},
        sink::{Signer, SigningFailed},
    },
    tag_receipt::{FindingStatus, TagFinding},
};
use ix_trace_rs::trace;
use serde_json::json;

use super::*;

pub(crate) const SOURCE_REPOSITORY: &str = "fictional/project";
pub(crate) const SOURCE_WORKFLOW: &str = ".github/workflows/tag-receipt-push.yml";

pub(crate) fn git(repo: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
        .args(arguments)
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_DIR")
        .output()
        .expect("git must start");
    assert!(output.status.success(), "{arguments:?}: {output:?}");
    String::from_utf8(output.stdout).expect("utf-8")
}

/// A repository with one commit on `main`.
pub(crate) fn repository(root: &Path) -> PathBuf {
    let repo = root.join("repo");
    std::fs::create_dir(&repo).expect("repo dir");
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "base"]);
    repo
}

pub(crate) fn tag_message(repo: &Path, name: &str, message: &str, date: Option<&str>) {
    let mut command = Command::new("git");
    command
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.invalid",
            "tag",
            "-a",
            "-m",
            message,
            name,
        ])
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_DIR");
    if let Some(date) = date {
        command.env("GIT_COMMITTER_DATE", date);
    }
    assert!(command.output().expect("git").status.success());
}

pub(crate) fn rev(repo: &Path, name: &str) -> String {
    git(repo, &["rev-parse", name]).trim().to_owned()
}

/// An unsigned annotated tag object written as is, so a message the porcelain
/// refuses (a NUL byte) can be tested.
fn forged_tag(repo: &Path, name: &str, message: &[u8]) {
    let commit = rev(repo, "HEAD");
    let mut object = format!("object {commit}\ntype commit\ntag {name}\n\n").into_bytes();
    object.extend_from_slice(message);
    let mut child = Command::new("git")
        .args(["hash-object", "-t", "tag", "-w", "--literally", "--stdin"])
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("git");
    std::io::Write::write_all(&mut child.stdin.take().expect("stdin"), &object).expect("write");
    let output = child.wait_with_output().expect("output");
    let id = String::from_utf8(output.stdout).expect("utf-8");
    git(repo, &["update-ref", &format!("refs/tags/{name}"), id.trim()]);
}

pub(crate) fn bundle_of(payload: &[u8]) -> Value {
    let envelope = sigstore_types::DsseEnvelope::new(
        "application/vnd.in-toto+json",
        sigstore_types::PayloadBytes::from(payload.to_vec()),
        sigstore_types::DsseSignature::new(
            sigstore_types::SignatureBytes::from(vec![0]),
            sigstore_types::KeyId::default(),
        ),
    );
    let bundle = sigstore_types::Bundle::new(
        sigstore_types::MediaType::Bundle0_3,
        sigstore_types::VerificationMaterial::new(
            sigstore_types::VerificationMaterialContent::PublicKey(
                sigstore_types::PublicKeyIdentifier::new("fictional"),
            ),
        ),
        sigstore_types::SignatureContent::DsseEnvelope(envelope),
    );
    serde_json::to_value(bundle).expect("bundle encodes")
}

/// A signer double that records which path was taken.
#[derive(Default)]
struct FakeSigner {
    fails: bool,
    private_calls: Cell<usize>,
    public_calls: Cell<usize>,
}

impl FakeSigner {
    fn bundle(&self, payload: &[u8]) -> Result<String, SigningFailed> {
        if self.fails {
            return Err(SigningFailed);
        }
        Ok(bundle_of(payload).to_string())
    }
}

impl Signer for FakeSigner {
    fn sign_for_public_log(
        &self,
        _proof: &engineering_assurance::record_sink::leak::PublicPathProof<'_>,
    ) -> Result<String, SigningFailed> {
        Err(SigningFailed)
    }

    fn sign_statement_for_public_log(
        &self,
        proof: &engineering_assurance::record_sink::leak::PublicStatementProof<'_>,
    ) -> Result<String, SigningFailed> {
        self.public_calls.set(self.public_calls.get() + 1);
        self.bundle(proof.statement_bytes())
    }

    fn sign_without_public_log(&self, canonical: &[u8]) -> Result<String, SigningFailed> {
        self.private_calls.set(self.private_calls.get() + 1);
        self.bundle(canonical)
    }

    fn signed_payload(&self, _bundle: &str) -> Result<Vec<u8>, SigningFailed> {
        Err(SigningFailed)
    }
}

struct AllPublic;

impl VisibilityLookup for AllPublic {
    fn repo_visibility(&self, _repo: &str) -> Result<Visibility, LookupFailed> {
        Ok(Visibility::Public)
    }
}

const POLICY: &str = r#"{"protocol":"engineering-assurance.receipt-policy/v1","tag_receipts":{"prefixes":["approve-"]}}"#;

fn protected(policy: Option<&str>, private: bool) -> Protected {
    Protected::from_policy_bytes(policy.map(str::as_bytes), private, Vec::new())
        .expect("valid policy")
}

fn hex(text: &str) -> String {
    text.bytes().map(|byte| format!("{byte:02x}")).collect()
}

fn record_request(repo: &Path, tag: &str, object: &str, pushed_at_unix: i64) -> RecordRequest {
    serde_json::from_value(json!({
        "protocol": RECORD_REQUEST_PROTOCOL,
        "repository": repo.display().to_string(),
        "github_repository": SOURCE_REPOSITORY,
        "base_branch": "main",
        "tag_hex": hex(tag),
        "tag_object": object,
        "pushed_at_unix": pushed_at_unix,
    }))
    .expect("request")
}

fn record(
    repo: &Path,
    tag: &str,
    protected: &Protected,
    signer: &FakeSigner,
) -> Result<Value, TagReceiptHostError> {
    let request = record_request(repo, tag, &rev(repo, &format!("refs/tags/{tag}")), 1_700_000_000);
    record_with(&request, protected, signer, &AllPublic)
        .map(|line| serde_json::from_slice(&line).expect("json line"))
}

/// The decoded statement inside a result's bundle.
fn statement_of(result: &Value) -> Value {
    let bundle = serde_json::to_string(&result["bundle"]).expect("bundle");
    let parsed = sigstore_types::Bundle::from_json(&bundle).expect("bundle parses");
    let sigstore_types::SignatureContent::DsseEnvelope(envelope) = parsed.content else {
        panic!("a DSSE envelope");
    };
    serde_json::from_slice(envelope.payload.as_ref()).expect("statement")
}

#[test]
#[trace("TC-1114", "FR-081-AC-1")]
fn a_prefix_tag_yields_one_signed_statement_for_the_pushed_tag_object() {
    let directory = tempfile::tempdir().expect("tempdir");
    let repo = repository(directory.path());
    tag_message(
        &repo,
        "approve-1",
        "Approve\n\nApproved-Record-Id: rec-1",
        None,
    );
    let signer = FakeSigner::default();
    let result = record(&repo, "approve-1", &protected(Some(POLICY), true), &signer).expect("receipt");
    assert_eq!(result["status"], "signed");
    assert_eq!(result["commit"], rev(&repo, "HEAD"));
    let statement = statement_of(&result);
    assert_eq!(statement["subject"][0]["name"], "refs/tags/approve-1");
    assert_eq!(
        statement["subject"][0]["digest"],
        json!({"gitTag": rev(&repo, "refs/tags/approve-1")})
    );
    assert_ne!(
        rev(&repo, "refs/tags/approve-1"),
        rev(&repo, "HEAD"),
        "the subject is the tag object, not the commit"
    );
    let members: BTreeSet<String> = statement["predicate"]
        .as_object()
        .expect("predicate")
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        members,
        BTreeSet::from(
            [
                "repository",
                "ref",
                "tag_object",
                "commit",
                "pushed_at",
                "trailers_readable",
                "record_ids"
            ]
            .map(str::to_owned)
        )
    );
    assert_eq!(statement["predicate"]["repository"], SOURCE_REPOSITORY);
    assert_eq!(statement["predicate"]["commit"], rev(&repo, "HEAD"));
    // A lightweight tag's object is the commit itself.
    git(&repo, &["tag", "approve-light"]);
    let light = record(&repo, "approve-light", &protected(Some(POLICY), true), &signer)
        .expect("lightweight receipt");
    assert_eq!(statement_of(&light)["predicate"]["tag_object"], rev(&repo, "HEAD"));
    assert_eq!(statement_of(&light)["predicate"]["record_ids"], json!([]));
}

#[test]
#[trace("TC-1115", "FR-081-AC-2")]
fn no_prefix_no_member_a_bad_member_and_a_non_utf8_name_end_as_specified() {
    let directory = tempfile::tempdir().expect("tempdir");
    let repo = repository(directory.path());
    tag_message(&repo, "approve-1", "m", None);
    tag_message(&repo, "other-1", "m", None);
    let signer = FakeSigner::default();
    // A name that starts with no prefix ends successfully with no receipt.
    let other = record(&repo, "other-1", &protected(Some(POLICY), true), &signer).expect("skipped");
    assert_eq!(other["status"], "skipped");
    assert!(other.get("bundle").is_none());
    // A policy with no `tag_receipts` member, and no policy file at all, write nothing.
    for policy in [
        Some(r#"{"protocol":"engineering-assurance.receipt-policy/v1"}"#),
        None,
    ] {
        let result =
            record(&repo, "approve-1", &protected(policy, true), &signer).expect("skipped");
        assert_eq!(result["status"], "skipped");
    }
    assert_eq!(signer.private_calls.get() + signer.public_calls.get(), 0);
    // A malformed member fails the run.
    let bad = Protected::from_policy_bytes(
        Some(br#"{"tag_receipts":{"prefixes":[],"extra":1}}"#),
        true,
        Vec::new(),
    );
    let error = bad.err().expect("malformed member");
    assert_eq!(error.code(), "tag_receipt_policy_invalid");
    // A tag name that is not valid UTF-8 fails the run.
    let mut request = record_request(&repo, "approve-1", &rev(&repo, "refs/tags/approve-1"), 1);
    request.tag_hex = "617070726f76652dff".to_owned();
    let error = record_with(&request, &protected(Some(POLICY), true), &signer, &AllPublic)
        .expect_err("not UTF-8");
    assert_eq!(error.code(), "tag_receipt_ref_name_invalid");
    // A tag object that is not the pushed one is an input mismatch.
    let request = record_request(&repo, "approve-1", &"0".repeat(40), 1);
    let error = record_with(&request, &protected(Some(POLICY), true), &signer, &AllPublic)
        .expect_err("mismatch");
    assert_eq!(error.code(), "tag_receipt_input_mismatch");
}

#[test]
#[trace("TC-1116", "FR-081-AC-3")]
fn pushed_at_is_the_platform_push_time_never_the_tagger_date() {
    let directory = tempfile::tempdir().expect("tempdir");
    let repo = repository(directory.path());
    tag_message(&repo, "approve-1", "m", Some("2099-01-01T00:00:00+0000"));
    let signer = FakeSigner::default();
    let result = record(&repo, "approve-1", &protected(Some(POLICY), true), &signer).expect("receipt");
    assert_eq!(
        statement_of(&result)["predicate"]["pushed_at"],
        "2023-11-14T22:13:20Z"
    );
}

#[test]
#[trace("TC-1117", "FR-081-AC-4")]
fn record_ids_are_copied_in_message_order_and_unreadable_trailers_carry_none() {
    let directory = tempfile::tempdir().expect("tempdir");
    let repo = repository(directory.path());
    let signer = FakeSigner::default();
    let policy = protected(Some(POLICY), true);
    let ids_of = |name: &str| {
        let predicate = statement_of(&record(&repo, name, &policy, &signer).expect("receipt"))
            ["predicate"]
            .clone();
        (
            predicate["trailers_readable"].clone(),
            predicate["record_ids"].clone(),
        )
    };
    tag_message(
        &repo,
        "approve-two",
        "Approve\n\nApproved-Record-Id: rec-b\nApproved-Record-Id: rec-a",
        None,
    );
    assert_eq!(ids_of("approve-two"), (json!(true), json!(["rec-b", "rec-a"])));
    tag_message(&repo, "approve-none", "Approve with no trailer", None);
    assert_eq!(ids_of("approve-none"), (json!(true), json!([])));
    forged_tag(&repo, "approve-nul", b"Approved-Record-Id: rec-1\n\0\n");
    assert_eq!(ids_of("approve-nul"), (json!(false), json!([])));
    let many = (0..65)
        .map(|index| format!("Approved-Record-Id: r{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    tag_message(&repo, "approve-many", &format!("Approve\n\n{many}"), None);
    assert_eq!(ids_of("approve-many"), (json!(false), json!([])));
    let long = "r".repeat(257);
    tag_message(
        &repo,
        "approve-long",
        &format!("Approve\n\nApproved-Record-Id: {long}"),
        None,
    );
    assert_eq!(ids_of("approve-long"), (json!(false), json!([])));
}

#[test]
#[trace("TC-1118", "FR-081-AC-10", "FR-081-AC-11")]
fn a_public_write_is_leak_checked_and_a_private_one_never_uses_the_public_log() {
    let directory = tempfile::tempdir().expect("tempdir");
    let repo = repository(directory.path());
    tag_message(
        &repo,
        "approve-secret",
        "Approve\n\nApproved-Record-Id: PRIV-9",
        None,
    );
    tag_message(
        &repo,
        "approve-fine",
        "Approve\n\nApproved-Record-Id: 123e4567-e89b-12d3-a456-426614174000",
        None,
    );
    // A public repository whose trailer references a private ticket team.
    let signer = FakeSigner::default();
    let public = protected(Some(POLICY), false);
    let error = record(&repo, "approve-secret", &public, &signer).expect_err("refused");
    assert_eq!(error.code(), "tag_receipt_leak_check_refused");
    assert_eq!(signer.public_calls.get() + signer.private_calls.get(), 0);
    // A public ticket team passes the check; the public path signs.
    let teams = Protected::from_policy_bytes(Some(POLICY.as_bytes()), false, vec!["PRIV".to_owned()])
        .expect("policy");
    record(&repo, "approve-secret", &teams, &signer).expect("a public team passes");
    assert_eq!(signer.public_calls.get(), 1);
    record(&repo, "approve-fine", &public, &signer).expect("an opaque id passes");
    assert_eq!(signer.public_calls.get(), 2);
    assert_eq!(signer.private_calls.get(), 0);

    // A private repository takes the private path whatever the trailer says,
    // and an unavailable signer writes nothing and never touches the public log.
    let private = protected(Some(POLICY), true);
    let working = FakeSigner::default();
    record(&repo, "approve-secret", &private, &working).expect("private receipt");
    assert_eq!((working.private_calls.get(), working.public_calls.get()), (1, 0));
    let unavailable = FakeSigner {
        fails: true,
        ..FakeSigner::default()
    };
    let error = record(&repo, "approve-fine", &private, &unavailable).expect_err("unavailable");
    assert_eq!(error.code(), "tag_receipt_signing_unavailable");
    assert_eq!(unavailable.public_calls.get(), 0);
    assert!(error.code().starts_with("tag_receipt_"));
}

// ---------------------------------------------------------------------------
// The notes ref: storing and pushing
// ---------------------------------------------------------------------------

fn statement_bundle(reference: &str, tag_object: &str, commit: &str) -> Value {
    let statement = engineering_assurance::tag_receipt::TagReceiptStatement::new(
        SOURCE_REPOSITORY,
        reference,
        tag_object,
        commit,
        "2026-11-20T10:00:00Z",
        engineering_assurance::tag_receipt::RecordTrailers::from_values(vec!["rec-1".to_owned()]),
    );
    bundle_of(&statement.canonical_bytes().expect("canonical"))
}

fn note_request(clone: &Path, commit: &str, bundle: &Value) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "protocol": NOTE_REQUEST_PROTOCOL,
        "repository": clone.display().to_string(),
        "remote": "origin",
        "commit": commit,
        "bundle": bundle,
    }))
    .expect("request")
}

/// A bare remote and a clone of it holding one pushed commit.
fn remote_and_clone(root: &Path, name: &str) -> (PathBuf, PathBuf) {
    let remote = root.join("remote.git");
    if !remote.exists() {
        std::fs::create_dir(&remote).expect("remote dir");
        git(&remote, &["init", "-q", "--bare", "-b", "main"]);
    }
    let clone = root.join(name);
    git(root, &["clone", "-q", remote.to_str().expect("utf-8"), name]);
    if rev_opt(&clone, "HEAD").is_none() {
        git(&clone, &["commit", "-q", "--allow-empty", "-m", "base"]);
        git(&clone, &["push", "-q", "origin", "main"]);
    }
    (remote, clone)
}

fn rev_opt(repo: &Path, name: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", name])
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_DIR")
        .output()
        .expect("git");
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).expect("utf-8").trim().to_owned())
}

fn remote_note(remote: &Path, commit: &str) -> Vec<String> {
    let output = Command::new("git")
        .args(["notes", "--ref", TAG_RECEIPT_NOTES_REF, "show", commit])
        .current_dir(remote)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_DIR")
        .output()
        .expect("git");
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8(output.stdout)
        .expect("utf-8")
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
#[trace("TC-1119", "FR-081-AC-5", "FR-081-AC-7")]
fn the_same_object_is_stored_once_and_a_new_object_adds_an_entry_to_the_tag_notes_only() {
    let directory = tempfile::tempdir().expect("tempdir");
    let (remote, clone) = remote_and_clone(directory.path(), "work");
    let commit = rev(&clone, "HEAD");
    let first = statement_bundle("refs/tags/approve-1", &"a".repeat(40), &commit);
    let stored = execute_note(&note_request(&clone, &commit, &first)).expect("stored");
    assert_eq!(serde_json::from_slice::<Value>(&stored).expect("json")["status"], "stored");
    let one = remote_note(&remote, &commit);
    assert_eq!(one.len(), 1);
    // Deleted and pushed again at the same object id: one entry.
    let again = statement_bundle("refs/tags/approve-1", &"a".repeat(40), &commit);
    let result = execute_note(&note_request(&clone, &commit, &again)).expect("again");
    assert_eq!(
        serde_json::from_slice::<Value>(&result).expect("json")["status"],
        "already_recorded"
    );
    assert_eq!(remote_note(&remote, &commit), one);
    // Re-signed: a different object id adds a second entry, the first unchanged.
    let resigned = statement_bundle("refs/tags/approve-1", &"b".repeat(40), &commit);
    execute_note(&note_request(&clone, &commit, &resigned)).expect("second entry");
    let two = remote_note(&remote, &commit);
    assert_eq!(two.len(), 2);
    assert_eq!(two[0], one[0]);
    // A tag receipt never appears under the change receipt notes ref, and the
    // change receipt verifier does not accept a tag receipt entry.
    assert!(rev_opt(&remote, "refs/notes/assurance-receipts").is_none());
    assert!(ReceiptBundleSet::parse_note(two.join("\n").as_bytes()).is_err());
    // A bundle for another commit is refused before anything is written.
    let other = statement_bundle("refs/tags/approve-3", &"c".repeat(40), &"d".repeat(40));
    let error = execute_note(&note_request(&clone, &commit, &other)).expect_err("other commit");
    assert_eq!(error.code(), "tag_receipt_request_invalid");
    assert_eq!(remote_note(&remote, &commit), two);
}

#[test]
#[trace("TC-1120", "FR-081-AC-6")]
fn an_oversized_note_and_six_rejected_pushes_write_nothing() {
    let directory = tempfile::tempdir().expect("tempdir");
    let (remote, clone) = remote_and_clone(directory.path(), "work");
    let commit = rev(&clone, "HEAD");

    // Six pushes in a row rejected: the run fails and the remote is unchanged.
    let hook = remote.join("hooks/pre-receive");
    std::fs::write(
        &hook,
        "#!/bin/sh\necho rejected >> \"$GIT_DIR/rejections\"\nexit 1\n",
    )
    .expect("hook");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    let bundle = statement_bundle("refs/tags/approve-1", &"a".repeat(40), &commit);
    let error = execute_note(&note_request(&clone, &commit, &bundle)).expect_err("rejected");
    assert_eq!(error.code(), "tag_receipt_note_conflict");
    let rejections = std::fs::read_to_string(remote.join("rejections")).expect("count");
    assert_eq!(rejections.lines().count(), 6);
    assert!(rev_opt(&remote, TAG_RECEIPT_NOTES_REF).is_none(), "no partial note");
    assert!(
        rev_opt(&clone, TAG_RECEIPT_NOTES_REF).is_none(),
        "no partial local note"
    );
    std::fs::remove_file(&hook).expect("remove hook");

    // A note that would exceed 1 MiB is refused and nothing is written.
    let filler: String = (0..100_000).map(|index| format!("{{\"filler\":{index}}}\n")).collect();
    let mut child = Command::new("git")
        .args(["notes", "--ref", TAG_RECEIPT_NOTES_REF, "add", "-f", "-F", "-", &commit])
        .current_dir(&clone)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_DIR")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .stdin(Stdio::piped())
        .spawn()
        .expect("git");
    std::io::Write::write_all(&mut child.stdin.take().expect("stdin"), filler.as_bytes())
        .expect("write");
    assert!(child.wait().expect("wait").success());
    git(&clone, &["push", "-q", "origin", TAG_RECEIPT_NOTES_REF]);
    let before = rev_opt(&remote, TAG_RECEIPT_NOTES_REF);
    assert!(before.is_some());
    let error = execute_note(&note_request(&clone, &commit, &bundle)).expect_err("too large");
    assert_eq!(error.code(), "tag_receipt_note_too_large");
    assert_eq!(rev_opt(&remote, TAG_RECEIPT_NOTES_REF), before);
}

#[test]
#[trace("TC-1121", "FR-081-AC-6")]
fn two_runs_for_two_tags_on_one_commit_both_end_with_their_entries() {
    let directory = tempfile::tempdir().expect("tempdir");
    let (remote, first) = remote_and_clone(directory.path(), "work-1");
    let (_, second) = remote_and_clone(directory.path(), "work-2");
    let commit = rev(&first, "HEAD");
    // The second run holds an earlier, unpushed note of its own: a push that
    // needs the merge of the remote's note with the local one.
    let local = statement_bundle("refs/tags/approve-0", &"0".repeat(40), &commit);
    execute_note(&note_request(&second, &commit, &local)).expect("local entry");
    git(&remote, &["update-ref", "-d", TAG_RECEIPT_NOTES_REF]);
    let threads: Vec<_> = [(first, "approve-1", 'a'), (second, "approve-2", 'b')]
        .into_iter()
        .map(|(clone, name, object)| {
            let commit = commit.clone();
            std::thread::spawn(move || {
                let bundle = statement_bundle(
                    &format!("refs/tags/{name}"),
                    &object.to_string().repeat(40),
                    &commit,
                );
                execute_note(&note_request(&clone, &commit, &bundle)).map(|_| ())
            })
        })
        .collect();
    for thread in threads {
        thread.join().expect("thread").expect("stored");
    }
    let lines = remote_note(&remote, &commit);
    let joined = lines.join("\n");
    assert_eq!(lines.len(), 3, "{joined}");
    let refs: BTreeSet<String> = lines
        .iter()
        .filter_map(|line| {
            let bundle: Value = serde_json::from_str(line).ok()?;
            let statement = engineering_assurance::tag_receipt::parse_statement(&bundle)?;
            Some(statement.predicate.reference)
        })
        .collect();
    assert_eq!(
        refs,
        BTreeSet::from(
            ["refs/tags/approve-0", "refs/tags/approve-1", "refs/tags/approve-2"].map(str::to_owned)
        )
    );
}

// ---------------------------------------------------------------------------
// Reading: verification and the comparison over a real repository
// ---------------------------------------------------------------------------

/// Certificate claims for the statement a bundle carries, as the identity
/// policy of `identities` expects them; the signature of the bundles in
/// `invalid` does not verify.
#[derive(Default)]
pub(crate) struct FakeVerifier {
    pub(crate) invalid: BTreeSet<String>,
    /// Bundles that verify cryptographically but were signed by an unlisted identity.
    pub(crate) untrusted: BTreeSet<String>,
}

impl BundleVerifier for FakeVerifier {
    fn verify(
        &self,
        _payload: &[u8],
        bundle_json: &str,
        _policy: &WorkflowIdentityPolicy,
    ) -> Result<(), VerificationFailure> {
        if self.invalid.contains(bundle_json) {
            Err(VerificationFailure::InvalidSignature)
        } else if self.untrusted.contains(bundle_json) {
            Err(VerificationFailure::UntrustedSigner)
        } else {
            Ok(())
        }
    }

    fn verify_with_claims(
        &self,
        payload: &[u8],
        bundle_json: &str,
        policy: &WorkflowIdentityPolicy,
    ) -> Result<Option<AuthenticatedWorkflowClaims>, VerificationFailure> {
        self.verify(payload, bundle_json, policy)?;
        let statement: Value = serde_json::from_slice(payload).unwrap_or(Value::Null);
        let reference = statement["predicate"]["ref"]
            .as_str()
            .unwrap_or("refs/heads/main")
            .to_owned();
        Ok(Some(AuthenticatedWorkflowClaims {
            source_repository_uri: Some(format!("{}{}github.com/{SOURCE_REPOSITORY}", "https:", "//")),
            build_config_uri: Some(format!(
                "{}{}github.com/{SOURCE_REPOSITORY}/{SOURCE_WORKFLOW}@{reference}",
                "https:", "//"
            )),
            source_repository_ref: Some(reference),
            build_trigger: Some("push".to_owned()),
        }))
    }
}

pub(crate) struct FakeSource(pub(crate) FakeVerifier);

impl VerifierSource for FakeSource {
    fn attempts(&self) -> Result<Vec<Attempt<'_>>, TagReceiptHostError> {
        Ok(vec![Attempt {
            verifier: &self.0,
            transparency: engineering_assurance::attestation_signing::TransparencyRequirement::PublicLog,
        }])
    }
}

pub(crate) fn identities() -> WorkflowIdentityDocument {
    WorkflowIdentityDocument::parse(identities_json().to_string().as_bytes()).expect("identities")
}

pub(crate) fn identities_json() -> Value {
    json!({
            "protocol": "engineering-assurance.workflow-identities/v1",
            "identities": [{
                "role": "receipt_writer",
                "issuer": concat!("https:", "//token.actions.githubusercontent.com"),
                "repository": "fictional/engineering-assurance",
                "workflow_path": ".github/workflows/tag-receipt.yml",
                "ref": "refs/heads/main",
                "source_repository": SOURCE_REPOSITORY,
                "source_workflow_path": SOURCE_WORKFLOW,
                "event": "push",
                "source_ref_prefix": "refs/tags/approve-"
            }]
        })
}

pub(crate) fn policy(accepted: &Value) -> TagReceiptPolicy {
    TagReceiptPolicy::parse(&json!({"prefixes": ["approve-"], "accepted_tags": accepted}))
        .expect("policy")
}

/// Writes one receipt line for `reference` at `tag_object` on `commit`; the
/// statement is exactly what the workflow would sign.
pub(crate) fn write_receipt(
    repo: &Path,
    reference: &str,
    tag_object: &str,
    commit: &str,
    record_ids: &[&str],
    readable: bool,
) -> Value {
    let trailers = if readable {
        engineering_assurance::tag_receipt::RecordTrailers::from_values(
            record_ids.iter().map(|id| (*id).to_owned()).collect(),
        )
    } else {
        engineering_assurance::tag_receipt::RecordTrailers::unreadable()
    };
    let statement = engineering_assurance::tag_receipt::TagReceiptStatement::new(
        SOURCE_REPOSITORY,
        reference,
        tag_object,
        commit,
        "2026-11-20T10:00:00Z",
        trailers,
    );
    let bundle = bundle_of(&statement.canonical_bytes().expect("canonical"));
    add_line(repo, commit, &bundle.to_string());
    bundle
}

pub(crate) fn add_line(repo: &Path, commit: &str, line: &str) {
    let existing = Command::new("git")
        .args(["notes", "--ref", TAG_RECEIPT_NOTES_REF, "show", commit])
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_DIR")
        .output()
        .expect("git");
    let mut text = if existing.status.success() {
        String::from_utf8(existing.stdout).expect("utf-8")
    } else {
        String::new()
    };
    text.push_str(line);
    text.push('\n');
    let mut child = Command::new("git")
        .args(["notes", "--ref", TAG_RECEIPT_NOTES_REF, "add", "-f", "-F", "-", commit])
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_DIR")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .stdin(Stdio::piped())
        .spawn()
        .expect("git");
    std::io::Write::write_all(&mut child.stdin.take().expect("stdin"), text.as_bytes())
        .expect("write");
    assert!(child.wait().expect("wait").success());
}

fn findings_of(repo: &Path, accepted: &Value, invalid: &[&Value]) -> Vec<(FindingStatus, bool)> {
    let verifier = FakeVerifier {
        invalid: invalid.iter().map(|bundle| bundle.to_string()).collect(),
        ..FakeVerifier::default()
    };
    repository_findings(repo, &identities(), &policy(accepted), &FakeSource(verifier))
        .expect("findings")
        .iter()
        .map(|judged| (judged.finding.status(), judged.accepted.is_some()))
        .collect()
}

#[test]
#[trace("TC-1122", "FR-082-AC-1", "FR-082-AC-2", "FR-082-AC-3", "FR-082-AC-6")]
fn the_repository_comparison_finds_a_deleted_changed_and_unreceipted_tag() {
    let directory = tempfile::tempdir().expect("tempdir");
    let repo = repository(directory.path());
    let commit = rev(&repo, "HEAD");
    tag_message(&repo, "approve-1", "m", None);
    // The notes ref being absent is an empty receipt set: the tag is unreceipted.
    assert_eq!(
        findings_of(&repo, &json!([]), &[]),
        [(FindingStatus::TagNoReceipt, false)]
    );
    let object = rev(&repo, "refs/tags/approve-1");
    write_receipt(&repo, "refs/tags/approve-1", &object, &commit, &[], true);
    // A tag that gained its receipt is not listed by the next run.
    assert!(findings_of(&repo, &json!([]), &[]).is_empty());
    // Deleted.
    git(&repo, &["tag", "-d", "approve-1"]);
    assert_eq!(
        findings_of(&repo, &json!([]), &[]),
        [(FindingStatus::MissingTag, false)]
    );
    // Pushed again at the identical object id: no finding.
    git(&repo, &["update-ref", "refs/tags/approve-1", &object]);
    assert!(findings_of(&repo, &json!([]), &[]).is_empty());
    // Re-created as a different tag object: the earlier receipt is changed and
    // the new object is unreceipted.
    tag_message_force(&repo, "approve-1", "another message");
    let changed = repository_findings(
        &repo,
        &identities(),
        &policy(&json!([])),
        &FakeSource(FakeVerifier::default()),
    )
    .expect("findings");
    assert_eq!(changed.len(), 2);
    let TagFinding::MissingTag {
        current_object: Some(current),
        ..
    } = &changed[0].finding
    else {
        panic!("a changed tag: {:?}", changed[0]);
    };
    assert_eq!(*current, rev(&repo, "refs/tags/approve-1"));
    // A tag with a non-matching prefix is never listed, whatever its age.
    tag_message(&repo, "other-1", "m", None);
    assert_eq!(findings_of(&repo, &json!([]), &[]).len(), 2);
}

fn tag_message_force(repo: &Path, name: &str, message: &str) {
    git(repo, &["tag", "-f", "-a", "-m", message, name]);
}

#[test]
#[trace("TC-1123", "FR-082-AC-4")]
fn a_bad_signature_an_unparsable_line_and_an_unlisted_signer_are_invalid_not_missing() {
    let directory = tempfile::tempdir().expect("tempdir");
    let repo = repository(directory.path());
    let commit = rev(&repo, "HEAD");
    let tampered = write_receipt(&repo, "refs/tags/approve-1", &"a".repeat(40), &commit, &[], true);
    add_line(&repo, &commit, "this line is not a bundle");
    // The tag the tampered receipt names is absent, yet it is not judged missing.
    let verifier = FakeVerifier {
        invalid: BTreeSet::from([tampered.to_string()]),
        ..FakeVerifier::default()
    };
    let judged = repository_findings(
        &repo,
        &identities(),
        &policy(&json!([])),
        &FakeSource(verifier),
    )
    .expect("findings");
    let reasons: Vec<_> = judged
        .iter()
        .map(|item| match &item.finding {
            TagFinding::InvalidReceipt(invalid) => invalid.reason,
            other => panic!("not invalid: {other:?}"),
        })
        .collect();
    assert_eq!(reasons.len(), 2);
    assert!(reasons.contains(&engineering_assurance::tag_receipt::InvalidReason::SignatureInvalid));
    assert!(reasons.contains(&engineering_assurance::tag_receipt::InvalidReason::PredicateInvalid));
    // A policy that lists no tag receipt identity leaves the signer untrusted.
    let none = WorkflowIdentityDocument::parse(
        json!({
            "protocol": "engineering-assurance.workflow-identities/v1",
            "identities": [{
                "role": "statement_writer",
                "issuer": concat!("https:", "//token.actions.githubusercontent.com"),
                "repository": "fictional/engineering-assurance",
                "workflow_path": ".github/workflows/review-gate.yml",
                "ref": "refs/heads/main"
            }]
        })
        .to_string()
        .as_bytes(),
    )
    .expect("identities");
    let judged = repository_findings(
        &repo,
        &none,
        &policy(&json!([])),
        &FakeSource(FakeVerifier::default()),
    )
    .expect("findings");
    assert!(judged.iter().any(|item| matches!(
        &item.finding,
        TagFinding::InvalidReceipt(invalid)
            if invalid.reason == engineering_assurance::tag_receipt::InvalidReason::UntrustedSigner
    )));
    // Notes under the change receipt ref are not read as tag receipts.
    assert!(
        judged
            .iter()
            .all(|item| !matches!(item.finding, TagFinding::MissingTag { .. }))
    );
}
