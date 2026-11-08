// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Contract and process-boundary tests for declared-scope verification (FR-032).

use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

use engineering_assurance::scope_footprint::{
    Footprint, MAX_CHANGED_PATHS, MAX_PATH_BYTES, MAX_SCOPE_ENTRIES, ScopeError, ScopeStatus,
    verify_scope,
};
use ix_trace_rs::trace;
use serde_json::{Value, json};

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn observed(paths: &[&str], truncated: bool) -> Footprint {
    Footprint::Observed {
        paths: strings(paths),
        truncated,
    }
}

#[test]
#[trace("TC-232", "FR-032-AC-1")]
fn tc_232_exact_entries_and_directory_prefixes_cover_the_change() {
    let verdict = verify_scope(
        &strings(&["src/a", "README.md"]),
        &observed(
            &["src/a/x.rs", "src/a/deep/y.rs", "README.md", "src/a"],
            false,
        ),
    )
    .expect("valid inputs");
    assert_eq!(verdict.status, ScopeStatus::Complete);
    assert!(verdict.uncovered_paths.is_empty());
    assert!(verdict.footprint_available);
    assert!(!verdict.truncated);
    // An empty change under an empty declaration is complete too.
    let empty = verify_scope(&[], &observed(&[], false)).expect("valid inputs");
    assert_eq!(empty.status, ScopeStatus::Complete);
}

#[test]
#[trace("TC-233", "FR-032-AC-2")]
fn tc_233_uncovered_paths_make_a_false_scope() {
    let verdict = verify_scope(
        &strings(&["src/a"]),
        &observed(&["z.txt", "src/ab/x.rs", "src/a/y.rs"], false),
    )
    .expect("valid inputs");
    assert_eq!(verdict.status, ScopeStatus::FalseScope);
    // The textual-prefix sibling `src/ab` is not covered by `src/a`; sorted.
    assert_eq!(verdict.uncovered_paths, strings(&["src/ab/x.rs", "z.txt"]));
    // A file entry does not cover a child spelled with another name.
    let file = verify_scope(&strings(&["src/a.rs"]), &observed(&["src/a.rs.bak"], false))
        .expect("valid inputs");
    assert_eq!(file.status, ScopeStatus::FalseScope);
    // An observed violation stands even when the footprint is also truncated.
    let cut =
        verify_scope(&strings(&["src/a"]), &observed(&["other"], true)).expect("valid inputs");
    assert_eq!(cut.status, ScopeStatus::FalseScope);
    assert!(cut.truncated);
}

#[test]
#[trace("TC-234", "FR-032-AC-3", "FR-032-CON-1")]
fn tc_234_unobserved_footprints_are_incomplete_never_complete() {
    let cut = verify_scope(&strings(&["src"]), &observed(&["src/x.rs"], true)).expect("valid");
    assert_eq!(cut.status, ScopeStatus::Incomplete);
    assert!(cut.truncated);
    assert!(cut.footprint_available);
    let none = verify_scope(&strings(&["src"]), &Footprint::Unavailable).expect("valid");
    assert_eq!(none.status, ScopeStatus::Incomplete);
    assert!(!none.footprint_available);
    assert!(!none.truncated);
    assert!(none.uncovered_paths.is_empty());
    let empty_cut = verify_scope(&[], &observed(&[], true)).expect("valid");
    assert_eq!(empty_cut.status, ScopeStatus::Incomplete);
}

#[test]
#[trace("TC-235", "FR-032-AC-4")]
fn tc_235_malformed_paths_duplicates_and_bounds_are_refused() {
    let long = "a/".repeat(MAX_PATH_BYTES);
    let bad = [
        "",
        "/abs",
        "a//b",
        "a/./b",
        "./a",
        "a/../b",
        "..",
        ".",
        "a\\b",
        "a\0b",
        "a/",
        "a\nb",
        "C:/x",
        long.as_str(),
    ];
    for path in bad {
        assert_eq!(
            verify_scope(&strings(&["ok"]), &observed(&[path], false)),
            Err(ScopeError::InvalidChangedPath { index: 0 }),
            "changed path {path:?}"
        );
        assert_eq!(
            verify_scope(&strings(&[path]), &observed(&["ok"], false)),
            Err(ScopeError::InvalidScopeEntry { index: 0 }),
            "scope entry {path:?}"
        );
    }
    assert_eq!(
        verify_scope(&strings(&["a", "b", "a"]), &Footprint::Unavailable),
        Err(ScopeError::DuplicateScopeEntry { index: 2 })
    );
    let many_scope: Vec<String> = (0..=MAX_SCOPE_ENTRIES).map(|i| format!("d{i}")).collect();
    assert_eq!(
        verify_scope(&many_scope, &Footprint::Unavailable),
        Err(ScopeError::ScopeTooLarge)
    );
    let many_paths: Vec<String> = (0..=MAX_CHANGED_PATHS).map(|i| format!("d{i}")).collect();
    assert_eq!(
        verify_scope(
            &[],
            &Footprint::Observed {
                paths: many_paths,
                truncated: false
            }
        ),
        Err(ScopeError::FootprintTooLarge)
    );
    // Duplicate changed paths are one path, not an error.
    let verdict = verify_scope(&strings(&["a"]), &observed(&["a/x", "a/x"], false)).expect("set");
    assert_eq!(verdict.status, ScopeStatus::Complete);
}

