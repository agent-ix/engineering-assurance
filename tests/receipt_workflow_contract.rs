// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Guard the Engineering Assurance receipt workflow structure.

use std::{fs, path::Path, process::Command};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use ix_trace_rs::trace;
use serde_json::{Value, json};

mod common;

use common::is_structural_action_reference;

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn workflow(name: &str) -> Value {
    let path = repository_root().join(".github/workflows").join(name);
    let bytes = fs::read(path).expect("receipt workflow exists");
    yaml_serde::from_slice(&bytes).expect("receipt workflow parses as YAML")
}

fn assert_permission(job: &Value, expected: &Value, name: &str) {
    assert_eq!(&job["permissions"], expected, "{name} permissions");
}

fn assert_actions_are_structural(steps: &[Value]) {
    for step in steps {
        if let Some(reference) = step["uses"].as_str() {
            assert!(
                is_structural_action_reference(reference),
                "action reference has a valid local or external structure: {reference}"
            );
        }
        if let Some(run) = step["run"].as_str() {
            assert!(
                !run.contains("${{"),
                "run scripts receive context through env"
            );
        }
        if step["uses"]
            .as_str()
            .is_some_and(|value| value.starts_with("actions/checkout@"))
        {
            assert_eq!(
                step["with"]["persist-credentials"], false,
                "checkout does not retain credentials"
            );
        }
    }
}

fn assert_record_job_is_isolated(jobs: &Value) {
    let record_steps = jobs["record"]["steps"].as_array().expect("record steps");
    assert!(
        record_steps.iter().all(|step| {
            !step["uses"].as_str().is_some_and(|reference| {
                reference.starts_with("actions/checkout@") || reference.starts_with("./")
            })
        }),
        "the identity-bearing record job neither checks out nor loads repository code"
    );
    assert!(
        record_steps.iter().any(|step| {
            step["run"]
                .as_str()
                .is_some_and(|run| run.contains("receipt-record --request"))
        }),
        "record job invokes the published receipt-record command"
    );
    assert!(
        record_steps.iter().any(|step| {
            step["uses"]
                .as_str()
                .is_some_and(|reference| reference.starts_with("actions/download-artifact@"))
        }),
        "record job receives gate outputs and Test Result as data"
    );
    assert!(
        record_steps.iter().all(|step| {
            !step["run"]
                .as_str()
                .is_some_and(|run| run.contains("cargo test") || run.contains("python3 -"))
        }),
        "record job does not execute repository gates or inline repository scripts"
    );
}

