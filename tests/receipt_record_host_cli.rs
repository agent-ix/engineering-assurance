// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! CLI boundary coverage for receipt-record production.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

use ix_trace_rs::trace;
use serde_json::{Value, json};
use tempfile::TempDir;

fn git(repository: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(args)
        .output()
        .expect("git starts")
}

fn git_text(repository: &Path, args: &[&str]) -> String {
    let output = git(repository, args);
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git output is UTF-8")
        .trim()
        .to_owned()
}

fn git_policies(repository: &Path) {
    let directory = repository.join(".engineering-assurance");
    fs::create_dir_all(&directory).expect("policy directory");
    fs::write(
        directory.join("receipt-policy.json"),
        json!({
            "protocol": "engineering-assurance.receipt-policy/v1",
            "gates": [{"name": "focused", "argv": ["true"]}],
            "adopted_at": "2026-11-14T00:00:00Z",
            "grace_minutes": 0
        })
        .to_string(),
    )
    .expect("receipt policy");
    fs::write(
        directory.join("approval-policy.json"),
        json!({
            "protocol": "engineering-assurance.approval-policy/v1",
            "required_roles": ["engineer"],
            "individuals": {"alice": [{"source": "github", "id": "123"}]},
            "roles": {"engineer": ["alice"]},
            "agents": []
        })
        .to_string(),
    )
    .expect("approval policy");
    fs::write(
        directory.join("sinks.json"),
        json!({
            "protocol": "engineering-assurance.sink-config/v1",
            "private_store": {"destination": "fictional/records"}
        })
        .to_string(),
    )
    .expect("sink policy");
    fs::write(directory.join("workflow-identities.json"), json!({
        "protocol": "engineering-assurance.workflow-identities/v1",
        "identities": [
            {
                "role": "receipt_writer", "issuer": concat!("https:", "//token.actions.githubusercontent.com"),
                "repository": "fictional/assurance", "workflow_path": ".github/workflows/change-receipt.yml",
                "ref": "refs/heads/main", "source_repository": "fictional/project",
                "source_workflow_path": ".github/workflows/change-receipt.yml", "event": "push",
                "source_ref": "refs/heads/main"
            },
            {
                "role": "statement_writer", "issuer": concat!("https:", "//token.actions.githubusercontent.com"),
                "repository": "fictional/assurance", "workflow_path": ".github/workflows/approval-statement.yml",
                "ref": "refs/heads/main"
            },
            {
                "role": "record_publisher", "issuer": concat!("https:", "//token.actions.githubusercontent.com"),
                "repository": "fictional/assurance", "workflow_path": ".github/workflows/record-publisher.yml",
                "ref": "refs/heads/main"
            }
        ]
    }).to_string()).expect("workflow identity policy");
}

