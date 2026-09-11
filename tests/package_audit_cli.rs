// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Real package-audit host-boundary coverage.

use std::{path::Path, process::Command};

use engineering_assurance::package_audit::PACKAGE_AUDIT_PROTOCOL;
use ix_trace_rs::trace;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptedResult {
    protocol: String,
    outcome: String,
    wheel_files: u64,
    npm_files: u64,
    installed_canonical_files: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorResult {
    protocol: String,
    capability: String,
    code: String,
    message: String,
}

fn run(root: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["package-audit", "--root"])
        .arg(root)
        .output()
        .expect("the Cargo-built package-audit CLI must terminate")
}

#[test]
#[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
fn tc_111_real_wheel_and_npm_archives_install_and_agree() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = run(root);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: AcceptedResult =
        serde_json::from_slice(&output.stdout).expect("stdout must be one JSON result");
    assert_eq!(result.protocol, PACKAGE_AUDIT_PROTOCOL);
    assert_eq!(result.outcome, "accepted");
    assert!(result.wheel_files > 0);
    assert!(result.npm_files > 0);
    assert!(result.installed_canonical_files > 0);
    for staged in [
        "manifest.yaml",
        "compatibility-matrix.json",
        "contracts",
        "fixtures",
        "schemas",
        "skeletons",
    ] {
        assert!(
            !root.join(staged).exists(),
            "package audit must clean {staged}"
        );
    }
}

#[test]
#[trace("TC-111", "FR-017-AC-3")]
fn tc_111_non_repository_root_fails_before_package_construction() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let output = run(&root);
    assert_eq!(output.status.code(), Some(2));
    let result: ErrorResult =
        serde_json::from_slice(&output.stdout).expect("stdout must be one typed error");
    assert_eq!(result.protocol, "engineering-assurance.error/v1");
    assert_eq!(result.capability, "package-audit");
    assert_eq!(result.code, "package_audit_root_invalid");
    assert!(!result.message.is_empty());
}

#[test]
#[trace("TC-112", "FR-017-AC-4", "FR-017-CON-2")]
fn tc_112_package_audit_make_target_is_exact_rust_dispatch() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("make")
        .args(["--no-print-directory", "-n", "package-audit"])
        .current_dir(root)
        .output()
        .expect("make dry-run must terminate");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("make dry-run output must be UTF-8"),
        "CARGO_BUILD_JOBS=2 cargo +1.98.1 run --locked --quiet -- package-audit --root .\n"
    );
}

#[test]
#[trace("TC-019", "FR-003-AC-6", "TC-037", "FR-007-AC-3")]
fn tc_019_and_tc_037_installation_instructions_remain_distinct_and_ordered() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(root.join("engineering_assurance/INSTALL.md"))
        .expect("installation instructions must be readable");
    let headings = [
        "## Local-source module installation",
        "## Repository-source module installation",
        "## Local-source agent-plugin installation",
        "## Repository-source agent-plugin installation",
    ];
    let positions = headings.map(|heading| {
        text.find(heading)
            .unwrap_or_else(|| panic!("missing installation heading: {heading}"))
    });
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    let canonical = text
        .find("engineering_assurance/skills/assurance-onboarding")
        .expect("canonical installation path must be documented");
    let pilot = text
        .find("pilots/assurance-workflows")
        .expect("pilot compatibility path must be documented");
    assert!(canonical < pilot);
}
