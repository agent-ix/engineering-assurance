// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary tests for `release-verify` (FR-047, FR-051) on real
//! repositories whose release tags are signed with fictional ed25519 keys
//! generated inside each test.

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

const REQUEST_PROTOCOL: &str = "engineering-assurance.release-verify-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.release-verify-result/v1";
const ALICE: &str = "alice@example.test";
const BASELINE_PATH: &str = "baseline.md";
const RECORD_PATH: &str = "release/REL-1.md";
const SNAPSHOT_PATH: &str = "release/snapshot.json";
const ITEMS: [(&str, &str); 3] = [
    ("spec/a.md", "spec_artifact"),
    ("soup/s.md", "soup_record"),
    ("evidence/e.txt", "evidence"),
];

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
    String::from_utf8(run_program("git", repo, &refs, None))
        .expect("utf-8")
        .trim_end()
        .to_owned()
}

/// An empty repository, keys `a`, `b` and `m`, and an allowed-signers file
/// mapping `a` to alice and `m` to mallory (key `b` is unlisted).
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    for key in ["a", "b", "m"] {
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
    let public = |key: &str| fs::read_to_string(root.join(format!("{key}.pub"))).expect("key");
    fs::write(
        root.join("allowed_signers"),
        format!(
            "{ALICE} {}mallory@example.test {}",
            public("a"),
            public("m")
        ),
    )
    .expect("allowed signers");
    let repo = root.join("repo");
    fs::create_dir(&repo).expect("repo dir");
    git(&repo, None, &["init", "-q", "-b", "main"]);
    Fixture { dir }
}

/// Writes `files` into the working tree, stages exactly those paths and
/// commits them on the current branch. Returns the new commit.
fn commit_files(fixture: &Fixture, files: &[(&str, &str)], message: &str) -> String {
    let repo = fixture.repo();
    for (path, content) in files {
        let target = repo.join(path);
        fs::create_dir_all(target.parent().expect("parent")).expect("directories");
        fs::write(target, content).expect("file");
        git(&repo, None, &["add", "--", path]);
    }
    git(&repo, None, &["commit", "-q", "-m", message]);
    head(fixture)
}

fn head(fixture: &Fixture) -> String {
    git(&fixture.repo(), None, &["rev-parse", "HEAD"])
}

fn baseline_document(items: &[(&str, &str)]) -> String {
    let mut text = String::from(
        "---\nid: BL-1\ntitle: Baseline one\ntype: Baseline\nname: Baseline one\nitems:\n",
    );
    for (path, kind) in items {
        let _ = write!(text, "  - path: {path}\n    kind: {kind}\n");
    }
    text.push_str("---\n\nThe baseline.\n");
    text
}

/// Commits the three items and the Baseline file; returns the baseline commit.
fn commit_baseline(fixture: &Fixture, extra: &[(&str, &str)]) -> String {
    let document = baseline_document(&ITEMS);
    let mut files = vec![
        ("spec/a.md", "spec a\n"),
        ("soup/s.md", "soup s\n"),
        ("evidence/e.txt", "evidence e\n"),
        (BASELINE_PATH, document.as_str()),
    ];
    files.extend_from_slice(extra);
    commit_files(fixture, &files, "baseline")
}

fn record_text(baseline_commit: &str, snapshot_path: &str, owner: &str) -> String {
    format!(
        "---\nid: REL-1\ntitle: Release one\ntype: Release\nname: Release one\n\
         baseline:\n  path: {BASELINE_PATH}\n  commit: \"{baseline_commit}\"\n\
         snapshot_path: {snapshot_path}\ndecision_owner: {owner}\n\
         accepted_anomalies:\n  - minor cosmetic defect\n---\n\nThe release record.\n"
    )
}

/// What a release commit holds.
struct Release {
    record: String,
    snapshot_path: String,
    snapshot: Option<String>,
    extra: Vec<(String, String)>,
}

impl Release {
    fn standard(baseline: &str) -> Self {
        Self {
            record: record_text(baseline, SNAPSHOT_PATH, ALICE),
            snapshot_path: SNAPSHOT_PATH.to_owned(),
            snapshot: Some("{\"evidence\":\"snapshot\"}\n".to_owned()),
            extra: Vec::new(),
        }
    }

    /// Commits the release on the current branch; returns the commit.
    fn commit(&self, fixture: &Fixture) -> String {
        let mut files: Vec<(&str, &str)> = vec![(RECORD_PATH, self.record.as_str())];
        if let Some(snapshot) = &self.snapshot {
            files.push((self.snapshot_path.as_str(), snapshot.as_str()));
        }
        for (path, content) in &self.extra {
            files.push((path.as_str(), content.as_str()));
        }
        commit_files(fixture, &files, "release")
    }
}

/// The `git-blob:` digest of the record at `commit`.
fn digest_at(fixture: &Fixture, commit: &str) -> String {
    let id = git(
        &fixture.repo(),
        None,
        &["rev-parse", &format!("{commit}:{RECORD_PATH}")],
    );
    format!("git-blob:{id}")
}

fn message(digest: &str) -> String {
    format!(
        "Release one\n\nApproved-Record-Id: REL-1\nApproved-Record-Digest: {digest}\n\
         Signed-off-meaning: approved for release"
    )
}

fn sign(fixture: &Fixture, key: &str, name: &str, commit: &str, message: &str) {
    let public = fixture.public(key);
    git(
        &fixture.repo(),
        Some(public.to_str().expect("utf-8")),
        &["tag", "-s", "-m", message, name, commit],
    );
}

/// Baseline, release commit and a tag signed by alice: the valid release.
fn standard(fixture: &Fixture) -> (String, String) {
    let baseline = commit_baseline(fixture, &[]);
    let release = Release::standard(&baseline).commit(fixture);
    let digest = digest_at(fixture, &release);
    sign(fixture, "a", "v1", &release, &message(&digest));
    (baseline, release)
}

