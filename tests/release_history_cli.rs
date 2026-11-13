// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary tests for `release-history` (FR-052) on real repositories
//! generated inside each test. The command verifies no signature, so the
//! fixture tags are unsigned.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const REQUEST_PROTOCOL: &str = "engineering-assurance.release-history-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.release-history/v1";
const DIR: &str = "records";

struct Repo {
    dir: tempfile::TempDir,
}

impl Repo {
    fn new() -> Self {
        let repo = Self {
            dir: tempfile::tempdir().expect("tempdir"),
        };
        repo.git(&["init", "-q", "-b", "main"]);
        repo
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn git_with(&self, date: Option<&str>, arguments: &[&str]) -> String {
        let mut command = Command::new("git");
        command
            .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
            .args(arguments)
            .current_dir(self.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env_remove("GIT_DIR");
        if let Some(date) = date {
            command
                .env("GIT_AUTHOR_DATE", date)
                .env("GIT_COMMITTER_DATE", date);
        }
        let output = command.output().expect("git must start");
        assert!(output.status.success(), "git {arguments:?}: {output:?}");
        String::from_utf8(output.stdout)
            .expect("utf-8")
            .trim_end()
            .to_owned()
    }

    fn git(&self, arguments: &[&str]) -> String {
        self.git_with(None, arguments)
    }

    /// Writes and stages `files`, removes and unstages `removed`, commits on
    /// the current branch and returns the commit.
    fn change(&self, files: &[(&str, &str)], removed: &[&str], date: Option<&str>) -> String {
        for (path, content) in files {
            let target = self.path().join(path);
            fs::create_dir_all(target.parent().expect("parent")).expect("directories");
            fs::write(target, content).expect("file");
            self.git(&["add", "--", path]);
        }
        for path in removed {
            self.git(&["rm", "-q", "--", path]);
        }
        self.git_with(date, &["commit", "-q", "--allow-empty", "-m", "change"]);
        self.head()
    }

    fn commit(&self, files: &[(&str, &str)]) -> String {
        self.change(files, &[], None)
    }

    fn head(&self) -> String {
        self.git(&["rev-parse", "HEAD"])
    }

    fn tag(&self, name: &str, target: &str) {
        self.git(&["tag", "-a", "-m", "release", name, target]);
    }
}

fn record(id: &str, revision: u64) -> String {
    format!("{DIR}/{id}/{revision}.json")
}

fn request(repo: &Repo, baseline: &str, tag: Option<&str>) -> Value {
    json!({
        "protocol": REQUEST_PROTOCOL,
        "repository": repo.path().display().to_string(),
        "baseline_commit": baseline,
        "previous_release_tag": tag,
        "records_dir": DIR,
    })
}

fn run_with_path(request: &Value, path: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"));
    command
        .args(["release-history", "--request", "-"])
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
        .write_all(request.to_string().as_bytes())
        .expect("request");
    child.wait_with_output().expect("output")
}

fn run(request: &Value) -> Output {
    run_with_path(request, None)
}

/// The result line of a request that must succeed.
fn listed(request: &Value) -> Value {
    let output = run(request);
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

/// The error code of a request that must be refused with exit 2.
fn refused_with(output: &Output) -> String {
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).expect("error line");
    assert_eq!(value["protocol"], "engineering-assurance.error/v1");
    value["code"].as_str().expect("code").to_owned()
}

fn refused(request: &Value) -> String {
    refused_with(&run(request))
}

/// `(record_id, revision, commit)` of each listed record.
fn triples(result: &Value) -> Vec<(String, u64, String)> {
    result["records"]
        .as_array()
        .expect("records")
        .iter()
        .map(|entry| {
            (
                entry["record_id"].as_str().expect("id").to_owned(),
                entry["revision"].as_u64().expect("revision"),
                entry["commit"].as_str().expect("commit").to_owned(),
            )
        })
        .collect()
}

fn triple(id: &str, revision: u64, commit: &str) -> (String, u64, String) {
    (id.to_owned(), revision, commit.to_owned())
}

/// Previous baseline, a release commit tagged `v1` on top of it, and its parent.
fn first_release(repo: &Repo) -> (String, String) {
    let baseline = repo.commit(&[("base.txt", "base\n")]);
    let release = repo.commit(&[("release.md", "release one\n")]);
    repo.tag("v1", &release);
    (baseline, release)
}

#[trace("TC-500", "FR-052-AC-1")]
#[test]
fn tc_500_lists_the_records_added_in_the_range_with_their_commits_in_order() {
    let repo = Repo::new();
    repo.commit(&[(&record("old", 1), "{}")]);
    let (_, release) = first_release(&repo);
    let c1 = repo.commit(&[(&record("zeta", 1), "{}")]);
    repo.commit(&[("other.txt", "x\n")]);
    let c3 = repo.commit(&[(&record("alpha", 2), "{}"), (&record("alpha", 1), "{}")]);

    let result = listed(&request(&repo, &c3, Some("v1")));

    assert_eq!(result["baseline_commit"], c3.as_str());
    assert_eq!(
        result["previous_release"],
        json!({ "tag": "v1", "commit": release })
    );
    assert_eq!(
        triples(&result),
        [
            triple("alpha", 1, &c3),
            triple("alpha", 2, &c3),
            triple("zeta", 1, &c1)
        ]
    );
    assert_eq!(result["records"][0]["path"], record("alpha", 1));
}

#[trace("TC-501", "FR-052-AC-2")]
#[test]
fn tc_501_skips_records_from_before_the_range_and_records_deleted_inside_it() {
    let repo = Repo::new();
    repo.commit(&[(&record("before", 1), "{}")]);
    first_release(&repo);
    repo.commit(&[(&record("gone", 1), "{}"), (&record("kept", 1), "{}")]);
    let tip = repo.change(&[], &[&record("gone", 1)], None);

    let result = listed(&request(&repo, &tip, Some("v1")));

    let ids: Vec<String> = triples(&result).into_iter().map(|(id, ..)| id).collect();
    assert_eq!(ids, ["kept"]);
}

#[trace("TC-502", "FR-052-AC-3")]
#[test]
fn tc_502_without_a_previous_tag_lists_every_record_present() {
    let repo = Repo::new();
    let first = repo.commit(&[(&record("a", 1), "{}")]);
    let second = repo.commit(&[(&record("b", 1), "{}")]);

    let result = listed(&request(&repo, &second, None));

    assert_eq!(result["previous_release"], Value::Null);
    assert_eq!(
        triples(&result),
        [triple("a", 1, &first), triple("b", 1, &second)]
    );
}

#[trace("TC-503", "FR-052-AC-4")]
#[test]
fn tc_503_a_release_commit_never_merged_still_gives_the_records_after_its_parent() {
    let repo = Repo::new();
    let baseline = repo.commit(&[("base.txt", "base\n")]);
    repo.git(&["checkout", "-q", "-b", "release-branch"]);
    let release = repo.commit(&[("release.md", "r\n"), (&record("rel", 1), "{}")]);
    repo.tag("v1", &release);
    repo.git(&["checkout", "-q", "main"]);
    let after = repo.commit(&[(&record("after", 1), "{}")]);

    let result = listed(&request(&repo, &after, Some("v1")));

    assert_eq!(result["previous_release"]["commit"], release.as_str());
    assert_eq!(triples(&result), [triple("after", 1, &after)]);
    assert_ne!(baseline, release);
}

#[trace("TC-504", "FR-052-AC-5")]
#[test]
fn tc_504_each_unusable_tag_or_commit_has_its_own_code() {
    let repo = Repo::new();
    let (baseline, release) = first_release(&repo);
    let tip = repo.commit(&[(&record("a", 1), "{}")]);

    // A tag on a two-parent commit.
    repo.git(&["checkout", "-q", "-b", "side", &baseline]);
    repo.commit(&[("side.txt", "s\n")]);
    repo.git(&["checkout", "-q", "main"]);
    repo.git(&["merge", "-q", "--no-ff", "-m", "merge", "side"]);
    let merge = repo.head();
    repo.tag("v-merge", &merge);
    assert_eq!(
        refused(&request(&repo, &merge, Some("v-merge"))),
        "release_history_previous_not_a_release_commit"
    );

    assert_eq!(
        refused(&request(&repo, &tip, Some("v-missing"))),
        "release_history_previous_tag_unknown"
    );

    assert_eq!(
        refused(&request(&repo, &"0".repeat(40), Some("v1"))),
        "release_history_baseline_commit_unknown"
    );

    // A release whose parent is not an ancestor of the baseline commit.
    repo.git(&["checkout", "-q", "--orphan", "unrelated"]);
    let unrelated_base = repo.commit(&[("u.txt", "u\n")]);
    let unrelated_release = repo.commit(&[("u2.txt", "u\n")]);
    repo.tag("v-unrelated", &unrelated_release);
    repo.git(&["checkout", "-q", "main"]);
    assert_eq!(
        refused(&request(&repo, &tip, Some("v-unrelated"))),
        "release_history_previous_not_ancestor"
    );
    assert_ne!(unrelated_base, release);
}

#[trace("TC-505", "FR-052-AC-6")]
#[test]
fn tc_505_working_tree_edits_and_invalid_record_bytes_change_nothing() {
    let repo = Repo::new();
    first_release(&repo);
    let tip = repo.commit(&[
        (&record("a", 1), "not json at all"),
        (&record("b", 1), "{}"),
    ]);
    let request = request(&repo, &tip, Some("v1"));
    let before = run(&request).stdout;
    assert_eq!(triples(&listed(&request)).len(), 2);

    fs::write(repo.path().join(record("a", 1)), "{\"edited\":true}").expect("edit");
    fs::remove_file(repo.path().join(record("b", 1))).expect("remove");
    fs::create_dir_all(repo.path().join(DIR).join("c")).expect("directory");
    fs::write(repo.path().join(record("c", 1)), "{}").expect("untracked");

    assert_eq!(run(&request).stdout, before);
}

#[trace("TC-506", "FR-052-AC-7")]
#[test]
fn tc_506_leaves_the_repository_unchanged_and_refuses_bad_requests_before_git() {
    let repo = Repo::new();
    first_release(&repo);
    let tip = repo.commit(&[(&record("a", 1), "{}")]);
    let state = || {
        (
            repo.git(&["for-each-ref"]),
            repo.git(&["status", "--porcelain=v2", "--branch"]),
            fs::read(repo.path().join(".git/index")).expect("index"),
        )
    };
    let before = state();
    listed(&request(&repo, &tip, Some("v1")));
    assert_eq!(state(), before);

    // With no git on PATH a request that reaches git fails; an invalid one does not.
    let nowhere = tempfile::tempdir().expect("empty directory");
    let good = request(&repo, &tip, Some("v1"));
    assert_eq!(
        refused_with(&run_with_path(&good, Some(nowhere.path()))),
        "release_history_git_failed"
    );
    let mut unknown_member = good.clone();
    unknown_member["extra"] = json!(1);
    let mut relative = good.clone();
    relative["repository"] = json!("relative/repo");
    let mut dashed_dir = good.clone();
    dashed_dir["records_dir"] = json!("-records");
    let mut missing_tag = good.clone();
    missing_tag
        .as_object_mut()
        .expect("object")
        .remove("previous_release_tag");
    let mut dashed_tag = good.clone();
    dashed_tag["previous_release_tag"] = json!("-v1");
    for bad in [
        unknown_member,
        relative,
        dashed_dir,
        missing_tag,
        dashed_tag,
    ] {
        assert_eq!(
            refused_with(&run_with_path(&bad, Some(nowhere.path()))),
            "release_history_request_invalid",
            "{bad}"
        );
    }
}

#[trace("TC-507", "FR-052-AC-8")]
#[test]
fn tc_507_two_runs_over_one_state_are_byte_identical() {
    let repo = Repo::new();
    first_release(&repo);
    let tip = repo.commit(&[(&record("b", 1), "{}"), (&record("a", 1), "{}")]);
    let request = request(&repo, &tip, Some("v1"));

    let first = run(&request);
    let second = run(&request);

    assert_eq!(first.status.code(), Some(0));
    assert_eq!(first.stdout, second.stdout);
}

#[trace("TC-508", "FR-052-AC-9")]
#[test]
fn tc_508_a_merged_record_gets_the_merge_commit_and_the_earliest_add_wins_whatever_the_dates() {
    let repo = Repo::new();
    first_release(&repo);
    let fork = repo.head();
    repo.git(&["checkout", "-q", "-b", "side"]);
    repo.commit(&[(&record("side", 1), "{}")]);
    repo.git(&["checkout", "-q", "main", "--detach"]);
    repo.git(&["checkout", "-q", "main"]);
    repo.commit(&[("main.txt", "m\n")]);
    repo.git(&["merge", "-q", "--no-ff", "-m", "merge side", "side"]);
    let merge = repo.head();
    // Added, deleted and added again; the first add carries the later date.
    let first_add = repo.change(
        &[(&record("twice", 1), "{}")],
        &[],
        Some("2030-01-01T00:00:00Z"),
    );
    repo.change(&[], &[&record("twice", 1)], Some("2020-01-01T00:00:00Z"));
    let tip = repo.change(
        &[(&record("twice", 1), "{}")],
        &[],
        Some("2020-01-02T00:00:00Z"),
    );

    let result = listed(&request(&repo, &tip, Some("v1")));

    assert_eq!(
        triples(&result),
        [triple("side", 1, &merge), triple("twice", 1, &first_add)]
    );
    assert_ne!(fork, merge);
}

#[trace("TC-509", "FR-052-AC-10")]
#[test]
fn tc_509_without_a_previous_tag_a_record_from_the_root_is_reported_at_the_root() {
    let repo = Repo::new();
    let root = repo.commit(&[(&record("a", 1), "{}")]);
    repo.commit(&[("x.txt", "x\n")]);
    let tip = repo.commit(&[(&record("a", 1), "{\"changed\":true}")]);

    let result = listed(&request(&repo, &tip, None));

    assert_eq!(triples(&result), [triple("a", 1, &root)]);
}

#[trace("TC-510", "FR-052-AC-11")]
#[test]
fn tc_510_revisions_order_numerically_and_other_files_are_ignored() {
    let repo = Repo::new();
    first_release(&repo);
    let ignored = [
        format!("{DIR}/a/x.json.tmp"),
        format!("{DIR}/a/v2.json"),
        format!("{DIR}/a/sub/3.json"),
        format!("{DIR}/.staging/a/1.json"),
        format!("{DIR}/a/07.json"),
        format!("{DIR}/a/0.json"),
        format!("{DIR}/a/1.5.json"),
        format!("{DIR}/a/+4.json"),
        format!("{DIR}/a/5.json.bak"),
        format!("{DIR}/top.json"),
    ];
    let mut files: Vec<(&str, &str)> = ignored.iter().map(|path| (path.as_str(), "{}")).collect();
    let (nine, ten) = (record("a", 9), record("a", 10));
    files.extend([(nine.as_str(), "{}"), (ten.as_str(), "{}")]);
    let tip = repo.commit(&files);

    let result = listed(&request(&repo, &tip, Some("v1")));

    assert_eq!(
        triples(&result),
        [triple("a", 9, &tip), triple("a", 10, &tip)]
    );
}

#[trace("TC-511", "FR-052-AC-12")]
#[test]
fn tc_511_malformed_baselines_and_records_dirs_are_refused_before_git() {
    let repo = Repo::new();
    first_release(&repo);
    let tip = repo.commit(&[(&record("a", 1), "{}")]);
    let nowhere = tempfile::tempdir().expect("empty directory");
    let mut cases = Vec::new();
    for baseline in [
        "main".to_owned(),
        tip[..12].to_owned(),
        tip.to_uppercase(),
        format!("{tip}0"),
    ] {
        cases.push(request(&repo, &baseline, Some("v1")));
    }
    for dir in ["", "/records", "records/../x", "rec*ds", "rec[a]ds", "rec?"] {
        let mut bad = request(&repo, &tip, Some("v1"));
        bad["records_dir"] = json!(dir);
        cases.push(bad);
    }
    for bad in cases {
        assert_eq!(
            refused_with(&run_with_path(&bad, Some(nowhere.path()))),
            "release_history_request_invalid",
            "{bad}"
        );
    }
}

#[trace("TC-512", "FR-052-AC-13")]
#[test]
fn tc_512_shallow_clones_branch_named_tags_and_non_commit_tags_are_distinct_refusals() {
    let repo = Repo::new();
    first_release(&repo);
    let tip = repo.commit(&[(&record("a", 1), "{}")]);

    repo.git(&["branch", "v9"]);
    assert_eq!(
        refused(&request(&repo, &tip, Some("v9"))),
        "release_history_previous_tag_unknown"
    );

    // A ref outside refs/tags/ and a tag whose name starts with refs/tags/ must
    // not be guessed into the tags v2 and v3.
    repo.git(&["branch", "refs/tags/v2"]);
    repo.tag("refs/tags/v3", &tip);
    for tag in ["v2", "v3"] {
        assert_eq!(
            refused(&request(&repo, &tip, Some(tag))),
            "release_history_previous_tag_unknown",
            "{tag}"
        );
    }

    let tree = repo.git(&["rev-parse", "HEAD^{tree}"]);
    let blob = repo.git(&["rev-parse", &format!("HEAD:{}", record("a", 1))]);
    repo.git(&["tag", "tree-tag", &tree]);
    repo.git(&["tag", "blob-tag", &blob]);
    for tag in ["tree-tag", "blob-tag"] {
        assert_eq!(
            refused(&request(&repo, &tip, Some(tag))),
            "release_history_previous_not_a_release_commit"
        );
    }

    let shallow: PathBuf = repo.path().with_extension("shallow");
    repo.git(&[
        "clone",
        "-q",
        "--depth",
        "1",
        &format!("file://{}", repo.path().display()),
        shallow.to_str().expect("utf-8"),
    ]);
    let mut shallow_request = request(&repo, &tip, Some("v-missing"));
    shallow_request["repository"] = json!(shallow.display().to_string());
    let code = refused(&shallow_request);
    fs::remove_dir_all(&shallow).expect("clean up the clone");
    assert_eq!(code, "release_history_repository_shallow");
}

#[trace("TC-515", "FR-052-AC-15")]
#[test]
fn tc_515_a_previous_baseline_only_reachable_through_a_merge_is_refused() {
    let repo = Repo::new();
    repo.commit(&[("base.txt", "base\n")]);
    repo.git(&["checkout", "-q", "-b", "rel"]);
    let previous_baseline = repo.commit(&[(&record("inprev", 1), "{}")]);
    let release = repo.commit(&[("release.md", "r\n")]);
    repo.tag("v1", &release);
    repo.git(&["checkout", "-q", "main"]);
    repo.commit(&[("main.txt", "m\n")]);
    repo.git(&[
        "merge",
        "-q",
        "--no-ff",
        "-m",
        "merge release branch",
        "rel",
    ]);
    let tip = repo.head();

    let output = run(&request(&repo, &tip, Some("v1")));

    assert_eq!(
        refused_with(&output),
        "release_history_previous_not_on_first_parent"
    );
    assert_ne!(previous_baseline, tip);
}

#[trace("TC-516", "FR-052-AC-16")]
#[test]
fn tc_516_git_failure_is_refused_and_replace_refs_and_grafts_change_nothing() {
    let not_a_repository = tempfile::tempdir().expect("directory");
    let repo = Repo::new();
    let (baseline, _) = first_release(&repo);
    let middle = repo.commit(&[(&record("mid", 1), "{}")]);
    let tip = repo.commit(&[(&record("top", 1), "{}")]);
    let mut broken = request(&repo, &tip, Some("v1"));
    broken["repository"] = json!(not_a_repository.path().display().to_string());
    assert_eq!(refused(&broken), "release_history_git_failed");

    let request = request(&repo, &tip, Some("v1"));
    let before = run(&request).stdout;
    assert_eq!(triples(&listed(&request)).len(), 2);

    // Replace the middle commit by a copy that adds no record.
    let tree = repo.git(&["rev-parse", &format!("{baseline}^{{tree}}")]);
    let parent = repo.git(&["rev-parse", &format!("{middle}^")]);
    let forged = repo.git(&["commit-tree", &tree, "-p", &parent, "-m", "forged"]);
    repo.git(&["replace", &middle, &forged]);
    // Graft the tip so that it has no parent.
    fs::write(repo.path().join(".git/info/grafts"), format!("{tip}\n")).expect("grafts");

    assert_eq!(run(&request).stdout, before);
}

#[trace("TC-517", "FR-052-AC-17")]
#[test]
fn tc_517_a_record_path_that_cannot_be_reported_is_refused_unless_it_is_gone() {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    let repo = Repo::new();
    first_release(&repo);
    let huge = record("a", 1).replace("/1.json", "/18446744073709551616.json");
    repo.commit(&[(&huge, "{}")]);
    let tip = repo.head();
    let request_value = request(&repo, &tip, Some("v1"));
    assert_eq!(
        refused(&request_value),
        "release_history_record_unrepresentable"
    );

    let gone = repo.change(&[], &[&huge], None);
    assert_eq!(
        triples(&listed(&self::request(&repo, &gone, Some("v1")))),
        []
    );

    let directory = repo.path().join(DIR);
    let raw = OsStr::from_bytes(b"\xff");
    fs::create_dir_all(directory.join(raw)).expect("directory");
    fs::write(directory.join(raw).join("1.json"), "{}").expect("file");
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "non-utf8"]);
    let tip = repo.head();
    assert_eq!(
        refused(&self::request(&repo, &tip, Some("v1"))),
        "release_history_record_unrepresentable"
    );
}
