// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! One release version, stated in three places, must be one version.
//!
//! The crate version, the Quoin module manifest and the compatibility matrix's
//! own pin each state which release this checkout is. They drifted once (module
//! at 0.5.0, crate and matrix at 0.4.1) and every `compatibility-observe` run
//! withheld the gate as a result.

use ix_trace_rs::trace;

const MODULE_MANIFEST: &str = include_str!("../engineering_assurance/manifest.yaml");
const MATRIX: &str = include_str!("../engineering_assurance/compatibility-matrix.json");

#[trace("TC-130", "FR-012-AC-11")]
#[test]
fn crate_module_and_matrix_pin_state_the_same_version() {
    let crate_version = env!("CARGO_PKG_VERSION");

    let module_version = MODULE_MANIFEST
        .lines()
        .find_map(|line| line.strip_prefix("version:"))
        .map(|value| value.trim().trim_matches(['\'', '"']))
        .expect("the module manifest must declare a top-level version");

    let matrix: serde_json::Value = serde_json::from_str(MATRIX).expect("the matrix must be JSON");
    let matrix_pin = matrix["components"]
        .as_array()
        .expect("the matrix lists its components")
        .iter()
        .find(|component| component["name"] == "engineering-assurance")
        .and_then(|component| component["version"].as_str())
        .expect("the matrix pins engineering-assurance");

    assert_eq!(
        module_version, crate_version,
        "manifest.yaml and Cargo.toml disagree"
    );
    assert_eq!(
        matrix_pin, crate_version,
        "the matrix's engineering-assurance pin and Cargo.toml disagree"
    );
}