fn assert_gate_job_boundary(jobs: &Value) -> String {
    let gate_steps = jobs["gates"]["steps"].as_array().expect("gate steps");
    let gate_text = gate_steps
        .iter()
        .filter_map(|step| step["run"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(gate_text.contains("producer-execute"));
    assert!(gate_text.contains("evidence record --suite SUITE-001"));
    assert!(gate_text.contains("evidence export-test-result"));
    assert!(gate_text.contains("receipt-gate-results/v1"));
    assert!(
        gate_text.contains("TC-560"),
        "the executed gate carries its declared TC-560 trace"
    );
    assert!(
        gate_text.contains("exitCode"),
        "gate outcomes use the published receipt camelCase contract"
    );
    assert!(gate_text.contains(" archive \"$INPUT_COMMIT\""));
    assert!(gate_text.contains("sha256sum"));
    assert!(jobs["gates"]["permissions"].get("id-token").is_none());
    assert!(jobs["gates"].get("secrets").is_none());
    assert!(jobs["gates"].get("environment").is_none());
    let runner = jobs["gates"]["runs-on"].as_str().expect("declared runner");
    assert!(
        ["ubuntu-", "macos-", "windows-"]
            .iter()
            .any(|prefix| runner.starts_with(prefix)),
        "gate job uses a GitHub-hosted runner"
    );
    gate_text
}

fn assert_gate_failure_exports_data_before_failing(gate_text: &str, jobs: &Value) {
    let export_at = gate_text
        .find("evidence export-test-result")
        .expect("the gates job exports Quoin Test Result data");
    let fail_after_export_at = gate_text
        .find("if [[ -e \"$RUNNER_TEMP/receipt-gate-failed\" ]]")
        .expect("the gate failure is reported after the export");
    assert!(export_at < fail_after_export_at);
    assert!(
        jobs["record"]["if"]
            .as_str()
            .is_some_and(|value| value.contains("always()"))
    );
    assert!(
        jobs["gates"]["steps"]
            .as_array()
            .is_some_and(|steps| steps.iter().any(|step| {
                step["uses"]
                    .as_str()
                    .is_some_and(|value| value.starts_with("actions/upload-artifact@"))
                    && step["if"].as_str().is_some_and(|condition| {
                        condition.contains("always()")
                            && condition.contains("adoption-boundary.outputs.skip != 'true'")
                    })
            }))
    );
}

fn assert_receipt_policy_and_suite() {
    let policy_path = repository_root().join(".engineering-assurance/receipt-policy.json");
    let policy: Value =
        serde_json::from_slice(&fs::read(policy_path).expect("receipt policy exists"))
            .expect("receipt policy parses as JSON");
    assert_eq!(
        policy["protocol"],
        "engineering-assurance.receipt-policy/v1"
    );
    assert_eq!(policy["gates"].as_array().map(Vec::len), Some(1));
    assert_eq!(policy["gates"][0]["name"], "receipt-workflow-contract");
    assert_eq!(policy["gates"][0]["argv"][0], "cargo");
    assert_eq!(policy["gates"][0]["argv"][1], "test");
    assert_eq!(policy["gates"][0]["argv"][2], "--locked");
    let gate_command = policy["gates"][0]["argv"]
        .as_array()
        .expect("receipt gate argv")
        .iter()
        .map(|argument| argument.as_str().expect("receipt gate argument"))
        .collect::<Vec<_>>()
        .join(" ");
    let workflow = workflow("change-receipt.yml");
    let record_steps = workflow["jobs"]["record"]["steps"]
        .as_array()
        .expect("record steps");
    assert!(
        record_steps.iter().all(|step| {
            !step["run"]
                .as_str()
                .is_some_and(|run| run.contains(&gate_command))
        }),
        "record job does not execute a configured receipt-policy command"
    );
    let suites = fs::read_to_string(repository_root().join("spec/evidence/suites.md"))
        .expect("suite registry exists");
    assert!(suites.contains("| SUITE-001 | Receipt workflow contract gate |"));
    assert!(suites.contains("| Integration |"));
}

#[cfg(unix)]
fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write fixture executable");
    let mut permissions = fs::metadata(path).expect("fixture metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make fixture executable");
}

#[cfg(unix)]
fn producer_request(root: &Path, path: &Path, marker: &Path, policy: &Value) -> Value {
    let argv = policy["gates"][0]["argv"]
        .as_array()
        .expect("fixture policy argv")
        .iter()
        .map(|argument| argument.as_str().expect("fixture argv string"))
        .collect::<Vec<_>>();
    let executable = argv.first().expect("fixture policy executable");
    json!({
        "protocol": "engineering-assurance.producer-execution-request/v1",
        "producer": {"name": "receipt-adversarial-gate", "version": "1", "executable": executable},
        "caller": {"kind": "fixture.receipt-gate", "version": "1"},
        "procedure": "direct",
        "capabilityRoot": fs::canonicalize(root).expect("canonical fixture root").display().to_string(),
        "arguments": argv.iter().skip(1).map(|value| json!({"kind":"literal", "value":value})).collect::<Vec<_>>(),
        "environment": {
            "PATH": format!("{}:/usr/bin:/bin", path.display()),
            "FIXTURE_MARKER_PATH": marker.display().to_string()
        },
        "inputs": [],
        "stdin": {"kind": "null"},
        "outputs": [],
        "containment": {"profile": "process-group-v1", "contract": {"kind": "fixture.receipt-gate", "version": "1"}},
        "cancellation": {"kind": "disabled"},
        "budget": {
            "timeoutMillis": 5000,
            "maxStdoutBytes": 1024,
            "maxStderrBytes": 1024,
            "maxInputBytes": 1024,
            "maxOutputArtifacts": 0,
            "maxOutputBytes": 0,
            "maxDescendants": 64,
            "maxConcurrency": 1
        },
        "response": {
            "protocol": {"kind": "engineering-assurance.process-evidence", "version": "1"},
            "adapter": {"kind": "engineering-assurance.cli-process-evidence-adapter", "version": "1"},
            "exitCodes": {"kind": "exact", "codes": [0]}
        }
    })
}

#[cfg(unix)]
fn git_at(root: &Path, arguments: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(arguments)
        .current_dir(root)
        .output()
        .expect("git starts")
}

#[cfg(unix)]
fn git_text_at(root: &Path, arguments: &[&str]) -> String {
    let output = git_at(root, arguments);
    assert!(
        output.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git output is UTF-8")
        .trim()
        .to_owned()
}

#[cfg(unix)]
fn git_executable_path() -> std::path::PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("test PATH exists"))
        .map(|directory| directory.join("git"))
        .find(|path| path.is_file())
        .expect("git executable is on PATH")
}

#[cfg(unix)]
fn configure_fixture_repo(repository: &Path) {
    git_text_at(repository, &["config", "user.name", "Receipt Fixture"]);
    git_text_at(
        repository,
        &["config", "user.email", "receipt@example.invalid"],
    );
}

#[cfg(unix)]
fn commit_fixture_file(repository: &Path, name: &str, contents: &str, message: &str) -> String {
    fs::write(repository.join(name), contents).expect("fixture file writes");
    git_text_at(repository, &["add", name]);
    git_text_at(repository, &["commit", "-qm", message]);
    git_text_at(repository, &["rev-parse", "HEAD"])
}