fn request(fixture: &Fixture, tag: &str) -> Value {
    json!({
        "protocol": REQUEST_PROTOCOL,
        "repository": fixture.repo().display().to_string(),
        "tag": tag,
        "record_path": RECORD_PATH,
        "allowed_signers": fixture.allowed().display().to_string(),
    })
}

fn run_command(subcommand: &str, body: &[u8], path: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"));
    command
        .args([subcommand, "--request", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = path {
        command.env("PATH", path);
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

fn result_on(request: &Value, path: Option<&Path>) -> Value {
    let output = run_command("release-verify", request.to_string().as_bytes(), path);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = std::str::from_utf8(&output.stdout).expect("utf-8");
    assert!(
        text.ends_with('\n') && text.matches('\n').count() == 1,
        "{text:?}"
    );
    let value: Value = serde_json::from_str(text).expect("json line");
    assert_eq!(value["protocol"], RESULT_PROTOCOL);
    value
}

fn verdict(fixture: &Fixture, tag: &str) -> Value {
    result_on(&request(fixture, tag), None)
}

fn reasons(result: &Value) -> Vec<String> {
    result["reasons"]
        .as_array()
        .expect("reasons")
        .iter()
        .map(|reason| reason.as_str().expect("reason text").to_owned())
        .collect()
}

/// Asserts `not_released` with exactly the listed reasons.
fn assert_not_released(result: &Value, expected: &[&str]) {
    assert_eq!(result["status"], "not_released", "{result}");
    assert_eq!(reasons(result), expected, "{result}");
    for member in ["release", "baseline", "approver", "meaning", "signed_at"] {
        assert!(result.get(member).is_none(), "{member}: {result}");
    }
}

/// Rewrites a tag object's bytes with `edit` and points the tag name at it.
fn rewrite_tag(fixture: &Fixture, name: &str, edit: impl Fn(&str) -> String) {
    let repo = fixture.repo();
    let object = git(&repo, None, &["cat-file", "tag", name]);
    let id = run_program(
        "git",
        &repo,
        &["hash-object", "-t", "tag", "-w", "--stdin"],
        Some(edit(&object).as_bytes()),
    );
    let id = String::from_utf8(id).expect("utf-8");
    git(
        &repo,
        None,
        &["update-ref", &format!("refs/tags/{name}"), id.trim()],
    );
}

#[test]
#[trace("TC-380", "FR-047-AC-1", "FR-051-AC-1")]
fn tc_380_a_signed_release_of_a_present_baseline_is_released() {
    let fixture = fixture();
    let (baseline, _) = standard(&fixture);
    let result = verdict(&fixture, "v1");
    assert_eq!(result["status"], "released", "{result}");
    assert_eq!(result["reasons"], json!([]));
    assert_eq!(result["approver"], ALICE);
    assert_eq!(result["meaning"], json!(["approved for release"]));
    assert_eq!(result["signed_at_source"], "tagger_date");
    assert!(
        result["signed_at"]
            .as_str()
            .is_some_and(|t| t.ends_with('Z'))
    );
    assert_eq!(
        result["release"],
        json!({"id": "REL-1", "name": "Release one", "snapshot_path": SNAPSHOT_PATH})
    );
    assert_eq!(result["baseline"]["name"], "Baseline one");
    assert_eq!(result["baseline"]["commit"], baseline);
    assert_eq!(
        result["baseline"]["items"],
        json!([
            {"path": "spec/a.md", "kind": "spec_artifact"},
            {"path": "soup/s.md", "kind": "soup_record"},
            {"path": "evidence/e.txt", "kind": "evidence"},
        ])
    );
    assert_eq!(
        result["accepted_anomalies"],
        json!(["minor cosmetic defect"])
    );
    assert_eq!(result["approval"]["status"], "approved");
    // The tag's tree is the baseline plus the record and the snapshot only.
    let changed = git(
        &fixture.repo(),
        None,
        &["diff-tree", "-r", "--name-only", &baseline, "v1^{commit}"],
    );
    let mut paths: Vec<&str> = changed.lines().collect();
    paths.sort_unstable();
    assert_eq!(paths, [RECORD_PATH, SNAPSHOT_PATH]);
}

#[test]
#[trace("TC-381", "FR-047-AC-2")]
fn tc_381_a_record_edited_after_the_message_was_composed_is_the_wrong_record() {
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[]);
    let first = Release::standard(&baseline);
    let first_commit = first.commit(&fixture);
    let stale = digest_at(&fixture, &first_commit);
    // The record is edited and recommitted; the tag message keeps the old id.
    git(&fixture.repo(), None, &["reset", "-q", "--hard", &baseline]);
    let mut edited = Release::standard(&baseline);
    edited
        .record
        .push_str("An edit after the message was composed.\n");
    let edited_commit = edited.commit(&fixture);
    sign(&fixture, "a", "v1", &edited_commit, &message(&stale));
    let result = verdict(&fixture, "v1");
    assert_eq!(result["status"], "not_released", "{result}");
    assert_eq!(reasons(&result), ["wrong_record"]);
    assert_eq!(result["approval"]["status"], "wrong_record");
    assert_eq!(result["approval"]["binding"]["record_digest"], "mismatch");
    assert!(result.get("release").is_none());
}

#[test]
#[trace("TC-382", "FR-047-AC-3")]
fn tc_382_a_mapped_signer_who_is_not_the_decision_owner_does_not_release() {
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[]);
    let release = Release::standard(&baseline).commit(&fixture);
    let digest = digest_at(&fixture, &release);
    sign(&fixture, "m", "v1", &release, &message(&digest));
    let result = verdict(&fixture, "v1");
    assert_eq!(result["status"], "not_released", "{result}");
    assert_eq!(reasons(&result), ["decision_owner_mismatch"]);
    assert_eq!(result["approval"]["approver"], "mallory@example.test");
    assert!(result.get("release").is_none());
}

#[test]
#[trace("TC-383", "FR-047-AC-4")]
fn tc_383_an_unsigned_annotated_tag_is_unsigned() {
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[]);
    let release = Release::standard(&baseline).commit(&fixture);
    let digest = digest_at(&fixture, &release);
    git(
        &fixture.repo(),
        None,
        &["tag", "-a", "-m", &message(&digest), "v1", &release],
    );
    let result = verdict(&fixture, "v1");
    assert_not_released(&result, &["unsigned"]);
    assert_eq!(result["approval"]["status"], "unsigned");
}

#[test]
#[trace("TC-384", "FR-047-AC-4")]
fn tc_384_a_lightweight_tag_is_unsigned() {
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[]);
    let release = Release::standard(&baseline).commit(&fixture);
    git(&fixture.repo(), None, &["tag", "v1", &release]);
    let result = verdict(&fixture, "v1");
    assert_not_released(&result, &["unsigned"]);
    assert_eq!(result["approval"]["status"], "unsigned");
}

