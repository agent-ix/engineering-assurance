// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Root Cargo package boundary tests for FR-014.

use std::process::Command;

use ix_trace_rs::trace;

#[trace("TC-096", "FR-014-AC-1", "FR-014-CON-3", "StR-003-VC-1")]
#[test]
fn tc_096_root_package_exports_library_and_native_cli() {
    assert_eq!(engineering_assurance::PACKAGE_NAME, "engineering-assurance");

    let output = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .arg("--version")
        .output()
        .expect("the Cargo-built CLI must be executable");

    assert!(output.status.success());
    let expected = format!(
        "{} {}\n",
        engineering_assurance::PACKAGE_NAME,
        engineering_assurance::PACKAGE_VERSION
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("version output must be UTF-8"),
        expected
    );
    assert!(output.stderr.is_empty());
}

/// Run the CLI with `args`, feeding `stdin`, and return its output.
fn run_cli(args: &[&str], stdin: &[u8]) -> std::process::Output {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the Cargo-built CLI must start");
    child
        .stdin
        .take()
        .expect("piped stdin must exist")
        .write_all(stdin)
        .expect("stdin must be writable");
    child.wait_with_output().expect("CLI must terminate")
}

#[trace("TC-098", "FR-014-AC-2")]
#[allow(clippy::too_many_lines)]
#[test]
fn tc_098_every_machine_facing_command_emits_one_versioned_json_result_on_stdout() {
    let missing = std::env::temp_dir().join("engineering-assurance-absent-root");
    let missing = missing.to_str().expect("temp path must be UTF-8");
    let cases: [(&str, Vec<&str>, &[u8]); 14] = [
        (
            "agent-evals-provider",
            vec!["agent-evals-provider", "--root", missing],
            b"{not json",
        ),
        ("onboarding", vec!["onboarding"], b"{not json"),
        (
            "workflow-invariants",
            vec!["workflow-invariants"],
            b"{not json",
        ),
        ("workflow-host", vec!["workflow-host"], b"{not json"),
        (
            "producer-execute",
            vec!["producer-execute", "--request", "{not json"],
            b"",
        ),
        (
            "scope-footprint",
            vec!["scope-footprint", "--request", "{not json"],
            b"",
        ),
        (
            "signature-verify",
            vec!["signature-verify", "--request", "{not json"],
            b"",
        ),
        (
            "approval-verify",
            vec!["approval-verify", "--request", "{not json"],
            b"",
        ),
        (
            "content-rights-tree",
            vec!["content-rights-tree", "--root", missing],
            b"",
        ),
        (
            "integration-evidence",
            vec!["integration-evidence", "--root", missing],
            b"",
        ),
        (
            "package-audit",
            vec!["package-audit", "--root", missing],
            b"",
        ),
        (
            "package-lifecycle",
            vec!["package-lifecycle", "stage", "--root", missing],
            b"",
        ),
        (
            "manifest-validate",
            vec![
                "manifest-validate",
                "--root",
                missing,
                "--schema-root",
                missing,
                "--registry-root",
                missing,
            ],
            b"",
        ),
        (
            "evaluation-aggregate",
            vec![
                "evaluation-aggregate",
                "--root",
                missing,
                "--workspace-root",
                missing,
                "--report",
                missing,
                "--output",
                missing,
            ],
            b"",
        ),
    ];
    for (capability, args, stdin) in cases {
        let output = run_cli(&args, stdin);
        assert!(!output.status.success(), "{capability} must refuse");
        assert!(
            !output.stderr.is_empty(),
            "{capability} must explain on stderr"
        );
        let stdout = std::str::from_utf8(&output.stdout).expect("stdout must be UTF-8");
        assert_eq!(stdout.lines().count(), 1, "{capability} stdout: {stdout:?}");
        assert!(
            stdout.ends_with('\n'),
            "{capability} stdout must end with a newline"
        );
        let result: serde_json::Value = serde_json::from_str(stdout)
            .unwrap_or_else(|_| panic!("{capability} stdout is not JSON"));
        assert_eq!(
            result["protocol"], "engineering-assurance.error/v1",
            "{capability}"
        );
        assert_eq!(result["capability"], capability);
    }
}
