// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Real package-audit host-boundary coverage.

use std::{path::Path, process::Command};

use engineering_assurance::package_audit::PACKAGE_AUDIT_PROTOCOL;
use ix_trace_rs::trace;
use serde_json::Value;

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
    let result: Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be one JSON result");
    assert_eq!(result["protocol"], PACKAGE_AUDIT_PROTOCOL);
    assert_eq!(result["outcome"], "accepted");
    assert!(result["wheel_files"].as_u64().unwrap() > 0);
    assert!(result["npm_files"].as_u64().unwrap() > 0);
    assert!(result["installed_canonical_files"].as_u64().unwrap() > 0);
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
    let result: Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be one typed error");
    assert_eq!(result["capability"], "package-audit");
    assert_eq!(result["code"], "package_audit_root_invalid");
}

#[test]
#[trace("TC-112", "FR-017-AC-4", "FR-017-CON-2")]
fn tc_112_package_audit_make_target_is_exact_rust_dispatch() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
    assert_eq!(
        makefile
            .matches("cargo +1.98.1 run --locked --quiet -- package-audit --root .")
            .count(),
        1
    );
    assert!(!makefile.contains("python3 scripts/audit_packages.py"));
    assert!(!makefile.contains("$(PYTHON) scripts/audit_packages.py"));
}