#[test]
#[trace("TC-385", "FR-047-AC-4")]
fn tc_385_a_tag_edited_after_signing_is_signed_bad() {
    let fixture = fixture();
    standard(&fixture);
    rewrite_tag(&fixture, "v1", |object| {
        object.replace("approved for release", "approved for something else")
    });
    let result = verdict(&fixture, "v1");
    assert_not_released(&result, &["signed_bad"]);
    assert_eq!(result["approval"]["status"], "signed_bad");
}

#[test]
#[trace("TC-386", "FR-047-AC-4")]
fn tc_386_a_good_signature_by_an_unmapped_key_is_an_untrusted_signer() {
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[]);
    let release = Release::standard(&baseline).commit(&fixture);
    let digest = digest_at(&fixture, &release);
    sign(&fixture, "b", "v1", &release, &message(&digest));
    let result = verdict(&fixture, "v1");
    assert_not_released(&result, &["untrusted_signer"]);
    assert_eq!(result["approval"]["status"], "untrusted_signer");
}

#[test]
#[trace("TC-387", "FR-047-AC-4")]
fn tc_387_a_tag_without_a_signed_off_meaning_is_meaning_missing() {
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[]);
    let release = Release::standard(&baseline).commit(&fixture);
    let digest = digest_at(&fixture, &release);
    let without =
        format!("Release one\n\nApproved-Record-Id: REL-1\nApproved-Record-Digest: {digest}");
    sign(&fixture, "a", "v1", &release, &without);
    let result = verdict(&fixture, "v1");
    assert_not_released(&result, &["meaning_missing"]);
    assert_eq!(result["approval"]["status"], "meaning_missing");
}

#[test]
#[trace("TC-388", "FR-047-AC-5")]
fn tc_388_a_baseline_commit_not_in_the_repository_is_unknown() {
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[]);
    let absent = "1".repeat(baseline.len());
    let mut release = Release::standard(&baseline);
    release.record = record_text(&absent, SNAPSHOT_PATH, ALICE);
    let commit = release.commit(&fixture);
    let digest = digest_at(&fixture, &commit);
    sign(&fixture, "a", "v1", &commit, &message(&digest));
    let result = verdict(&fixture, "v1");
    // The release commit's parent cannot be compared with an unknown commit.
    assert_not_released(&result, &["baseline_commit_unknown"]);
    assert_eq!(result["approval"]["status"], "approved");
}

#[test]
#[trace("TC-389", "FR-047-AC-5", "FR-046-AC-4")]
fn tc_389_an_item_absent_at_the_baseline_commit_is_named_even_when_in_the_working_tree() {
    let fixture = fixture();
    let repo = fixture.repo();
    // The evidence file is listed but never committed: it exists only in the
    // working tree.
    let document = baseline_document(&ITEMS);
    commit_files(
        &fixture,
        &[
            ("spec/a.md", "spec a\n"),
            ("soup/s.md", "soup s\n"),
            (BASELINE_PATH, &document),
        ],
        "baseline",
    );
    fs::create_dir_all(repo.join("evidence")).expect("directory");
    fs::write(
        repo.join("evidence/e.txt"),
        "present only in the working tree\n",
    )
    .expect("file");
    let baseline = head(&fixture);
    let release = Release::standard(&baseline).commit(&fixture);
    let digest = digest_at(&fixture, &release);
    sign(&fixture, "a", "v1", &release, &message(&digest));
    let result = verdict(&fixture, "v1");
    assert_not_released(&result, &["baseline_item_absent"]);
    assert_eq!(result["absent_items"], json!(["evidence/e.txt"]));
}

#[test]
#[trace("TC-390", "FR-046-AC-4", "FR-046-AC-5")]
fn tc_390_an_item_modified_in_the_working_tree_is_still_present_and_the_tree_is_untouched() {
    let fixture = fixture();
    let repo = fixture.repo();
    standard(&fixture);
    fs::write(repo.join("spec/a.md"), "modified after the release\n").expect("modify");
    let status_before = git(&repo, None, &["status", "--porcelain=v2", "--branch"]);
    let refs_before = git(&repo, None, &["for-each-ref"]);
    let index_before = git(&repo, None, &["ls-files", "--stage"]);
    let result = verdict(&fixture, "v1");
    assert_eq!(result["status"], "released", "{result}");
    assert_eq!(
        fs::read_to_string(repo.join("spec/a.md")).expect("file"),
        "modified after the release\n"
    );
    assert_eq!(
        git(&repo, None, &["status", "--porcelain=v2", "--branch"]),
        status_before
    );
    assert_eq!(git(&repo, None, &["for-each-ref"]), refs_before);
    assert_eq!(git(&repo, None, &["ls-files", "--stage"]), index_before);
}

#[test]
#[trace("TC-391", "FR-047-AC-5", "FR-046-AC-6")]
fn tc_391_a_baseline_file_absent_at_the_baseline_commit_is_absent() {
    let fixture = fixture();
    let baseline = commit_files(&fixture, &[("spec/a.md", "spec a\n")], "no baseline file");
    let release = Release::standard(&baseline).commit(&fixture);
    let digest = digest_at(&fixture, &release);
    sign(&fixture, "a", "v1", &release, &message(&digest));
    let result = verdict(&fixture, "v1");
    assert_not_released(&result, &["baseline_file_absent"]);
    assert!(result.get("absent_items").is_none());
}

