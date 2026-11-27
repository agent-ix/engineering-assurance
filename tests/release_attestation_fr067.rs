// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Focused behavioral checks for release-attestation workflow policy.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

use ix_trace_rs::trace;
use serde_json::Value;

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn workflow() -> Value {
    let path = repository_root().join(".github/workflows/release-attestation.yml");
    let bytes = fs::read(path).expect("release-attestation workflow exists");
    yaml_serde::from_slice(&bytes).expect("release-attestation workflow parses")
}

fn git(repo: &Path, args: &[&str]) -> Output {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("git starts");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn release_verify_script() -> String {
    let reusable = workflow();
    let steps = reusable["jobs"]["verify"]["steps"]
        .as_array()
        .expect("verify steps");
    steps
        .iter()
        .find(|step| step["name"] == "Verify the signed release tag against protected policy")
        .expect("release-verify workflow step")["run"]
        .as_str()
        .expect("release-verify script")
        .to_owned()
}

#[trace("TC-539", "FR-067-AC-3")]
#[test]
fn release_verification_reads_allowed_signers_from_the_protected_base() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let protected_base = temp.path().join("protected-base");
    fs::create_dir(&protected_base).expect("protected-base directory");
    git(&protected_base, &["init", "-q"]);
    git(&protected_base, &["config", "user.name", "Fixture"]);
    git(
        &protected_base,
        &["config", "user.email", "fixture@example.test"],
    );
    fs::write(protected_base.join("allowed_signers"), "protected signer\n")
        .expect("protected policy");
    git(&protected_base, &["add", "allowed_signers"]);
    git(&protected_base, &["commit", "-q", "-m", "protected policy"]);

    let source = temp.path().join("source");
    fs::create_dir(&source).expect("source directory");
    git(&source, &["init", "-q"]);
    fs::write(source.join("allowed_signers"), "tag-commit signer\n").expect("tag policy");
    assert_eq!(
        fs::read(source.join("allowed_signers")).expect("tag policy contents"),
        b"tag-commit signer\n"
    );

    let runner_temp = temp.path().join("runner-temp");
    let tooling = runner_temp.join("ea-tooling");
    fs::create_dir_all(&tooling).expect("tooling directory");
    let observed_policy = temp.path().join("observed-policy");
    let executable = tooling.join("engineering-assurance");
    fs::write(
        &executable,
        format!(
            "#!/usr/bin/env python3\n\
             import json, pathlib, sys\n\
             request = json.loads(sys.argv[sys.argv.index('--request') + 1])\n\
             pathlib.Path({:?}).write_bytes(pathlib.Path(request['allowed_signers']).read_bytes())\n\
             print('{{\"status\":\"released\"}}')\n",
            observed_policy
        ),
    )
    .expect("fake verifier");
    let mut permissions = fs::metadata(&executable)
        .expect("fake verifier metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions).expect("fake verifier executable");

    let output = Command::new("bash")
        .arg("-euo")
        .arg("pipefail")
        .arg("-c")
        .arg(release_verify_script())
        .current_dir(temp.path())
        .env("RUNNER_TEMP", &runner_temp)
        .env("INPUT_TAG", "v1.2.3")
        .env("RECORD_PATH", "release/record.md")
        .env("ALLOWED_SIGNERS_PATH", "allowed_signers")
        .env("GITHUB_WORKSPACE", temp.path())
        .output()
        .expect("release-verify workflow step starts");
    assert!(
        output.status.success(),
        "workflow step failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(observed_policy).expect("policy received by verifier"),
        b"protected signer\n"
    );
}

#[trace("TC-538", "FR-067-AC-2")]
#[test]
fn a_non_released_tag_status_fails_before_the_attestation_job() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let protected_base = temp.path().join("protected-base");
    fs::create_dir(&protected_base).expect("protected-base directory");
    git(&protected_base, &["init", "-q"]);
    git(&protected_base, &["config", "user.name", "Fixture"]);
    git(
        &protected_base,
        &["config", "user.email", "fixture@example.test"],
    );
    fs::write(protected_base.join("allowed_signers"), "fixture signer\n")
        .expect("protected policy");
    git(&protected_base, &["add", "allowed_signers"]);
    git(&protected_base, &["commit", "-q", "-m", "protected policy"]);

    let runner_temp = temp.path().join("runner-temp");
    let tooling = runner_temp.join("ea-tooling");
    fs::create_dir_all(&tooling).expect("tooling directory");
    let executable = tooling.join("engineering-assurance");
    fs::write(
        &executable,
        "#!/bin/sh\nprintf '%s\\n' '{\"status\":\"not_released\",\"reasons\":[\"untrusted_signer\"]}'\n",
    )
    .expect("fixture release verifier");
    let mut permissions = fs::metadata(&executable)
        .expect("fixture verifier metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions).expect("fixture verifier executable");

    let reusable = workflow();
    let steps = reusable["jobs"]["verify"]["steps"]
        .as_array()
        .expect("verify steps");
    let verify_index = steps
        .iter()
        .position(|step| step["name"] == "Verify the signed release tag against protected policy")
        .expect("release verification step");
    let upload_index = steps
        .iter()
        .position(|step| step["name"] == "Upload verified attestation inputs")
        .expect("attestation input upload step");
    assert!(
        verify_index < upload_index,
        "tag verification precedes attestation inputs"
    );
    assert_eq!(reusable["jobs"]["attest"]["needs"], "verify");

    let script = release_verify_script();
    let output = Command::new("bash")
        .args(["-euo", "pipefail", "-c", script.as_str()])
        .current_dir(temp.path())
        .env("RUNNER_TEMP", &runner_temp)
        .env("INPUT_TAG", "v1.2.3")
        .env("RECORD_PATH", "release/record.md")
        .env("ALLOWED_SIGNERS_PATH", "allowed_signers")
        .env("GITHUB_WORKSPACE", temp.path())
        .output()
        .expect("release verification workflow step starts");
    assert_eq!(output.status.code(), Some(1));
    let rendered = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        rendered.contains("::error::release_verify_status=not_released"),
        "the failed workflow reports the release-verify status: {rendered}"
    );
}