#[cfg(unix)]
fn create_notes_fixture(root: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let remote = root.join("remote.git");
    assert!(
        Command::new("git")
            .args(["init", "--bare", "-q"])
            .arg(&remote)
            .status()
            .expect("bare git init starts")
            .success()
    );
    assert!(
        Command::new("git")
            .arg(format!("--git-dir={}", remote.display()))
            .args(["symbolic-ref", "HEAD", "refs/heads/main"])
            .status()
            .expect("bare remote HEAD is set")
            .success()
    );
    let checkout = root.join("checkout");
    fs::create_dir_all(&checkout).expect("checkout directory is created");
    assert!(
        git_at(&checkout, &["init", "-q"]).status.success(),
        "fixture repository initializes"
    );
    configure_fixture_repo(&checkout);
    commit_fixture_file(&checkout, "base.txt", "base\n", "base");
    git_text_at(&checkout, &["branch", "-M", "main"]);
    git_text_at(
        &checkout,
        &[
            "remote",
            "add",
            "origin",
            remote.to_str().expect("remote path UTF-8"),
        ],
    );
    git_text_at(&checkout, &["push", "-u", "origin", "main"]);
    (remote, checkout)
}

#[cfg(unix)]
fn receipt_notes_workflow_script() -> String {
    let receipt_workflow = workflow("change-receipt.yml");
    let steps = receipt_workflow["jobs"]["notes"]["steps"]
        .as_array()
        .expect("notes job steps");
    steps
        .iter()
        .find(|step| step["name"] == "Store the signed receipt in Git notes")
        .and_then(|step| step["run"].as_str())
        .expect("notes store script")
        .to_owned()
}

#[cfg(unix)]
fn prepare_notes_runner(root: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let runner_temp = root.join("runner-temp");
    let artifacts = runner_temp.join("receipt-signed-data");
    let bin = root.join("bin");
    fs::create_dir_all(&artifacts).expect("receipt artifacts directory");
    fs::create_dir_all(&bin).expect("fixture bin directory");
    fs::write(
        artifacts.join("receipt-result.json"),
        r#"{"bundles":{"change_record":{},"test_result":{},"summary":{}}}"#,
    )
    .expect("receipt result artifact");
    fs::write(
        artifacts.join("receipt-context.json"),
        r#"{"merged_pull_request":"fictional/project#17"}"#,
    )
    .expect("receipt context artifact");
    write_executable(
        &bin.join("gh"),
        "#!/bin/sh\nset -eu\n[ \"$1\" = auth ] && [ \"$2\" = setup-git ]\n",
    );
    write_executable(
        &bin.join("receipt-note-cli"),
        "#!/bin/sh\nset -eu\nrequest=\"$3\"\ncommit=$(printf '%s' \"$request\" | jq -r '.commit')\nnote=\"$RUNNER_TEMP/receipt-note.txt\"\nprintf '{\\\"bundle\\\":\\\"change:%s\\\"}\\n{\\\"bundle\\\":\\\"test:%s\\\"}\\n{\\\"bundle\\\":\\\"summary:%s\\\"}\\n' \"$commit\" \"$commit\" \"$commit\" > \"$note\"\ngit notes --ref=refs/notes/assurance-receipts add -f -F \"$note\" \"$commit\"\n",
    );
    (runner_temp, bin)
}

#[cfg(unix)]
#[derive(Clone, Copy)]
struct ReceiptJobOptions<'a> {
    extra_path: Option<&'a Path>,
    barrier: Option<(&'a Path, &'a Path)>,
}

#[cfg(unix)]
fn run_receipt_notes_job(
    script: &str,
    repository: &Path,
    runner_temp: &Path,
    bin: &Path,
    commit: &str,
    reject_count_file: &Path,
    options: ReceiptJobOptions<'_>,
) -> std::process::Output {
    let mut paths = Vec::new();
    if let Some(extra_path) = options.extra_path {
        paths.push(extra_path.display().to_string());
    }
    paths.push(bin.display().to_string());
    paths.push("/usr/bin:/bin".to_owned());
    let mut command = Command::new("/bin/bash");
    command
        .args(["-c", script])
        .current_dir(repository)
        .env("PATH", paths.join(":"))
        .env("EA_CLI", bin.join("receipt-note-cli"))
        .env("RUNNER_TEMP", runner_temp)
        .env("INPUT_COMMIT", commit)
        .env("REPOSITORY", "fictional/project")
        .env("DEFAULT_BRANCH", "main")
        .env("GH_TOKEN", "fixture-token")
        .env("REJECTION_COUNT_FILE", reject_count_file);
    if let Some((barrier_ready, barrier_peer)) = options.barrier {
        command
            .env("BARRIER_READY", barrier_ready)
            .env("BARRIER_PEER", barrier_peer)
            .env("REAL_GIT", git_executable_path());
    }
    command.output().expect("receipt notes job starts")
}