#[test]
#[trace("TC-392", "FR-047-AC-5", "FR-046-AC-6")]
fn tc_392_a_baseline_file_with_invalid_frontmatter_is_invalid() {
    let fixture = fixture();
    let invalid = "---\nid: BL-1\ntitle: Baseline one\ntype: Baseline\nname: Baseline one\n---\n";
    let baseline = commit_files(&fixture, &[(BASELINE_PATH, invalid)], "invalid baseline");
    let release = Release::standard(&baseline).commit(&fixture);
    let digest = digest_at(&fixture, &release);
    sign(&fixture, "a", "v1", &release, &message(&digest));
    let result = verdict(&fixture, "v1");
    assert_not_released(&result, &["baseline_file_invalid"]);
}

#[test]
#[trace("TC-393", "FR-046-AC-6", "FR-047-AC-5")]
fn tc_393_a_baseline_listing_one_path_twice_is_invalid() {
    let fixture = fixture();
    // The same path with two different kinds: the archetype's schema cannot
    // compare items by a member, so the resolution refuses it.
    let duplicated =
        baseline_document(&[("spec/a.md", "spec_artifact"), ("spec/a.md", "evidence")]);
    let baseline = commit_files(
        &fixture,
        &[("spec/a.md", "spec a\n"), (BASELINE_PATH, &duplicated)],
        "duplicate path",
    );
    let release = Release::standard(&baseline).commit(&fixture);
    let digest = digest_at(&fixture, &release);
    sign(&fixture, "a", "v1", &release, &message(&digest));
    let result = verdict(&fixture, "v1");
    assert_not_released(&result, &["baseline_file_invalid"]);
}

#[test]
#[trace("TC-394", "FR-046-AC-3", "FR-049-AC-1")]
fn tc_394_two_baselines_report_the_two_commit_hashes_git_gives() {
    let fixture = fixture();
    let (first_baseline, _) = standard(&fixture);
    let repo = fixture.repo();
    // A second release on a branch from the first baseline, with an item changed.
    git(
        &repo,
        None,
        &["checkout", "-q", "-b", "second", &first_baseline],
    );
    let second_baseline = commit_files(&fixture, &[("spec/a.md", "spec a, revised\n")], "revise");
    let mut release = Release::standard(&second_baseline);
    release.record = release.record.replace("REL-1", "REL-2");
    let second_release = {
        let files: Vec<(&str, &str)> = vec![
            ("release/REL-1.md", release.record.as_str()),
            (SNAPSHOT_PATH, "{}\n"),
        ];
        commit_files(&fixture, &files, "second release")
    };
    let digest = digest_at(&fixture, &second_release);
    sign(
        &fixture,
        "a",
        "v2",
        &second_release,
        &message(&digest).replace("REL-1", "REL-2"),
    );
    let first = verdict(&fixture, "v1");
    let second = verdict(&fixture, "v2");
    assert_eq!(first["status"], "released", "{first}");
    assert_eq!(second["status"], "released", "{second}");
    assert_eq!(first["baseline"]["commit"], first_baseline);
    assert_eq!(second["baseline"]["commit"], second_baseline);
    assert_ne!(first["baseline"]["commit"], second["baseline"]["commit"]);
    // Nothing is derived from item content: the items carry a path and a kind.
    for item in second["baseline"]["items"].as_array().expect("items") {
        let mut members: Vec<&String> = item.as_object().expect("item").keys().collect();
        members.sort();
        assert_eq!(members, ["kind", "path"]);
    }
}

/// A release commit with two parents (the baseline and a side commit) whose
/// tree equals the baseline plus the record and the snapshot. Returns the
/// baseline commit and the release commit.
fn merge_release(fixture: &Fixture) -> (String, String) {
    let repo = fixture.repo();
    let baseline = commit_baseline(fixture, &[]);
    git(&repo, None, &["checkout", "-q", "-b", "side"]);
    commit_files(fixture, &[("side.txt", "side\n")], "side");
    git(&repo, None, &["checkout", "-q", "main"]);
    git(
        &repo,
        None,
        &[
            "merge",
            "-q",
            "-s",
            "ours",
            "--no-ff",
            "--no-commit",
            "side",
        ],
    );
    let release = Release::standard(&baseline);
    fs::create_dir_all(repo.join("release")).expect("directory");
    fs::write(repo.join(RECORD_PATH), &release.record).expect("record");
    fs::write(repo.join(SNAPSHOT_PATH), "{}\n").expect("snapshot");
    git(&repo, None, &["add", RECORD_PATH, SNAPSHOT_PATH]);
    git(&repo, None, &["commit", "-q", "-m", "merge release"]);
    (baseline, head(fixture))
}

#[test]
#[trace("TC-395", "FR-047-AC-6")]
fn tc_395_a_release_commit_with_two_parents_is_not_a_child_of_the_baseline() {
    let fixture = fixture();
    let (_, commit) = merge_release(&fixture);
    let digest = digest_at(&fixture, &commit);
    sign(&fixture, "a", "v1", &commit, &message(&digest));
    let result = verdict(&fixture, "v1");
    assert_not_released(&result, &["release_commit_not_child_of_baseline"]);
}

#[test]
#[trace("TC-396", "FR-047-AC-6")]
fn tc_396_a_release_commit_on_a_different_parent_is_not_a_child_of_the_baseline() {
    let fixture = fixture();
    let earlier = commit_files(&fixture, &[("earlier.txt", "earlier\n")], "earlier");
    let baseline = commit_baseline(&fixture, &[]);
    git(
        &fixture.repo(),
        None,
        &["checkout", "-q", "--detach", &earlier],
    );
    let release = Release::standard(&baseline).commit(&fixture);
    let digest = digest_at(&fixture, &release);
    sign(&fixture, "a", "v1", &release, &message(&digest));
    let result = verdict(&fixture, "v1");
    assert_not_released(&result, &["release_commit_not_child_of_baseline"]);
}

