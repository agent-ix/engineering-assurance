// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

use std::process::Command;

use ix_trace_rs::trace;

use super::*;

fn git(repo: &std::path::Path, arguments: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
        .args(arguments)
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .expect("git must start");
    assert!(status.success(), "{arguments:?}");
}

#[test]
#[trace("TC-365", "FR-043-AC-16")]
fn an_expired_total_time_truncates_before_the_first_tag() {
    let directory = tempfile::tempdir().expect("tempdir");
    let repo = directory.path().join("repo");
    std::fs::create_dir(&repo).expect("repo");
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "base"]);
    git(&repo, &["tag", "a-light"]);
    git(&repo, &["tag", "-a", "-m", "one", "b-annotated"]);
    git(&repo, &["tag", "-a", "-m", "two", "c-annotated"]);
    let signers = directory.path().join("allowed_signers");
    std::fs::write(&signers, "").expect("signers");
    let request = serde_json::json!({
        "protocol": REQUEST_PROTOCOL,
        "repository": repo.display().to_string(),
        "record_id": "r-1",
        "record_digest": "d-1",
        "allowed_signers": signers.display().to_string(),
    });
    let line = execute_within(request.to_string().as_bytes(), Format::Json, Duration::ZERO)
        .expect("a trail");
    let result: Value = serde_json::from_slice(&line).expect("json");
    assert_eq!(result["entries"], serde_json::json!([]));
    assert_eq!(result["truncated"], true);
    assert_eq!(result["truncation"], "time_limit");
    // Only the lightweight ref before the first annotated tag was reached.
    assert_eq!(result["considered"], 1);
    assert_eq!(result["complete"], false);
}