#[cfg(unix)]
#[trace("TC-599", "FR-076-AC-3")]
#[test]
fn receipt_note_jobs_merge_stale_pushes_and_fail_after_six_rejections() {
    let script = receipt_notes_workflow_script();

    let concurrent_root = tempfile::tempdir().expect("concurrent receipt fixture");
    let (remote, _seed) = create_notes_fixture(concurrent_root.path());
    let checkout_a = concurrent_root.path().join("checkout-a");
    assert!(
        Command::new("git")
            .args(["clone", "-q"])
            .arg(&remote)
            .arg(&checkout_a)
            .status()
            .expect("first git clone starts")
            .success()
    );
    configure_fixture_repo(&checkout_a);
    let commit_a = commit_fixture_file(&checkout_a, "change-a.txt", "a\n", "change a");
    git_text_at(&checkout_a, &["push", "origin", "main"]);
    let checkout_b = concurrent_root.path().join("checkout-b");
    assert!(
        Command::new("git")
            .args(["clone", "-q"])
            .arg(&remote)
            .arg(&checkout_b)
            .status()
            .expect("second git clone starts")
            .success()
    );
    configure_fixture_repo(&checkout_b);
    let commit_b = commit_fixture_file(&checkout_b, "change-b.txt", "b\n", "change b");
    git_text_at(&checkout_b, &["push", "origin", "main"]);
    let (runner_temp_a, bin_a) = prepare_notes_runner(&concurrent_root.path().join("run-a"));
    let count_a = concurrent_root.path().join("pushes-a.txt");
    let (runner_temp_b, bin_b) = prepare_notes_runner(&concurrent_root.path().join("run-b"));
    let count_b = concurrent_root.path().join("pushes-b.txt");
    let barrier = concurrent_root.path().join("push-barrier");
    fs::create_dir(&barrier).expect("push barrier directory");
    let ready_a = barrier.join("a-ready");
    let ready_b = barrier.join("b-ready");
    let git_wrapper_a = concurrent_root.path().join("git-wrapper-a");
    let git_wrapper_b = concurrent_root.path().join("git-wrapper-b");
    fs::create_dir(&git_wrapper_a).expect("first git wrapper directory");
    fs::create_dir(&git_wrapper_b).expect("second git wrapper directory");
    let barrier_git = "#!/bin/sh\nset -eu\nif [ \"${1-}\" = push ]; then\n  : > \"$BARRIER_READY\"\n  count=0\n  while [ ! -e \"$BARRIER_PEER\" ]; do\n    count=$((count + 1))\n    if [ \"$count\" -ge 500 ]; then exit 80; fi\n    sleep 0.01\n  done\nfi\nexec \"$REAL_GIT\" \"$@\"\n";
    write_executable(&git_wrapper_a.join("git"), barrier_git);
    write_executable(&git_wrapper_b.join("git"), barrier_git);
    let script_first = script.clone();
    let checkout_first_thread = checkout_a.clone();
    let runner_first_thread = runner_temp_a.clone();
    let bin_first_thread = bin_a.clone();
    let count_first_thread = count_a.clone();
    let wrapper_first_thread = git_wrapper_a.clone();
    let ready_first_thread = ready_a.clone();
    let ready_second_thread = ready_b.clone();
    let commit_first_thread = commit_a.clone();
    let run_first_thread = std::thread::spawn(move || {
        run_receipt_notes_job(
            &script_first,
            &checkout_first_thread,
            &runner_first_thread,
            &bin_first_thread,
            &commit_first_thread,
            &count_first_thread,
            ReceiptJobOptions {
                extra_path: Some(&wrapper_first_thread),
                barrier: Some((&ready_first_thread, &ready_second_thread)),
            },
        )
    });
    let script_second = script.clone();
    let count_second_thread = count_b.clone();
    let wrapper_second_thread = git_wrapper_b.clone();
    let ready_second_thread = ready_b.clone();
    let ready_first_thread = ready_a.clone();
    let commit_second_thread = commit_b.clone();
    let run_second_thread = std::thread::spawn(move || {
        run_receipt_notes_job(
            &script_second,
            &checkout_b,
            &runner_temp_b,
            &bin_b,
            &commit_second_thread,
            &count_second_thread,
            ReceiptJobOptions {
                extra_path: Some(&wrapper_second_thread),
                barrier: Some((&ready_second_thread, &ready_first_thread)),
            },
        )
    });
    let run_first = run_first_thread.join().expect("first notes job joins");
    let run_second = run_second_thread.join().expect("second notes job joins");
    assert_concurrent_note_jobs_succeeded(&run_first, &run_second, &remote, &commit_a, &commit_b);
    assert_rejected_note_push_exhausts_retries();
}

#[cfg(unix)]
fn assert_concurrent_note_jobs_succeeded(
    run_first: &std::process::Output,
    run_second: &std::process::Output,
    remote: &Path,
    commit_a: &str,
    commit_b: &str,
) {
    assert!(
        run_first.status.success(),
        "first note job failed with {:?}: stdout={} stderr={}",
        run_first.status.code(),
        String::from_utf8_lossy(&run_first.stdout),
        String::from_utf8_lossy(&run_first.stderr)
    );
    assert!(
        run_second.status.success(),
        "stale note job did not recover: {}",
        String::from_utf8_lossy(&run_second.stderr)
    );
    assert_remote_notes_contain_commits(remote, commit_a, commit_b);
}

#[cfg(unix)]
fn assert_remote_notes_contain_commits(remote: &Path, commit_a: &str, commit_b: &str) {
    let remote_notes = Command::new("git")
        .arg(format!("--git-dir={}", remote.display()))
        .args(["notes", "--ref=refs/notes/assurance-receipts", "list"])
        .output()
        .expect("remote notes list runs");
    assert!(remote_notes.status.success());
    let note_objects = String::from_utf8(remote_notes.stdout).expect("notes output is UTF-8");
    assert!(note_objects.lines().any(|line| line.ends_with(&commit_a)));
    assert!(note_objects.lines().any(|line| line.ends_with(&commit_b)));
}