#[test]
#[trace("TC-397", "FR-047-AC-6")]
fn tc_397_a_parent_that_is_a_descendant_of_the_baseline_is_not_the_baseline() {
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[]);
    commit_files(&fixture, &[("later.txt", "after the baseline\n")], "later");
    let release = Release::standard(&baseline).commit(&fixture);
    let digest = digest_at(&fixture, &release);
    sign(&fixture, "a", "v1", &release, &message(&digest));
    let result = verdict(&fixture, "v1");
    assert_not_released(&result, &["release_commit_not_child_of_baseline"]);
}

#[test]
#[trace("TC-398", "FR-047-AC-6", "FR-051-AC-3")]
fn tc_398_a_release_commit_that_changes_a_further_path_changes_other_paths() {
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[]);
    let mut release = Release::standard(&baseline);
    release
        .extra
        .push(("notes.txt".to_owned(), "one more path\n".to_owned()));
    let commit = release.commit(&fixture);
    let digest = digest_at(&fixture, &commit);
    sign(&fixture, "a", "v1", &commit, &message(&digest));
    let result = verdict(&fixture, "v1");
    assert_not_released(&result, &["release_commit_changes_other_paths"]);
}

#[test]
#[trace("TC-399", "FR-047-AC-6", "FR-051-AC-8")]
fn tc_399_a_record_modified_rather_than_added_is_not_added() {
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[(RECORD_PATH, "an earlier file\n")]);
    let release = Release::standard(&baseline).commit(&fixture);
    let digest = digest_at(&fixture, &release);
    sign(&fixture, "a", "v1", &release, &message(&digest));
    let result = verdict(&fixture, "v1");
    assert_not_released(&result, &["release_commit_record_not_added"]);
}

#[test]
#[trace("TC-400", "FR-051-AC-2")]
fn tc_400_a_release_commit_without_the_snapshot_still_has_every_other_check_evaluated() {
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[]);
    let mut release = Release::standard(&baseline);
    release.snapshot = None;
    let commit = release.commit(&fixture);
    let digest = digest_at(&fixture, &commit);
    // Signed by a mapped person who is not the decision owner: both reasons.
    sign(&fixture, "m", "v1", &commit, &message(&digest));
    let result = verdict(&fixture, "v1");
    assert_eq!(result["status"], "not_released", "{result}");
    assert_eq!(
        reasons(&result),
        [
            "release_commit_snapshot_not_added",
            "decision_owner_mismatch"
        ]
    );
}

#[test]
#[trace("TC-401", "FR-051-AC-3")]
fn tc_401_a_snapshot_in_the_working_tree_or_a_later_commit_is_not_added() {
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[]);
    let mut release = Release::standard(&baseline);
    release.snapshot = None;
    let commit = release.commit(&fixture);
    let digest = digest_at(&fixture, &commit);
    sign(&fixture, "a", "v1", &commit, &message(&digest));
    fs::write(fixture.repo().join(SNAPSHOT_PATH), "{}\n").expect("working tree snapshot");
    let working = verdict(&fixture, "v1");
    assert_not_released(&working, &["release_commit_snapshot_not_added"]);
    commit_files(&fixture, &[(SNAPSHOT_PATH, "{}\n")], "snapshot, too late");
    let later = verdict(&fixture, "v1");
    assert_not_released(&later, &["release_commit_snapshot_not_added"]);
}

#[test]
#[trace("TC-402", "FR-047-AC-6", "FR-051-AC-8")]
fn tc_402_a_snapshot_path_that_exists_at_the_baseline_is_present_at_baseline() {
    // As a Baseline item, as the Baseline file, and as an earlier snapshot.
    for (label, snapshot_path, extra) in [
        ("item", "spec/a.md", Vec::new()),
        ("baseline file", BASELINE_PATH, Vec::new()),
        (
            "earlier snapshot",
            SNAPSHOT_PATH,
            vec![(SNAPSHOT_PATH, "{\"earlier\":true}\n")],
        ),
    ] {
        let fixture = fixture();
        let baseline = commit_baseline(&fixture, &extra);
        let mut release = Release::standard(&baseline);
        release.record = record_text(&baseline, snapshot_path, ALICE);
        release.snapshot_path = snapshot_path.to_owned();
        release.snapshot = Some("{\"replaced\":true}\n".to_owned());
        let commit = release.commit(&fixture);
        let digest = digest_at(&fixture, &commit);
        sign(&fixture, "a", "v1", &commit, &message(&digest));
        let result = verdict(&fixture, "v1");
        assert_eq!(result["status"], "not_released", "{label}: {result}");
        let found = reasons(&result);
        assert!(
            found.contains(&"snapshot_path_present_at_baseline".to_owned()),
            "{label}: {result}"
        );
        // A modified file is not reported as added.
        assert!(
            found.contains(&"release_commit_snapshot_not_added".to_owned()),
            "{label}: {result}"
        );
    }
}

#[test]
#[trace("TC-403", "FR-047-AC-7")]
fn tc_403_a_record_missing_at_the_release_commit_is_unreadable_without_an_approval() {
    let fixture = fixture();
    commit_baseline(&fixture, &[]);
    let commit = commit_files(&fixture, &[(SNAPSHOT_PATH, "{}\n")], "snapshot only");
    sign(&fixture, "a", "v1", &commit, &message("git-blob:unknown"));
    let result = verdict(&fixture, "v1");
    assert_eq!(result["status"], "not_released", "{result}");
    assert_eq!(reasons(&result), ["record_unreadable"]);
    assert!(result.get("approval").is_none(), "{result}");
}

