// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Machine-boundary and confined-publication tests for Rust onboarding.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use engineering_assurance::onboarding::{REQUEST_PROTOCOL, RESULT_PROTOCOL};
use ix_trace_rs::trace;
use serde_json::{Value, json};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(name: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "engineering-assurance-{name}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("unique test directory must be creatable");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn module_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("engineering_assurance")
        .canonicalize()
        .expect("installed module fixture must resolve")
}

fn request(root: &Path, changes: &Value) -> Value {
    let mut request = json!({
        "protocol": REQUEST_PROTOCOL,
        "repository_root": root,
        "module_root": module_root(),
        "quire_executable": "quire",
        "decision_boundary": "one fictional candidate revision",
        "decision_owner": "release-owner",
    });
    request
        .as_object_mut()
        .expect("request fixture must be an object")
        .extend(
            changes
                .as_object()
                .expect("request changes must be an object")
                .clone(),
        );
    request
}

fn run(request: &Value) -> Output {
    run_bytes(&serde_json::to_vec(request).expect("request fixture must serialize"))
}

fn run_bytes(encoded: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .arg("onboarding")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the Cargo-built CLI must start");
    child
        .stdin
        .take()
        .expect("piped stdin must exist")
        .write_all(encoded)
        .expect("test request must be writable");
    child.wait_with_output().expect("CLI must terminate")
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().expect("fixture path must have a parent"))
        .expect("fixture parent must be creatable");
    fs::write(path, content).expect("fixture must be writable");
}

fn copy_skeleton(root: &Path, artifact_type: &str, target: &str) -> PathBuf {
    let destination = root.join(target);
    fs::create_dir_all(
        destination
            .parent()
            .expect("artifact fixture must have a parent"),
    )
    .expect("artifact parent must be creatable");
    fs::copy(
        module_root()
            .join("skeletons")
            .join(format!("{artifact_type}.md")),
        &destination,
    )
    .expect("installed skeleton must be copyable");
    destination
}

fn python_inventory(root: &Path) -> Value {
    let script = r#"
import json
import sys
from pathlib import Path
from engineering_assurance.onboarding import inventory_repository

print(json.dumps(inventory_repository(Path(sys.argv[1]), quire_bin="quire").to_dict(), separators=(",", ":")))
"#;
    let output = Command::new("python3")
        .args(["-c", script])
        .arg(root)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("retained Python onboarding reference must start");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("reference inventory must be JSON")
}

fn result(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("CLI stdout must contain one JSON value")
}

fn assert_no_stages(root: &Path) {
    let stages = fs::read_dir(root.join("spec"))
        .expect("spec fixture must remain readable")
        .collect::<Result<Vec<_>, _>>()
        .expect("spec entries must remain readable")
        .into_iter()
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".staged"))
        .collect::<Vec<_>>();
    assert!(stages.is_empty(), "staged files remain after refusal");
}

#[trace("TC-105", "FR-016-AC-1", "FR-016-CON-4")]
#[test]
fn tc_105_cli_inventory_and_reuse_match_the_retained_reference() {
    let repository = TestDirectory::new("inventory");
    copy_skeleton(repository.path(), "AssuranceProfile", "spec/AP-001.md");
    copy_skeleton(repository.path(), "MeasurementPlan", "spec/MP-001.md");
    write(
        &repository.path().join("decisions/release.md"),
        "---\ntype: DecisionRecord\n---\n# Decision\n",
    );
    write(
        &repository.path().join(".github/workflows/assurance.yml"),
        "name: fictional producer\n",
    );
    write(
        &repository.path().join("spec/evidence/observation.json"),
        "{}\n",
    );
    write(
        &repository.path().join("spec/AP-malformed.md"),
        "---\n- not-a-mapping\n---\n# Malformed\n",
    );
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        repository.path().join("spec/AP-001.md"),
        repository.path().join("spec/AP-link.md"),
    )
    .expect("fixture symlink must be creatable");

    let existing = repository.path().join("spec/AP-001.md");
    let before = fs::read(&existing).expect("existing artifact must be readable");
    let output = run(&request(
        repository.path(),
        &json!({"requested_artifact": "AssuranceProfile"}),
    ));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    assert!(!output.stdout[..output.stdout.len() - 1].contains(&b'\n'));
    let result = result(&output);
    assert_eq!(result["protocol"], RESULT_PROTOCOL);
    assert_eq!(result["status"], "reuse");
    assert_eq!(result["artifact_path"], "spec/AP-001.md");
    assert_eq!(result["inventory"], python_inventory(repository.path()));
    assert_eq!(
        fs::read(existing).expect("existing artifact must remain readable"),
        before
    );
}