#[trace("TC-1000", "FR-075-AC-9")]
#[test]
fn receipt_record_cli_reads_base_policy_and_retains_private_record_before_signing() {
    let temp = TempDir::new().expect("temporary fixture");
    let source = temp.path().join("source");
    fs::create_dir(&source).expect("source repo");
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&source)
            .status()
            .expect("git init")
            .success()
    );
    for (key, value) in [
        ("user.name", "Receipt Fixture"),
        ("user.email", "receipt@example.invalid"),
    ] {
        git_text(&source, &["config", key, value]);
    }
    git_policies(&source);
    fs::write(source.join("tracked.txt"), "receipt fixture\n").expect("tracked file");
    git_text(&source, &["add", "."]);
    git_text(&source, &["commit", "-qm", "fixture"]);
    git_text(&source, &["branch", "-M", "main"]);
    let commit = git_text(&source, &["rev-parse", "HEAD"]);
    let tree = git_text(&source, &["rev-parse", "HEAD^{tree}"]);

    let remote = temp.path().join("records.git");
    assert!(
        Command::new("git")
            .args(["init", "--bare", "-q"])
            .arg(&remote)
            .status()
            .expect("bare store")
            .success()
    );
    let git_config = temp.path().join("gitconfig");
    fs::write(
        &git_config,
        format!(
            "[url \"file://{}\"]\n    insteadOf = {}\n",
            remote.display(),
            concat!("https:", "//github.com/fictional/records.git")
        ),
    )
    .expect("Git URL rewrite");
    let fake_bin = temp.path().join("bin");
    fs::create_dir(&fake_bin).expect("fake tool directory");
    let fake_gh = fake_bin.join("gh");
    let gh_log = temp.path().join("gh.log");
    fs::write(&fake_gh, format!(
        "#!/bin/sh\necho \"$2\" >> \"$FAKE_GH_LOG\"\ncase \"$2\" in\n  repos/fictional/project) printf '{{\"default_branch\":\"main\",\"private\":true}}\\n' ;;\n  repos/fictional/project/branches/main) printf '{{\"protected\":true,\"commit\":{{\"sha\":\"{commit}\"}}}}\\n' ;;\n  *) exit 2 ;;\nesac\n"
    )).expect("fake gh");
    fs::set_permissions(&fake_gh, fs::Permissions::from_mode(0o755)).expect("executable gh");

    let output = invoke_receipt_record(
        &temp,
        &source,
        &commit,
        &tree,
        &git_config,
        &fake_bin,
        &gh_log,
    );
    assert!(
        !output.status.success(),
        "unavailable signing identity fails closed"
    );
    let error: Value = serde_json::from_slice(&output.stdout).expect("structured CLI error");
    assert_eq!(error["capability"], "receipt-record");
    assert_ne!(
        error["code"],
        "receipt_record_request_invalid",
        "{error}; gh calls={}",
        fs::read_to_string(&gh_log).unwrap_or_default()
    );

    let private_file = "records/66696374696f6e616c/70726f6a656374/09090909-0909-4909-8909-090909090909/private.json";
    let retained = Command::new("git")
        .arg("--git-dir")
        .arg(&remote)
        .args([
            "show",
            &format!("refs/heads/engineering-assurance-private-records:{private_file}"),
        ])
        .output()
        .expect("read retained private record");
    assert!(
        retained.status.success(),
        "private input was not retained: {}; CLI={error}; gh calls={}",
        String::from_utf8_lossy(&retained.stderr),
        fs::read_to_string(&gh_log).unwrap_or_default()
    );
    let retained: Value = serde_json::from_slice(&retained.stdout).expect("private record JSON");
    assert_eq!(
        retained.pointer("/change_record/pullRequest"),
        Some(&json!("fictional/project#17"))
    );
}

fn invoke_receipt_record(
    temp: &TempDir,
    source: &Path,
    commit: &str,
    tree: &str,
    git_config: &Path,
    fake_bin: &Path,
    gh_log: &Path,
) -> Output {
    let gates = temp.path().join("gates.json");
    fs::write(
        &gates,
        json!({
            "protocol": "engineering-assurance.receipt-gate-results/v1",
            "commit": commit,
            "results": [{"name": "focused", "exitCode": 0}]
        })
        .to_string(),
    )
    .expect("gate results");
    let test_result = temp.path().join("test-result.json");
    fs::write(&test_result, json!({
        "_type": concat!("https:", "//in-toto.io/Statement/v1"),
        "subject": [{"name": "fictional/project", "digest": {"gitCommit": commit, "gitTree": tree}}],
        "predicateType": concat!("https:", "//in-toto.io/attestation/test-result/v0.1"),
        "predicate": {"result": "PASSED", "configuration": {}, "passedTests": ["tc_fixture"], "warnedTests": [], "failedTests": []}
    }).to_string()).expect("Test Result");
    let request = json!({
        "protocol": "engineering-assurance.receipt-record-request/v1", "repository": source,
        "github_repository": "fictional/project", "base_branch": "main", "commit": commit,
        "merged_pull_request": "fictional/project#17", "gate_results": gates,
        "test_result": test_result, "record_id": "09090909-0909-4909-8909-090909090909",
        "receipt_signer_repository": "fictional/assurance",
        "receipt_signer_workflow_path": ".github/workflows/change-receipt.yml", "receipt_signer_ref": "refs/heads/main",
        "record_publisher_repository": "fictional/assurance",
        "record_publisher_workflow_path": ".github/workflows/record-publisher.yml", "record_publisher_ref": "refs/heads/main",
        "approval_signer_repository": "fictional/assurance",
        "approval_signer_workflow_path": ".github/workflows/approval-statement.yml", "approval_signer_ref": "refs/heads/main"
    }).to_string();
    let original_path = std::env::var_os("PATH").expect("PATH exists");
    let path = std::env::join_paths(
        std::iter::once(fake_bin.to_path_buf()).chain(std::env::split_paths(&original_path)),
    )
    .expect("fixture PATH");
    Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["receipt-record", "--request", &request])
        .env("PATH", path)
        .env("GIT_CONFIG_GLOBAL", git_config)
        .env("FAKE_GH_LOG", gh_log)
        .env_remove("ACTIONS_ID_TOKEN_REQUEST_URL")
        .env_remove("ACTIONS_ID_TOKEN_REQUEST_TOKEN")
        .output()
        .expect("receipt-record command")
}