#[test]
#[trace("TC-404", "FR-047-AC-7")]
fn tc_404_an_invalid_record_or_one_outside_the_closed_schema_is_invalid_without_an_approval() {
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[]);
    let valid = record_text(&baseline, SNAPSHOT_PATH, ALICE);
    let cases = [
        (
            "extra member",
            valid.replace("---\n\nThe", "extra: 1\n---\n\nThe"),
        ),
        ("no frontmatter", "The release record.\n".to_owned()),
        (
            "wrong type",
            valid.replace("type: Release", "type: Baseline"),
        ),
        ("bad id", valid.replace("id: REL-1", "id: X-1")),
        (
            "empty owner",
            valid.replace(&format!("decision_owner: {ALICE}"), "decision_owner: \"\""),
        ),
        ("short commit", valid.replace(&baseline, "abc123")),
        (
            "missing snapshot_path",
            valid.replace(&format!("snapshot_path: {SNAPSHOT_PATH}\n"), ""),
        ),
    ];
    for (label, record) in cases {
        let mut release = Release::standard(&baseline);
        release.record = record;
        let commit = release.commit(&fixture);
        let digest = digest_at(&fixture, &commit);
        let tag = format!("v-{}", label.replace(' ', "-"));
        sign(&fixture, "a", &tag, &commit, &message(&digest));
        let result = verdict(&fixture, &tag);
        assert_eq!(result["status"], "not_released", "{label}: {result}");
        assert_eq!(reasons(&result), ["record_invalid"], "{label}: {result}");
        assert!(result.get("approval").is_none(), "{label}: {result}");
    }
}

#[test]
#[trace("TC-405", "FR-051-AC-4")]
fn tc_405_a_snapshot_path_that_breaks_the_path_rules_or_equals_the_record_is_invalid() {
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[]);
    let cases = [
        ("the record path", RECORD_PATH.to_owned()),
        ("absolute", "/etc/snapshot".to_owned()),
        ("dot dot", "../snapshot".to_owned()),
        ("glob", "release/*.json".to_owned()),
        ("control character", "\"release/\\u0007x\"".to_owned()),
        ("leading space", "\"release/ x\"".to_owned()),
    ];
    for (label, snapshot_path) in cases {
        let mut release = Release::standard(&baseline);
        release.record = record_text(&baseline, &snapshot_path, ALICE);
        release.snapshot = None;
        let commit = release.commit(&fixture);
        let digest = digest_at(&fixture, &commit);
        let tag = format!("v-{}", label.replace(' ', "-"));
        sign(&fixture, "a", &tag, &commit, &message(&digest));
        let result = verdict(&fixture, &tag);
        assert_eq!(reasons(&result), ["record_invalid"], "{label}: {result}");
        assert!(result.get("approval").is_none(), "{label}: {result}");
    }
}

#[test]
#[trace("TC-406", "FR-047-AC-8")]
fn tc_406_a_record_edited_in_the_working_tree_after_tagging_does_not_change_the_verdict() {
    let fixture = fixture();
    standard(&fixture);
    let before = verdict(&fixture, "v1");
    assert_eq!(before["status"], "released", "{before}");
    let record = fixture.repo().join(RECORD_PATH);
    fs::write(&record, "---\nnot a record any more\n---\n").expect("edit");
    assert_eq!(verdict(&fixture, "v1"), before);
    fs::remove_file(&record).expect("remove");
    assert_eq!(verdict(&fixture, "v1"), before);
}

#[test]
#[trace("TC-407", "FR-047-AC-9")]
fn tc_407_a_signed_commit_or_a_note_carrying_the_trailers_is_never_released() {
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[]);
    let release = Release::standard(&baseline);
    let record_commit = release.commit(&fixture);
    let digest = digest_at(&fixture, &record_commit);
    let repo = fixture.repo();
    // A signed commit whose message carries the trailers.
    let public = fixture.public("a");
    let key = public.to_str().expect("utf-8");
    git(
        &repo,
        Some(key),
        &[
            "commit",
            "-q",
            "-S",
            "--allow-empty",
            "-m",
            &message(&digest),
        ],
    );
    git(&repo, None, &["branch", "signed-commit"]);
    // A note on the release commit carrying the trailers.
    git(
        &repo,
        None,
        &["notes", "add", "-m", &message(&digest), &record_commit],
    );
    for name in ["signed-commit", &record_commit] {
        let result = verdict(&fixture, name);
        assert_eq!(result["status"], "not_released", "{name}: {result}");
        // FR-041 never approves a commit: it is not an annotated tag object.
        assert_ne!(result["approval"]["status"], "approved", "{name}: {result}");
        assert!(!reasons(&result).is_empty(), "{name}: {result}");
    }
    let note_ref = verdict(&fixture, "refs/notes/commits");
    assert_eq!(note_ref["status"], "not_released", "{note_ref}");
    // A merge commit whose message carries the trailers, named as the tag.
    let merged = self::fixture();
    let (_, merge) = merge_release(&merged);
    let merge_digest = digest_at(&merged, &merge);
    git(
        &merged.repo(),
        None,
        &["commit", "-q", "--amend", "-m", &message(&merge_digest)],
    );
    let result = verdict(&merged, "main");
    assert_eq!(result["status"], "not_released", "{result}");
    assert_ne!(result["approval"]["status"], "approved", "{result}");
    assert!(
        reasons(&result).contains(&"release_commit_not_child_of_baseline".to_owned()),
        "{result}"
    );
}

#[test]
#[trace("TC-423", "FR-047-AC-13")]
fn tc_423_a_replace_ref_cannot_change_the_record_read_under_a_valid_signature() {
    let fixture = fixture();
    let (baseline, release) = standard(&fixture);
    let repo = fixture.repo();
    let record_id = git(
        &repo,
        None,
        &["rev-parse", &format!("{release}:{RECORD_PATH}")],
    );
    let mut forged = Release::standard(&baseline).record;
    forged = forged.replace("minor cosmetic defect", "FORGED critical defect waived");
    let forged_id = String::from_utf8(run_program(
        "git",
        &repo,
        &["hash-object", "-w", "--stdin"],
        Some(forged.as_bytes()),
    ))
    .expect("utf-8");
    git(&repo, None, &["replace", &record_id, forged_id.trim()]);
    assert!(!git(&repo, None, &["for-each-ref", "refs/replace"]).is_empty());
    let result = verdict(&fixture, "v1");
    assert_eq!(result["status"], "released", "{result}");
    assert_eq!(
        result["accepted_anomalies"],
        json!(["minor cosmetic defect"])
    );
}