#[test]
#[trace("TC-236", "FR-032-AC-5")]
fn tc_236_unused_scope_is_reported_without_changing_the_status() {
    let verdict = verify_scope(
        &strings(&["zeta", "src/a", "docs", "alpha/b"]),
        &observed(&["src/a/x.rs"], false),
    )
    .expect("valid inputs");
    assert_eq!(verdict.status, ScopeStatus::Complete);
    assert_eq!(verdict.unused_scope, strings(&["alpha/b", "docs", "zeta"]));
    // A nested entry under a used one is still unused unless a path reaches it.
    // Over-declaration is not knowable from a partial or missing footprint.
    let cut = verify_scope(&strings(&["docs"]), &observed(&["src/a/x.rs"], true)).expect("valid");
    assert!(cut.unused_scope.is_empty());
    let nested = verify_scope(&strings(&["src", "src/a"]), &observed(&["src/b.rs"], false))
        .expect("valid inputs");
    assert_eq!(nested.unused_scope, strings(&["src/a"]));
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
}

fn git(root: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(arguments)
        .current_dir(root)
        .env_remove("GIT_DIR")
        .output()
        .expect("git must start");
    assert!(output.status.success(), "git {arguments:?}: {output:?}");
    String::from_utf8(output.stdout).expect("utf-8")
}

fn write(root: &Path, path: &str, content: &str) {
    let full = root.join(path);
    fs::create_dir_all(full.parent().expect("parent")).expect("dirs");
    fs::write(full, content).expect("write");
}

/// A repository with a base commit, returning the temp dir.
fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    git(root, &["init", "-q", "-b", "main"]);
    write(
        root,
        "src/a/one.rs",
        "one one one one one\nline two\nline three\n",
    );
    write(root, "src/a/gone.rs", "gone\n");
    write(
        root,
        "docs/guide.md",
        "guide guide guide guide\nsecond\nthird\nfourth\n",
    );
    write(root, "keep.txt", "keep\n");
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "base"]);
    dir
}

fn commit_all(root: &Path, message: &str) {
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", message]);
}

fn request(root: &Path, base: &str, candidate: &str, scope: &[&str]) -> Value {
    json!({
        "protocol": "engineering-assurance.scope-footprint-request/v1",
        "repository": fs::canonicalize(root).expect("root").display().to_string(),
        "base": base,
        "candidate": candidate,
        "scope": scope,
    })
}

