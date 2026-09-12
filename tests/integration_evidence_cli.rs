// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Direct command coverage for the Rust integration-evidence host.

use std::{fs, process::Command};

use ix_trace_rs::trace;
use tempfile::TempDir;

fn fixture_root(marker: Option<&str>) -> TempDir {
    let root = TempDir::new().expect("fixture root must be available");
    fs::create_dir_all(root.path().join("spec")).expect("spec directory must be creatable");
    fs::write(
        root.path().join("spec/tests.md"),
        marker.unwrap_or("# Test matrix\n| Status |\n| --- |\n| pass |\n"),
    )
    .expect("test matrix must be writable");
    root
}

fn quire_fixture(root: &TempDir) -> std::path::PathBuf {
    let quire = root.path().join("quire-fixture.sh");
    fs::write(
        &quire,
        "#!/bin/sh\nprintf '%s\\n' '{\"totals\":{\"backed\":92,\"total\":92},\"unbacked_rows\":[],\"status_lies\":[],\"untracked_symbols\":[],\"groups\":[{\"document\":\"spec/tests.md\",\"target\":\"test-case\",\"backed\":133,\"total\":133}],\"diagnostics\":[]}'\n",
    )
    .expect("quire fixture must be writable");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&quire, fs::Permissions::from_mode(0o755))
            .expect("quire fixture must be executable");
    }
    quire
}

#[test]
#[trace("TC-111", "FR-017-AC-3")]
fn tc_111_cli_accepts_typed_complete_traceability_only_evidence() {
    let root = fixture_root(None);
    let quire = quire_fixture(&root);
    let output = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args([
            "integration-evidence",
            "--root",
            root.path().to_str().expect("root must be UTF-8"),
            "--quire",
            quire.to_str().expect("quire fixture must be UTF-8"),
            "--traceability-only",
        ])
        .output()
        .expect("integration-evidence command must terminate");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout must be UTF-8"),
        "integration evidence passed: traceability complete\n"
    );
}

#[test]
#[trace("TC-111", "FR-017-AC-3")]
fn tc_111_cli_refuses_non_passing_matrix_markers_after_complete_coverage() {
    let root = fixture_root(Some("# Test matrix\n🚧\n"));
    let quire = quire_fixture(&root);
    let output = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args([
            "integration-evidence",
            "--root",
            root.path().to_str().expect("root must be UTF-8"),
            "--quire",
            quire.to_str().expect("quire fixture must be UTF-8"),
            "--traceability-only",
        ])
        .output()
        .expect("integration-evidence command must terminate");
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("integration_evidence_matrix_invalid")
    );
}

#[test]
#[trace("TC-111", "FR-017-AC-3")]
fn tc_111_cli_refuses_release_mode_without_an_immutable_source_revision() {
    let root = fixture_root(None);
    let quire = quire_fixture(&root);
    let workspace = TempDir::new().expect("workspace fixture must be available");
    let output = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args([
            "integration-evidence",
            "--root",
            root.path().to_str().expect("root must be UTF-8"),
            "--quire",
            quire.to_str().expect("quire fixture must be UTF-8"),
            "--artifact",
            "artifacts/aggregate.json",
            "--workspace-root",
            workspace.path().to_str().expect("workspace must be UTF-8"),
        ])
        .output()
        .expect("integration-evidence command must terminate");
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("integration_evidence_revision_unavailable"),
        "{stdout}"
    );
}