#[cfg(unix)]
fn assert_rejected_note_push_exhausts_retries() {
    let script = receipt_notes_workflow_script();
    let rejected_root = tempfile::tempdir().expect("rejected receipt fixture");
    let (rejected_remote, rejected_checkout) = create_notes_fixture(rejected_root.path());
    let rejected_commit = commit_fixture_file(
        &rejected_checkout,
        "change.txt",
        "rejected\n",
        "rejected change",
    );
    git_text_at(&rejected_checkout, &["push", "origin", "main"]);
    let rejection_count = rejected_root.path().join("rejections.txt");
    write_executable(
        &rejected_remote.join("hooks/pre-receive"),
        "#!/bin/sh\nset -eu\nprintf 'rejected\\n' >> \"$REJECTION_COUNT_FILE\"\nexit 1\n",
    );
    let (runner_temp, bin) = prepare_notes_runner(&rejected_root.path().join("run"));
    let run = run_receipt_notes_job(
        &script,
        &rejected_checkout,
        &runner_temp,
        &bin,
        &rejected_commit,
        &rejection_count,
        ReceiptJobOptions {
            extra_path: None,
            barrier: None,
        },
    );
    assert_eq!(run.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&run.stdout).contains("receipt_note_conflict"));
    assert_eq!(
        fs::read_to_string(rejection_count).unwrap().lines().count(),
        6
    );
    let remote_notes_ref = Command::new("git")
        .arg(format!("--git-dir={}", rejected_remote.display()))
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            "refs/notes/assurance-receipts",
        ])
        .status()
        .expect("remote note ref check runs");
    assert!(
        !remote_notes_ref.success(),
        "rejected receipt leaves no remote note"
    );
}

#[cfg(unix)]
#[trace("TC-595", "FR-075-AC-6")]
#[test]
fn adversarial_receipt_gate_cannot_read_signing_tokens_or_use_signer() {
    let root = tempfile::tempdir().expect("temporary fixture root");
    let bin = root.path().join("bin");
    fs::create_dir(&bin).expect("fixture bin directory");
    let marker = root.path().join("adversarial-gate-marker");
    let shell = fs::canonicalize("/bin/sh").expect("resolve shell to a regular executable");
    let adversarial_command = "if [ -n \"${GH_TOKEN:-}\" ] || [ -n \"${ACTIONS_ID_TOKEN_REQUEST_TOKEN:-}\" ]; then exit 41; fi; if engineering-assurance receipt-record --request /dev/null; then exit 42; else code=$?; [ \"$code\" -eq 77 ] || exit 43; fi; printf 'token-absent-and-signer-refused\\n' >> \"$FIXTURE_MARKER_PATH\"";
    let fixture_policy = json!({
        "protocol": "engineering-assurance.receipt-policy/v1",
        "gates": [{"name":"adversarial-token-gate", "argv":[shell.display().to_string(), "-c", adversarial_command]}]
    });
    write_executable(
        &bin.join("engineering-assurance"),
        "#!/bin/sh\nprintf 'signer-invoked\\n' >> \"$FIXTURE_MARKER_PATH\"\nif [ -n \"${GH_TOKEN:-}\" ] || [ -n \"${ACTIONS_ID_TOKEN_REQUEST_TOKEN:-}\" ]; then exit 76; fi\nexit 77\n",
    );

    let workflow = workflow("change-receipt.yml");
    let jobs = &workflow["jobs"];
    let gate_text = assert_gate_job_boundary(jobs);
    assert_record_job_is_isolated(jobs);
    assert!(gate_text.contains("producer-execute"));

    let request = producer_request(root.path(), &bin, &marker, &fixture_policy);
    let output = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["producer-execute", "--request"])
        .arg(request.to_string())
        .env("GH_TOKEN", "ambient-fixture-token")
        .env("ACTIONS_ID_TOKEN_REQUEST_TOKEN", "ambient-fixture-token")
        .output()
        .expect("producer-execute starts");
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    let result: Value = serde_json::from_slice(&output.stdout).expect("producer result JSON");
    assert_eq!(result["state"]["kind"], "completed", "result: {result}");
    assert_eq!(result["state"]["observation"]["exitCode"], 0);
    let marker_text = fs::read_to_string(marker).expect("fixture gate wrote its outcome");
    assert_eq!(
        marker_text,
        "signer-invoked\ntoken-absent-and-signer-refused\n"
    );
}