#[test]
#[trace("TC-424", "FR-047-AC-13")]
fn tc_424_a_graft_or_replace_graft_cannot_turn_a_merge_release_into_a_child() {
    let fixture = fixture();
    let (baseline, commit) = merge_release(&fixture);
    let digest = digest_at(&fixture, &commit);
    sign(&fixture, "a", "v1", &commit, &message(&digest));
    let repo = fixture.repo();
    git(&repo, None, &["replace", "--graft", &commit, &baseline]);
    let replaced = verdict(&fixture, "v1");
    assert_not_released(&replaced, &["release_commit_not_child_of_baseline"]);
    git(&repo, None, &["replace", "-d", &commit]);
    fs::write(
        repo.join(".git/info/grafts"),
        format!("{commit} {baseline}\n"),
    )
    .expect("grafts");
    let grafted = verdict(&fixture, "v1");
    assert_not_released(&grafted, &["release_commit_not_child_of_baseline"]);
}

#[test]
#[trace("TC-425", "FR-047-AC-13")]
fn tc_425_a_git_that_fails_is_a_machine_error_and_never_a_reason() {
    let fixture = fixture();
    standard(&fixture);
    let shim = fixture.root().join("shim-bin");
    fs::create_dir(&shim).expect("shim dir");
    fs::write(shim.join("git"), "#!/bin/sh\nexit 3\n").expect("shim");
    fs::set_permissions(shim.join("git"), fs::Permissions::from_mode(0o755)).expect("mode");
    let body = request(&fixture, "v1");
    for path in [Some(shim.as_path()), None] {
        let mut body = body.clone();
        if path.is_none() {
            // A directory that is not a repository: git exits 128.
            body["repository"] = json!(fixture.root().display().to_string());
        }
        let output = run_command("release-verify", body.to_string().as_bytes(), path);
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
        assert_eq!(error["code"], "release_verify_git_failed", "{error}");
    }
    // A tag that names nothing is not a git failure: the record cannot be read.
    let missing = verdict(&fixture, "no-such-tag");
    assert_eq!(reasons(&missing), ["record_unreadable"], "{missing}");
    assert!(missing.get("approval").is_none());
}

#[test]
#[trace("TC-426", "FR-047-AC-6")]
fn tc_426_a_non_utf8_path_is_not_mistaken_for_the_snapshot_path() {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[]);
    // The snapshot path holds U+FFFD, which is what a lossy decode of 0xFF gives.
    let snapshot_path = "release/\u{fffd}.json";
    let mut release = Release::standard(&baseline);
    release.record = record_text(&baseline, snapshot_path, ALICE);
    let repo = fixture.repo();
    fs::create_dir_all(repo.join("release")).expect("directory");
    fs::write(repo.join(RECORD_PATH), &release.record).expect("record");
    fs::write(repo.join(OsStr::from_bytes(b"release/\xff.json")), "{}\n").expect("odd path");
    git(&repo, None, &["add", "release"]);
    git(&repo, None, &["commit", "-q", "-m", "release"]);
    let commit = head(&fixture);
    let digest = digest_at(&fixture, &commit);
    sign(&fixture, "a", "v1", &commit, &message(&digest));
    let result = verdict(&fixture, "v1");
    assert_not_released(
        &result,
        &[
            "release_commit_changes_other_paths",
            "release_commit_snapshot_not_added",
        ],
    );
}

#[test]
#[trace("TC-427", "FR-046-AC-6")]
fn tc_427_a_baseline_with_more_than_1024_items_is_invalid() {
    let fixture = fixture();
    let items: Vec<(String, &str)> = (0..1025)
        .map(|n| (format!("f{n}.txt"), "evidence"))
        .collect();
    let refs: Vec<(&str, &str)> = items.iter().map(|(p, k)| (p.as_str(), *k)).collect();
    let document = baseline_document(&refs);
    let baseline = commit_files(&fixture, &[(BASELINE_PATH, &document)], "large baseline");
    let release = Release::standard(&baseline).commit(&fixture);
    let digest = digest_at(&fixture, &release);
    sign(&fixture, "a", "v1", &release, &message(&digest));
    let result = verdict(&fixture, "v1");
    assert_not_released(&result, &["baseline_file_invalid"]);
}

#[test]
#[trace("TC-408", "FR-047-AC-10")]
fn tc_408_the_approval_member_equals_the_approval_verify_output() {
    let fixture = fixture();
    let (_, release) = standard(&fixture);
    let digest = digest_at(&fixture, &release);
    let result = verdict(&fixture, "v1");
    let approval = json!({
        "protocol": "engineering-assurance.approval-verify-request/v1",
        "repository": fixture.repo().display().to_string(),
        "tag": "v1",
        "record_id": "REL-1",
        "record_digest": digest,
        "allowed_signers": fixture.allowed().display().to_string(),
    });
    let output = run_command("approval-verify", approval.to_string().as_bytes(), None);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let expected: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(result["approval"], expected);
}