fn run(request: &Value) -> Output {
    let mut child = bin()
        .args(["scope-footprint", "--request", "-"])
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

fn verdict(output: &Output) -> Value {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = std::str::from_utf8(&output.stdout).expect("utf-8");
    assert!(
        text.ends_with('\n') && text.matches('\n').count() == 1,
        "{text:?}"
    );
    let value: Value = serde_json::from_str(text).expect("json line");
    assert_eq!(
        value["protocol"],
        "engineering-assurance.scope-footprint-result/v1"
    );
    value
}

#[test]
#[trace("TC-237", "FR-032-AC-6")]
fn tc_237_the_command_reports_the_real_footprint() {
    let repo = repository();
    let root = repo.path();
    write(root, "src/a/one.rs", "changed\n");
    write(root, "src/a/new.rs", "new\n");
    fs::remove_file(root.join("src/a/gone.rs")).expect("delete");
    commit_all(root, "in scope");
    let inside = verdict(&run(&request(root, "HEAD~1", "HEAD", &["src/a"])));
    assert_eq!(inside["status"], "complete");
    assert_eq!(inside["uncovered_paths"], json!([]));
    assert_eq!(inside["footprint_available"], true);

    // A tag and a full commit id resolve the same way.
    git(root, &["tag", "cand"]);
    let by_tag = verdict(&run(&request(root, "main~1", "cand", &["src/a"])));
    assert_eq!(by_tag["status"], "complete");

    write(root, "docs/guide.md", "edited\n");
    commit_all(root, "out of scope");
    let outside = verdict(&run(&request(root, "HEAD~1", "HEAD", &["src/a"])));
    assert_eq!(outside["status"], "false_scope");
    assert_eq!(outside["uncovered_paths"], json!(["docs/guide.md"]));
    assert_eq!(outside["unused_scope"], json!(["src/a"]));

    // A rename reports both its paths: covering only the new one is a false scope.
    git(root, &["mv", "docs/guide.md", "src/a/guide.md"]);
    commit_all(root, "rename");
    let renamed_new_only = verdict(&run(&request(root, "HEAD~1", "HEAD", &["src/a"])));
    assert_eq!(renamed_new_only["status"], "false_scope");
    assert_eq!(
        renamed_new_only["uncovered_paths"],
        json!(["docs/guide.md"])
    );
    let renamed_both = verdict(&run(&request(root, "HEAD~1", "HEAD", &["src/a", "docs"])));
    assert_eq!(renamed_both["status"], "complete");

    let unknown = verdict(&run(&request(
        root,
        "HEAD~1",
        "no-such-revision",
        &["src/a"],
    )));
    assert_eq!(unknown["status"], "incomplete");
    assert_eq!(unknown["footprint_available"], false);

    let plain = tempfile::tempdir().expect("tempdir");
    let not_repository = verdict(&run(&request(plain.path(), "HEAD~1", "HEAD", &["src/a"])));
    assert_eq!(not_repository["status"], "incomplete");
    assert_eq!(not_repository["footprint_available"], false);
}

#[test]
#[trace("TC-238", "FR-032-AC-7")]
fn tc_238_invalid_requests_yield_machine_errors_and_exit_two() {
    let repo = repository();
    let root = repo.path();
    let marker = root.join("git-was-started");
    let mut cases: Vec<(Vec<u8>, &str)> = vec![
        (b"not json".to_vec(), "scope_footprint_request_unreadable"),
        (b"[]".to_vec(), "scope_footprint_request_invalid"),
    ];
    let mut unknown_field = request(root, "HEAD", "HEAD", &[]);
    unknown_field["extra"] = json!(1);
    cases.push((
        unknown_field.to_string().into_bytes(),
        "scope_footprint_request_invalid",
    ));
    let mut wrong_protocol = request(root, "HEAD", "HEAD", &[]);
    wrong_protocol["protocol"] = json!("engineering-assurance.scope-footprint-request/v2");
    cases.push((
        wrong_protocol.to_string().into_bytes(),
        "scope_footprint_request_invalid",
    ));
    let mut relative = request(root, "HEAD", "HEAD", &[]);
    relative["repository"] = json!("relative/dir");
    cases.push((
        relative.to_string().into_bytes(),
        "scope_footprint_request_invalid",
    ));
    cases.push((
        request(root, "HEAD", "HEAD", &["a/../b"])
            .to_string()
            .into_bytes(),
        "scope_footprint_scope_invalid",
    ));
    cases.push((
        request(root, "HEAD", "HEAD", &["a", "a"])
            .to_string()
            .into_bytes(),
        "scope_footprint_scope_invalid",
    ));
    for revision in ["--output=git-was-started", "", "a b"] {
        cases.push((
            request(root, revision, "HEAD", &["a"])
                .to_string()
                .into_bytes(),
            "scope_footprint_revision_invalid",
        ));
        cases.push((
            request(root, "HEAD", revision, &["a"])
                .to_string()
                .into_bytes(),
            "scope_footprint_revision_invalid",
        ));
    }
    for (body, code) in cases {
        let mut child = bin()
            .args(["scope-footprint", "--request", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(&body)
            .expect("body");
        let output = child.wait_with_output().expect("output");
        assert_eq!(output.status.code(), Some(2), "{code}: {output:?}");
        let error: Value = serde_json::from_slice(&output.stdout).expect("error json");
        assert_eq!(error["protocol"], "engineering-assurance.error/v1");
        assert_eq!(error["capability"], "scope-footprint");
        assert_eq!(error["code"], code);
        assert!(!output.stderr.is_empty());
    }
    assert!(
        !marker.exists(),
        "an option-like revision must not reach git"
    );
    // The request may also be the argument value.
    let output = bin()
        .args(["scope-footprint", "--request", "not json"])
        .output()
        .expect("start");
    assert_eq!(output.status.code(), Some(2));
}