#[trace("TC-560", "TC-563", "FR-070-AC-4", "FR-070-AC-7", "FR-070-AC-15")]
#[test]
fn receipt_workflow_jobs_have_exact_permissions_and_keep_code_out_of_record_job() {
    let workflow = workflow("change-receipt.yml");
    assert_eq!(
        workflow["on"].as_object().map(serde_json::Map::len),
        Some(1)
    );
    assert!(workflow["on"].get("workflow_call").is_some());
    assert_eq!(
        workflow["on"]["workflow_call"]["inputs"]["commit"]["type"],
        "string"
    );

    let jobs = &workflow["jobs"];
    assert_permission(
        &jobs["gates"],
        &serde_json::json!({"contents":"read"}),
        "gates",
    );
    assert_permission(
        &jobs["record"],
        &serde_json::json!({"contents":"read", "id-token":"write"}),
        "record",
    );
    assert_permission(
        &jobs["notes"],
        &serde_json::json!({"contents":"write"}),
        "notes",
    );

    for name in ["gates", "record", "notes"] {
        let steps = jobs[name]["steps"].as_array().expect("job steps");
        assert_actions_are_structural(steps);
    }
    assert_record_job_is_isolated(jobs);
    assert!(!format!("{workflow}").contains("statements_not_retained"));

    let gate_text = assert_gate_job_boundary(jobs);
    assert!(
        gate_text.contains("\"--request\", req_path.read_text()"),
        "producer-execute receives the serialized request content, not its path"
    );
    assert!(
        gate_text.contains("[\"rustup\", \"which\", \"cargo\"]")
            && gate_text.contains("RUSTUP_TOOLCHAIN"),
        "producer-execute receives the real Cargo binary, not the Rustup proxy"
    );
    assert_gate_failure_exports_data_before_failing(&gate_text, jobs);
    assert!(
        jobs["record"]["if"]
            .as_str()
            .is_some_and(|value| value.contains("always()"))
    );
    assert_receipt_policy_and_suite();
}

#[trace("TC-592", "FR-075-AC-3")]
#[test]
fn failed_receipt_run_does_not_write_to_the_default_branch() {
    let workflow = workflow("change-receipt.yml");
    let jobs = &workflow["jobs"];
    let gate_text = assert_gate_job_boundary(jobs);
    assert_gate_failure_exports_data_before_failing(&gate_text, jobs);
    assert_record_job_is_isolated(jobs);
    assert!(
        jobs["record"]["if"]
            .as_str()
            .is_some_and(|value| value.contains("always()"))
    );

    let note_steps = jobs["notes"]["steps"].as_array().expect("notes steps");
    let note_writer = note_steps
        .iter()
        .find(|step| {
            step["run"]
                .as_str()
                .is_some_and(|run| run.contains("receipt-note --request"))
        })
        .expect("receipt notes writer");
    let note_script = note_writer["run"].as_str().expect("notes script");
    assert!(note_script.contains("refs/notes/assurance-receipts"));
    assert!(!note_script.contains("git push origin refs/heads/"));
    assert!(!note_script.contains("git reset"));
    assert!(!note_script.contains("git checkout"));
}

#[trace("TC-593", "FR-075-AC-4")]
#[test]
fn receipt_gates_read_the_policy_from_the_protected_base_checkout() {
    let workflow = workflow("change-receipt.yml");
    let jobs = &workflow["jobs"];
    let gate_steps = jobs["gates"]["steps"].as_array().expect("gate steps");
    let checkout = gate_steps
        .iter()
        .find(|step| step["name"] == "Check out the protected branch policy as data")
        .expect("protected base checkout");
    assert_eq!(
        checkout["with"]["ref"],
        "${{ github.event.repository.default_branch }}"
    );
    assert_eq!(checkout["with"]["path"], "protected-base");
    let run = gate_steps
        .iter()
        .find(|step| {
            step["run"].as_str().is_some_and(|script| {
                script.contains("receipt-policy.json") && script.contains("producer-execute")
            })
        })
        .expect("gate runner reads receipt policy")["run"]
        .as_str()
        .expect("gate runner script");
    assert!(run.contains("protected / \".engineering-assurance/receipt-policy.json\""));
    assert!(!run.contains("source / \".engineering-assurance/receipt-policy.json\""));
}

#[trace("TC-594", "FR-075-AC-5")]
#[test]
fn commits_before_adoption_skip_receipt_record_and_note_jobs() {
    let receipt_workflow = workflow("change-receipt.yml");
    let jobs = &receipt_workflow["jobs"];
    assert!(
        jobs["gates"]["outputs"]["skip_before_adoption"]
            .as_str()
            .is_some_and(|value| value.contains("adoption-boundary.outputs.skip"))
    );
    let steps = jobs["gates"]["steps"].as_array().expect("gate steps");
    let boundary = steps
        .iter()
        .find(|step| step["id"] == "adoption-boundary")
        .expect("adoption boundary check");
    let script = boundary["run"].as_str().expect("adoption script");
    assert!(script.contains("receipt-policy.json"));
    assert!(script.contains("git -C \"$SOURCE_DIR\" merge-base --is-ancestor"));
    assert!(script.contains("skip=true"));
    assert!(
        steps
            .iter()
            .filter(|step| step["id"] != "adoption-boundary"
                && step["name"] != "Check the pushed commit input"
                && step["name"] != "Check out the pushed commit as gate input"
                && step["name"] != "Check out the protected branch policy as data")
            .all(|step| {
                step["if"]
                    .as_str()
                    .is_some_and(|condition| condition.contains("adoption-boundary.outputs.skip"))
            })
    );
    assert!(
        jobs["record"]["if"]
            .as_str()
            .is_some_and(|condition| condition.contains("skip_before_adoption != 'true'"))
    );
    assert!(
        jobs["notes"]["if"]
            .as_str()
            .is_some_and(|condition| condition.contains("needs.record.result == 'success'"))
    );

    let missing = workflow("missing-receipt-check.yml");
    let checker = missing["jobs"]["check"]["steps"]
        .as_array()
        .expect("missing receipt steps")
        .iter()
        .find(|step| {
            step["run"]
                .as_str()
                .is_some_and(|script| script.contains("receipt-missing"))
        })
        .expect("missing receipt invocation");
    assert!(
        checker["run"]
            .as_str()
            .expect("missing receipt script")
            .contains("receipt-missing --request")
    );
}

