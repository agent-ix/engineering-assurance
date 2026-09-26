// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Black-box coverage for the explicit-root manifest qualification command.

use std::{env, fs, path::Path, process::Command};

use ix_trace_rs::trace;
use serde_json::Value;
use tempfile::TempDir;

fn fixture() -> (TempDir, TempDir, TempDir) {
    let repository = TempDir::new().expect("repository fixture must be creatable");
    let schema_root = TempDir::new().expect("schema fixture must be creatable");
    let registry_root = TempDir::new().expect("registry fixture must be creatable");
    let package = repository.path().join("engineering_assurance");
    fs::create_dir_all(package.join("schemas"))
        .expect("schema fixture directory must be creatable");
    fs::create_dir_all(package.join("skeletons"))
        .expect("skeleton fixture directory must be creatable");
    fs::write(
        package.join("manifest.yaml"),
        "name: engineering-assurance\nversion: 0.5.0\nartifact_types:\n  - name: sample\n    frontmatter_schema_ref: schemas/sample.schema.json\n    allowed_links: [supports]\n    body_extraction:\n      yield_pattern:\n        match:\n          body:\n            after_heading: Required\n            required: true\n",
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
        schema_root.path().join("module-manifest.schema.json"),
        b"{\"type\":\"object\"}",
    )
    .expect("module schema fixture must be writable");
    fs::write(
        registry_root.path().join("manifest.yaml"),
        "edge_types:\n  supports: {}\n",
    )
    .expect("edge registry fixture must be writable");
    (repository, schema_root, registry_root)
}

#[test]
#[trace("TC-131", "FR-017-AC-8", "FR-014-AC-2")]
fn tc_131_cli_emits_one_versioned_machine_result_for_explicit_roots() {
    let (repository, schema_root, registry_root) = fixture();
    let output = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args([
            "manifest-validate",
            "--root",
            repository
                .path()
                .to_str()
                .expect("fixture root must be UTF-8"),
            "--schema-root",
            schema_root
                .path()
                .to_str()
                .expect("schema root must be UTF-8"),
            "--registry-root",
            registry_root
                .path()
                .to_str()
                .expect("registry root must be UTF-8"),
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

#[test]
#[trace("TC-131", "FR-017-AC-8", "FR-017-CON-3")]
fn tc_131_make_target_requires_and_forwards_the_explicit_schema_and_registry_roots() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("make")
        .args([
            "--no-print-directory",
            "-n",
            "manifest-validate",
            "MANIFEST_SCHEMA_ROOT=/schema-root",
            "MANIFEST_REGISTRY_ROOT=/registry-root",
        ])
        .current_dir(root)
        .output()
        .expect("make dry run must terminate");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("make output must be UTF-8");
    assert!(!stdout.contains("scripts/validate_manifest.py"));
    // GNU make and BSD make print continuation indentation differently in
    // dry-run output. The contract is the command and its forwarded values.
    for expected in [
        "manifest-validate",
        "--root .",
        "--schema-root \"/schema-root\"",
        "--registry-root \"/registry-root\"",
    ] {
        assert!(
            stdout.contains(expected),
            "make omitted {expected:?}: {stdout}"
        );
    }
}

/// Real conformance of this repository's own `engineering_assurance/manifest.yaml`
/// against filament-core-service's authoritative module-manifest schema and
/// spec-artifacts-iso's real edge registry. The fixture-backed parity suite
/// cannot assert this, because its schema and registry are locally authored.
/// This CLI takes only explicit operator-supplied roots; it never discovers,
/// fetches, or vendors a copy of either file, so this test needs an operator
/// (or a CI job wired separately from this one) to supply both roots
/// explicitly:
//
// Keep requirement and ticket ids out of this doc comment and the ignore
// reason: Quire reads id-shaped tokens here as trace tags, and the Rust
// marker reconciliation in tests/traceability_reconciliation.rs then refuses
// them as unbound. The trace attribute below is
// the only binding.
///
/// - `EA_MANIFEST_SCHEMA_ROOT`: a directory containing
///   `module-manifest.schema.json` (e.g. `filament-core-service`'s
///   `filament_core_service/schemas/`).
/// - `EA_MANIFEST_REGISTRY_ROOT`: a directory containing `manifest.yaml` with
///   the `edge_types` registry (e.g. spec-artifacts-iso's installed module
///   root, `spec_artifacts_iso/`).
///
/// Run with `cargo test --features full --test manifest_host_cli -- --ignored` after
/// exporting both.
#[test]
#[ignore = "requires EA_MANIFEST_SCHEMA_ROOT and EA_MANIFEST_REGISTRY_ROOT pointed at real, \
            operator-supplied checkouts; see the doc comment above"]
#[trace("TC-121", "FR-017-AC-7", "FR-017-AC-8")]
fn tc_121_real_manifest_conforms_to_the_authoritative_schema_and_edge_registry() {
    let schema_root = env::var("EA_MANIFEST_SCHEMA_ROOT").expect(
        "EA_MANIFEST_SCHEMA_ROOT must name a directory holding module-manifest.schema.json",
    );
    let registry_root = env::var("EA_MANIFEST_REGISTRY_ROOT").expect(
        "EA_MANIFEST_REGISTRY_ROOT must name a directory holding the edge-registry manifest.yaml",
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args([
            "manifest-validate",
            "--root",
            root.to_str().expect("repository root must be UTF-8"),
            "--schema-root",
            &schema_root,
            "--registry-root",
            &registry_root,
        ])
        .output()
        .expect("manifest command must be runnable");
    let stdout = std::str::from_utf8(&output.stdout).expect("result must be UTF-8");
    assert!(
        output.status.success(),
        "manifest-validate refused the real manifest against the real schema and registry: {stdout} {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: Value = serde_json::from_str(stdout).expect("result must be JSON");
    assert_eq!(
        result.get("outcome").and_then(Value::as_str),
        Some("accepted"),
        "engineering_assurance/manifest.yaml does not conform to FR-035: {result}"
    );
}
