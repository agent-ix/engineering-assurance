// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Black-box coverage for the explicit-root manifest qualification command.

use std::{fs, process::Command};

use ix_trace_rs::trace;
use serde_json::Value;
use tempfile::TempDir;

fn fixture() -> (TempDir, TempDir) {
    let repository = TempDir::new().expect("repository fixture must be creatable");
    let module = TempDir::new().expect("module fixture must be creatable");
    let package = repository.path().join("engineering_assurance");
    fs::create_dir_all(package.join("schemas"))
        .expect("schema fixture directory must be creatable");
    fs::create_dir_all(package.join("skeletons"))
        .expect("skeleton fixture directory must be creatable");
    fs::write(
        package.join("manifest.yaml"),
        "name: engineering-assurance\nversion: 0.2.1\nartifact_types:\n  - name: sample\n    frontmatter_schema_ref: schemas/sample.schema.json\n    allowed_links: [supports]\n    body_extraction:\n      yield_pattern:\n        match:\n          body:\n            after_heading: Required\n            required: true\n",
    )
    .expect("manifest fixture must be writable");
    fs::write(
        package.join("schemas/sample.schema.json"),
        b"{\"type\":\"object\"}",
    )
    .expect("artifact schema fixture must be writable");
    fs::write(
        package.join("skeletons/sample.md"),
        "---\nkind: sample\n---\n## Required\n",
    )
    .expect("skeleton fixture must be writable");
    fs::write(
        module.path().join("module-manifest.schema.json"),
        b"{\"type\":\"object\"}",
    )
    .expect("module schema fixture must be writable");
    fs::write(
        module.path().join("manifest.yaml"),
        "edge_types:\n  supports: {}\n",
    )
    .expect("edge registry fixture must be writable");
    (repository, module)
}

#[test]
#[trace("TC-131", "FR-017-AC-8", "FR-014-AC-2")]
fn tc_131_cli_emits_one_versioned_machine_result_for_explicit_roots() {
    let (repository, module) = fixture();
    let output = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args([
            "manifest-validate",
            "--root",
            repository
                .path()
                .to_str()
                .expect("fixture root must be UTF-8"),
            "--module-root",
            module.path().to_str().expect("module root must be UTF-8"),
        ])
        .output()
        .expect("manifest command must be runnable");

    assert!(output.status.success());
    assert_eq!(output.stderr, b"");
    let stdout = std::str::from_utf8(&output.stdout).expect("result must be UTF-8");
    assert_eq!(stdout.lines().count(), 1);
    let result: Value = serde_json::from_str(stdout).expect("result must be JSON");
    assert_eq!(
        result.get("protocol").and_then(Value::as_str),
        Some("engineering-assurance.manifest-validate/v1")
    );
    assert_eq!(
        result.get("capability").and_then(Value::as_str),
        Some("manifest-validate")
    );
    assert_eq!(
        result.get("outcome").and_then(Value::as_str),
        Some("accepted")
    );
}