#[trace("TC-559", "FR-070-AC-3")]
#[test]
fn action_reference_structure_rejects_missing_or_malformed_components() {
    for valid in [
        "./.github/workflows/change-receipt.yml",
        "./actions/local/action.yml",
        "actions/checkout@v7",
        "owner/repository/path/to/action@v1.2.3",
        "owner/repository@refs/tags/v2",
    ] {
        assert!(
            is_structural_action_reference(valid),
            "valid action reference rejected: {valid}"
        );
    }

    for invalid in [
        "./",
        ".//action.yml",
        "./../action.yml",
        "./actions//action.yml",
        "./actions/action@v1",
        "@v4",
        "owner/@v4",
        "/repository@v4",
        "owner/repository@",
        "owner//repository@v4",
        "owner/repository@v4@extra",
        "owner/repository@v4 extra",
        "owner/repository@refs//heads/main",
    ] {
        assert!(
            !is_structural_action_reference(invalid),
            "malformed action reference accepted: {invalid}"
        );
    }
}

#[trace("TC-991", "FR-070-AC-16")]
#[test]
fn main_receipt_caller_is_absent_and_missing_check_is_declared() {
    let policy_root = repository_root().join(".engineering-assurance");
    for policy in [
        "workflow-identities.json",
        "approval-policy.json",
        "sinks.json",
    ] {
        assert!(
            !policy_root.join(policy).exists(),
            "test assumes protected receipt policy `{policy}` is not configured"
        );
    }
    assert!(
        !repository_root()
            .join(".github/workflows/receipt.yml")
            .exists(),
        "main receipt caller stays disabled until protected receipt policies are configured"
    );
    let check = workflow("missing-receipt-check.yml");
    assert_eq!(check["on"].as_object().map(serde_json::Map::len), Some(2));
    assert!(check["on"].get("schedule").is_some());
    assert!(check["on"].get("workflow_dispatch").is_some());
    assert_eq!(
        check["jobs"]["check"]["permissions"],
        serde_json::json!({"contents":"read"})
    );
    let steps = check["jobs"]["check"]["steps"]
        .as_array()
        .expect("check steps");
    assert_actions_are_structural(steps);
    let receipt_missing = steps
        .iter()
        .find(|step| {
            step["run"]
                .as_str()
                .is_some_and(|run| run.contains("receipt-missing"))
        })
        .expect("receipt-missing invocation");
    let run = receipt_missing["run"].as_str().expect("shell script");
    assert!(run.contains("record_publisher_repository"));
    assert!(run.contains("record_publisher_workflow_path"));
    assert!(run.contains("record_publisher_ref"));
}

#[trace("TC-1125", "FR-070-AC-14", "FR-081-AC-10")]
#[test]
fn tag_receipt_workflow_has_the_declared_trigger_jobs_permissions_and_trigger_inputs() {
    let reusable = workflow("tag-receipt.yml");
    let jobs = &reusable["jobs"];
    assert_tag_receipt_build_job(jobs);
    assert_tag_receipt_runtime_jobs(jobs);
    assert_tag_receipt_trigger_inputs(&reusable);

    let caller = workflow("tag-receipt-push.yml");
    assert_eq!(caller["on"]["push"]["tags"][0], "**");
    let receipt = &caller["jobs"]["receipt"];
    assert_eq!(receipt["uses"], "./.github/workflows/tag-receipt.yml");
    assert_eq!(receipt["with"]["tag"], "${{ github.ref_name }}");
    assert_permission(
        receipt,
        &serde_json::json!({"contents":"write", "id-token":"write"}),
        "tag receipt caller",
    );
}