#[trace("TC-105", "FR-016-AC-1", "FR-016-CON-4")]
#[test]
fn tc_105_cli_authors_one_quire_validated_confined_artifact() {
    let repository = TestDirectory::new("author");
    let target = repository.path().join("spec/AP-002.md");
    let output = run(&request(
        repository.path(),
        &json!({
            "requested_artifact": "AssuranceProfile",
            "justification": "material fictional decision",
            "target": "spec/AP-002.md",
            "frontmatter": {
                "id": "AP-002",
                "title": "Fictional candidate decision profile",
                "owner": "release-owner",
            },
        }),
    ));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result = result(&output);
    assert_eq!(result["protocol"], RESULT_PROTOCOL);
    assert_eq!(result["status"], "authored");
    assert_eq!(result["artifact_path"], "spec/AP-002.md");
    assert!(target.is_file());
    let validation = Command::new("quire")
        .args(["validate", "--module"])
        .arg(module_root())
        .arg("--strict")
        .arg(&target)
        .current_dir(repository.path())
        .output()
        .expect("Quire must start");
    assert!(
        validation.status.success(),
        "{}",
        String::from_utf8_lossy(&validation.stderr)
    );
    assert_no_stages(repository.path());
}

#[trace("TC-105", "FR-016-AC-1", "FR-016-CON-4")]
#[test]
fn tc_105_cli_refuses_invalid_or_escaping_publication_without_bytes() {
    let repository = TestDirectory::new("refusals");
    fs::create_dir(repository.path().join("spec")).expect("spec fixture must be creatable");

    let invalid = run(&request(
        repository.path(),
        &json!({
            "requested_artifact": "AssuranceProfile",
            "justification": "material fictional decision",
            "target": "spec/AP-invalid.md",
            "frontmatter": {"owner": null},
        }),
    ));
    assert_eq!(invalid.status.code(), Some(2));
    assert_eq!(result(&invalid)["code"], "onboarding_publication_failed");
    assert!(!repository.path().join("spec/AP-invalid.md").exists());
    assert_no_stages(repository.path());

    let absolute = repository.path().join("AP-absolute.md");
    for target in [
        "../AP-escape.md".to_owned(),
        absolute.to_string_lossy().into_owned(),
    ] {
        let output = run(&request(
            repository.path(),
            &json!({
                "requested_artifact": "AssuranceProfile",
                "justification": "material fictional decision",
                "target": &target,
                "frontmatter": {"id": "AP-refused"},
            }),
        ));
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(result(&output)["code"], "onboarding_target_invalid");
    }
    assert!(!absolute.exists());

    let existing = repository.path().join("spec/AP-existing.md");
    fs::write(&existing, b"sentinel\n").expect("existing fixture must be writable");
    let output = run(&request(
        repository.path(),
        &json!({
            "requested_artifact": "AssuranceProfile",
            "justification": "material fictional decision",
            "target": "spec/AP-existing.md",
            "frontmatter": {"id": "AP-existing"},
        }),
    ));
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(result(&output)["code"], "onboarding_target_invalid");
    assert_eq!(fs::read(&existing).unwrap(), b"sentinel\n");
    assert_no_stages(repository.path());
}

#[cfg(unix)]
#[trace("TC-105", "FR-016-AC-1", "FR-016-CON-4")]
#[test]
fn tc_105_cli_refuses_a_symlink_target_escape() {
    let repository = TestDirectory::new("symlink-root");
    let outside = TestDirectory::new("symlink-outside");
    std::os::unix::fs::symlink(outside.path(), repository.path().join("escape"))
        .expect("fixture symlink must be creatable");
    let output = run(&request(
        repository.path(),
        &json!({
            "requested_artifact": "AssuranceProfile",
            "justification": "material fictional decision",
            "target": "escape/AP-escaped.md",
            "frontmatter": {"id": "AP-escaped"},
        }),
    ));
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(result(&output)["code"], "onboarding_target_invalid");
    assert!(!outside.path().join("AP-escaped.md").exists());
}