#[test]
#[trace("TC-409", "FR-047-AC-10", "FR-053-AC-2")]
fn tc_409_invalid_requests_are_machine_errors_and_never_start_git() {
    let fixture = fixture();
    standard(&fixture);
    let marker = fixture.root().join("git-was-started");
    let shim = fixture.root().join("shim-bin");
    fs::create_dir(&shim).expect("shim dir");
    fs::write(
        shim.join("git"),
        format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()),
    )
    .expect("shim");
    fs::set_permissions(shim.join("git"), fs::Permissions::from_mode(0o755)).expect("mode");
    let mutations: Vec<(&str, Value)> = vec![
        ("unknown member", json!({"extra": 1})),
        ("attestation member", json!({"attestation": "/tmp/a.json"})),
        ("protocol", json!({"protocol": "wrong"})),
        ("relative repository", json!({"repository": "repo"})),
        ("relative signers", json!({"allowed_signers": "signers"})),
        ("dash tag", json!({"tag": "--help"})),
        ("tag space", json!({"tag": "v 1"})),
        ("dash record path", json!({"record_path": "-release.md"})),
        (
            "absolute record path",
            json!({"record_path": "/release.md"}),
        ),
        (
            "dot dot record path",
            json!({"record_path": "../release.md"}),
        ),
        ("glob record path", json!({"record_path": "release/*.md"})),
        ("gitsign empty", json!({"gitsign": {}})),
    ];
    for (label, mutation) in mutations {
        let mut body = request(&fixture, "v1");
        for (key, value) in mutation.as_object().expect("object") {
            body[key] = value.clone();
        }
        let output = run_command("release-verify", body.to_string().as_bytes(), Some(&shim));
        assert_eq!(output.status.code(), Some(2), "{label}: {output:?}");
        let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
        assert_eq!(error["protocol"], "engineering-assurance.error/v1");
        assert_eq!(error["capability"], "release-verify");
        assert_eq!(error["code"], "release_verify_request_invalid", "{label}");
    }
    let output = run_command("release-verify", b"{not json", Some(&shim));
    assert_eq!(output.status.code(), Some(2));
    let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
    assert_eq!(error["code"], "release_verify_request_unreadable");
    let mut body = request(&fixture, "v1");
    body["allowed_signers"] = json!(fixture.root().join("missing").display().to_string());
    let output = run_command("release-verify", body.to_string().as_bytes(), Some(&shim));
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
    assert_eq!(error["code"], "release_verify_allowed_signers_invalid");
    assert!(!marker.exists(), "a refused request must not start git");
}

#[test]
#[trace("TC-410", "FR-053-AC-1")]
fn tc_410_attestation_files_neither_make_nor_unmake_a_release() {
    let fixture = fixture();
    let (_, release) = standard(&fixture);
    let without = verdict(&fixture, "v1");
    assert_eq!(without["status"], "released", "{without}");
    // Attestation files in the working tree and in a later commit change nothing.
    fs::write(
        fixture.repo().join("release/attestation.json"),
        "{\"verdict\":\"approved\"}\n",
    )
    .expect("working tree attestation");
    commit_files(
        &fixture,
        &[("release/attestation-2.json", "{\"verdict\":\"approved\"}\n")],
        "attestation",
    );
    assert_eq!(verdict(&fixture, "v1"), without);
    // The same release commit under an unsigned tag is never released.
    let digest = digest_at(&fixture, &release);
    git(
        &fixture.repo(),
        None,
        &["tag", "-a", "-m", &message(&digest), "v-unsigned", &release],
    );
    let result = verdict(&fixture, "v-unsigned");
    assert_not_released(&result, &["unsigned"]);
}

#[test]
#[trace("TC-411", "FR-051-AC-5", "FR-051-AC-6")]
fn tc_411_a_snapshot_that_is_not_json_does_not_change_the_verdict_or_the_members() {
    let reference = {
        let fixture = fixture();
        standard(&fixture);
        verdict(&fixture, "v1")
    };
    let fixture = fixture();
    let baseline = commit_baseline(&fixture, &[]);
    let mut release = Release::standard(&baseline);
    release.snapshot = Some("{not json at all\u{0}\n".to_owned());
    let commit = release.commit(&fixture);
    let digest = digest_at(&fixture, &commit);
    sign(&fixture, "a", "v1", &commit, &message(&digest));
    let result = verdict(&fixture, "v1");
    assert_eq!(result["status"], "released", "{result}");
    let members = |value: &Value| -> Vec<String> {
        let mut keys: Vec<String> = value.as_object().expect("object").keys().cloned().collect();
        keys.sort();
        keys
    };
    assert_eq!(members(&result), members(&reference));
    // The only snapshot member is the path the record names.
    assert_eq!(result["release"]["snapshot_path"], SNAPSHOT_PATH);
    assert_eq!(members(&result["release"]), ["id", "name", "snapshot_path"]);
}

#[test]
#[trace("TC-412", "FR-049-AC-1", "FR-049-AC-2")]
fn tc_412_the_result_is_complete_and_the_same_without_any_other_tool_on_the_path() {
    let fixture = fixture();
    standard(&fixture);
    let with_path = verdict(&fixture, "v1");
    // A path that holds only git and ssh-keygen: no Quoin or Quire executable.
    let bin = fixture.root().join("only-git");
    fs::create_dir(&bin).expect("bin");
    for tool in ["git", "ssh-keygen"] {
        let found = ["/usr/bin", "/bin", "/usr/local/bin"]
            .iter()
            .map(|dir| Path::new(dir).join(tool))
            .find(|candidate| candidate.exists())
            .expect("tool on the machine");
        std::os::unix::fs::symlink(found, bin.join(tool)).expect("symlink");
    }
    let restricted = result_on(&request(&fixture, "v1"), Some(&bin));
    assert_eq!(restricted, with_path);
    for member in [
        "status",
        "approver",
        "meaning",
        "signed_at",
        "signed_at_source",
        "baseline",
        "accepted_anomalies",
        "approval",
        "release",
    ] {
        assert!(restricted.get(member).is_some(), "{member}: {restricted}");
    }
}

#[test]
#[trace("TC-428", "FR-046-AC-4")]
fn tc_428_a_baseline_item_spelled_as_pathspec_magic_is_a_literal_path() {
    let fixture = fixture();
    // `:(exclude)spec/a.md` is pathspec magic unless the path is taken literally.
    let items = [
        ("spec/a.md", "spec_artifact"),
        (":(exclude)spec/a.md", "evidence"),
    ];
    let document = baseline_document(&items);
    let baseline = commit_files(
        &fixture,
        &[("spec/a.md", "spec a\n"), (BASELINE_PATH, &document)],
        "baseline",
    );
    let release = Release::standard(&baseline).commit(&fixture);
    let digest = digest_at(&fixture, &release);
    sign(&fixture, "a", "v1", &release, &message(&digest));
    let result = verdict(&fixture, "v1");
    assert_not_released(&result, &["baseline_item_absent"]);
    assert_eq!(result["absent_items"], json!([":(exclude)spec/a.md"]));
}