fn assert_tag_receipt_build_job(jobs: &Value) {
    assert_eq!(jobs.as_object().map(serde_json::Map::len), Some(3));
    assert_eq!(jobs["record"]["needs"], "build");
    assert_eq!(jobs["notes"]["needs"], json!(["build", "record"]));
    let build_steps = jobs["build"]["steps"].as_array().expect("build steps");
    assert_actions_are_structural(build_steps);
    let source_checkout = build_steps
        .iter()
        .find(|step| {
            step["uses"]
                .as_str()
                .is_some_and(|value| value.starts_with("actions/checkout@"))
        })
        .expect("default source checkout");
    assert_eq!(
        source_checkout["with"]["repository"],
        "agent-ix/engineering-assurance"
    );
    assert_eq!(source_checkout["with"]["ref"], "main");
    assert_eq!(source_checkout["with"]["path"], "source");
    let build = build_steps
        .iter()
        .find(|step| {
            step["run"]
                .as_str()
                .is_some_and(|run| run.contains("cargo build"))
        })
        .expect("source build step");
    let build_script = build["run"].as_str().expect("build script");
    assert!(
        build_script
            .contains("cargo build --locked --release --features full --bin engineering-assurance")
    );
    assert!(build_script.contains("tag-receipt-record --help"));
    assert!(build_script.contains("tag-receipt-note --help"));
    assert!(build_steps.iter().any(|step| {
        step["uses"]
            .as_str()
            .is_some_and(|value| value.starts_with("actions/upload-artifact@"))
            && step["with"]["name"] == "tag-receipt-cli"
    }));
}

fn assert_tag_receipt_runtime_jobs(jobs: &Value) {
    assert_permission(
        &jobs["record"],
        &serde_json::json!({"contents":"read", "id-token":"write"}),
        "tag receipt record",
    );
    assert_permission(
        &jobs["notes"],
        &serde_json::json!({"contents":"write"}),
        "tag receipt notes",
    );
    assert_eq!(jobs["notes"]["needs"], json!(["build", "record"]));
    for job in ["record", "notes"] {
        let steps = jobs[job]["steps"].as_array().expect("steps");
        assert_actions_are_structural(steps);
        assert!(
            steps.iter().any(|step| {
                step["uses"]
                    .as_str()
                    .is_some_and(|value| value.starts_with("actions/download-artifact@"))
                    && step["with"]["name"] == "tag-receipt-cli"
            }),
            "{job} executes the CLI artifact built from checked-out source"
        );
        assert!(
            steps
                .iter()
                .all(|step| step["run"].as_str().is_none_or(|run| {
                    !run.contains("cargo ") && !run.contains("python3") && !run.contains("npm ")
                })),
            "{job} runs no repository gate, script or package"
        );
    }
    assert_record_job_is_isolated_for_tags(jobs);
}

fn assert_tag_receipt_trigger_inputs(reusable: &Value) {
    let jobs = &reusable["jobs"];
    assert_eq!(
        reusable["on"].as_object().map(serde_json::Map::len),
        Some(1)
    );
    assert_eq!(
        reusable["on"]["workflow_call"]["inputs"]["tag"]["type"],
        "string"
    );
    assert!(!format!("{reusable}").to_lowercase().contains("agent-run"));

    // A `tag` input that is not the triggering tag fails before any signing step.
    let record_steps = jobs["record"]["steps"].as_array().expect("record steps");
    let check = record_steps[0]["run"].as_str().expect("check step");
    assert!(check.contains("tag_receipt_input_mismatch"));
    assert!(check.contains("$INPUT_TAG\" != \"$TRIGGER_TAG"));
    assert!(record_steps[1..].iter().all(|step| {
        step["run"]
            .as_str()
            .is_none_or(|run| !run.contains("tag_receipt_input_mismatch"))
    }),);
    // The tag name, tag object id and push time come from the trigger context.
    let sign = record_steps
        .iter()
        .find(|step| {
            step["run"]
                .as_str()
                .is_some_and(|run| run.contains("tag-receipt-record --request"))
        })
        .expect("signing step");
    assert_eq!(sign["env"]["TAG_NAME"], "${{ github.ref_name }}");
    assert_eq!(sign["env"]["TAG_OBJECT"], "${{ github.event.after }}");
    assert_eq!(
        sign["env"]["PUSHED_AT_UNIX"],
        "${{ github.event.repository.pushed_at }}"
    );
}

fn assert_record_job_is_isolated_for_tags(jobs: &Value) {
    let record_steps = jobs["record"]["steps"].as_array().expect("record steps");
    assert!(
        record_steps.iter().all(|step| {
            !step["uses"].as_str().is_some_and(|reference| {
                reference.starts_with("actions/checkout@") || reference.starts_with("./")
            })
        }),
        "the identity-bearing tag receipt record job does not check out tag-controlled source"
    );
}

#[trace("TC-1126", "FR-081-AC-1")]
#[test]
fn checked_out_source_cli_reports_its_package_version_and_exposes_both_tag_receipt_commands() {
    let cli = env!("CARGO_BIN_EXE_engineering-assurance");
    let version = Command::new(cli)
        .arg("--version")
        .output()
        .expect("built source CLI starts");
    assert!(version.status.success(), "--version: {version:?}");
    let version = String::from_utf8(version.stdout).expect("version is UTF-8");
    assert!(
        version.contains(env!("CARGO_PKG_VERSION")),
        "CLI version must come from this checked-out package: {version:?}"
    );

    for subcommand in ["tag-receipt-record", "tag-receipt-note"] {
        let output = Command::new(cli)
            .args([subcommand, "--help"])
            .output()
            .expect("built source CLI starts");
        assert!(output.status.success(), "{subcommand} --help: {output:?}");
        let help = String::from_utf8(output.stdout).expect("help is UTF-8");
        assert!(help.contains(subcommand), "{subcommand}: {help}");
    }
}