#[trace("TC-105", "FR-016-AC-1", "FR-016-CON-4")]
#[test]
fn tc_105_cli_preserves_unavailable_quire_and_rejects_bad_boundaries() {
    let repository = TestDirectory::new("quire-unavailable");
    copy_skeleton(repository.path(), "AssuranceProfile", "spec/AP-001.md");
    let unavailable = run(&request(
        repository.path(),
        &json!({
            "requested_artifact": "AssuranceProfile",
            "quire_executable": "/definitely/missing/quire",
        }),
    ));
    assert!(unavailable.status.success());
    let unavailable = result(&unavailable);
    assert_eq!(unavailable["status"], "needs-human-selection");
    assert_eq!(
        unavailable["inventory"]["assurance_artifacts"][0]["diagnostics"],
        json!(["quire-unavailable"])
    );

    let publication = TestDirectory::new("quire-publication-unavailable");
    fs::create_dir(publication.path().join("spec")).expect("publication fixture must be creatable");
    let unavailable = run(&request(
        publication.path(),
        &json!({
            "requested_artifact": "AssuranceProfile",
            "justification": "material fictional decision",
            "target": "spec/AP-002.md",
            "frontmatter": {"id": "AP-002"},
            "quire_executable": "/definitely/missing/quire",
        }),
    ));
    assert_eq!(unavailable.status.code(), Some(2));
    assert_eq!(
        result(&unavailable)["code"],
        "onboarding_publication_failed"
    );
    assert!(!publication.path().join("spec/AP-002.md").exists());
    assert_no_stages(publication.path());

    let incomplete_module = TestDirectory::new("incomplete-module");
    fs::create_dir(incomplete_module.path().join("skeletons"))
        .expect("incomplete module fixture must be creatable");
    let invalid_root = run(&request(
        repository.path(),
        &json!({"module_root": incomplete_module.path()}),
    ));
    assert_eq!(invalid_root.status.code(), Some(2));
    assert_eq!(
        result(&invalid_root)["code"],
        "onboarding_module_root_invalid"
    );

    let malformed = run_bytes(b"{not-json");
    assert_eq!(malformed.status.code(), Some(2));
    assert_eq!(result(&malformed)["code"], "onboarding_request_invalid");

    let oversized = run_bytes(&vec![b' '; 8 * 1024 * 1024 + 1]);
    assert_eq!(oversized.status.code(), Some(2));
    assert_eq!(result(&oversized)["code"], "onboarding_request_invalid");

    let unsupported = run(&request(
        repository.path(),
        &json!({"protocol": "engineering-assurance.onboarding/v2"}),
    ));
    assert_eq!(unsupported.status.code(), Some(2));
    assert_eq!(
        result(&unsupported)["code"],
        "unsupported_onboarding_protocol"
    );
}

#[trace("TC-105", "FR-016-AC-1", "FR-016-CON-4")]
#[test]
fn tc_105_cli_keeps_ambiguous_yaml_identity_out_of_artifact_selection() {
    let repository = TestDirectory::new("ambiguous-yaml");
    write(
        &repository.path().join("spec/AP-duplicate.md"),
        "---\ntype: DecisionRecord\ntype: AssuranceProfile\n---\n# Duplicate\n",
    );
    write(
        &repository.path().join("spec/AP-merge.md"),
        "---\ndefaults: &defaults\n  type: AssuranceProfile\n<<: *defaults\n---\n# Merge\n",
    );
    let output = run(&request(repository.path(), &json!({})));
    assert!(output.status.success());
    let result = result(&output);
    assert_eq!(result["status"], "no-applicable-work");
    assert_eq!(result["inventory"]["assurance_artifacts"], json!([]));
    assert_eq!(
        result["inventory"]["unresolved_inputs"],
        json!([
            "malformed-frontmatter:spec/AP-duplicate.md",
            "malformed-frontmatter:spec/AP-merge.md",
        ])
    );
}
