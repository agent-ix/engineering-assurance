// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Root Cargo package boundary tests for FR-014.

use std::process::Command;

use ix_trace_rs::trace;

#[trace("TC-096", "FR-014-AC-1")]
#[test]
fn tc_096_root_package_exports_library_and_native_cli() {
    assert_eq!(engineering_assurance::PACKAGE_NAME, "engineering-assurance");
    assert_eq!(engineering_assurance::PACKAGE_VERSION, "0.1.0");

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

#[trace("TC-116", "NFR-005-AC-1")]
#[test]
fn tc_116_package_declares_the_qualified_rust_version() {
    assert_eq!(env!("CARGO_PKG_RUST_VERSION"), "1.98.1");
}
