// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! One release version, stated in several places, must be one version.
//!
//! The crate, the Quoin module manifest, the compatibility matrix's own pin,
//! the three plugin manifests, `package.json` and `setup.cfg` each state which
//! release this checkout is. They drifted once (module at 0.5.0, everything
//! else at 0.4.1) and every `compatibility-observe` run withheld the gate as a
//! result.

use ix_trace_rs::trace;

const MODULE_MANIFEST: &str = include_str!("../engineering_assurance/manifest.yaml");
const MATRIX: &str = include_str!("../engineering_assurance/compatibility-matrix.json");
const SETUP_CFG: &str = include_str!("../setup.cfg");
const JSON_MANIFESTS: [(&str, &str); 4] = [
    ("package.json", include_str!("../package.json")),
    (
        ".claude-plugin/plugin.json",
        include_str!("../.claude-plugin/plugin.json"),
    ),
    (
        ".codex-plugin/plugin.json",
        include_str!("../.codex-plugin/plugin.json"),
    ),
    (
        ".github/plugin/plugin.json",
        include_str!("../.github/plugin/plugin.json"),
    ),
];

#[trace("TC-191", "FR-012-AC-12")]
#[test]
fn crate_module_matrix_plugins_and_package_state_the_same_version() {
    let crate_version = env!("CARGO_PKG_VERSION");

    let module_version = MODULE_MANIFEST
        .lines()
        .find_map(|line| line.strip_prefix("version:"))
        .map(|value| value.trim().trim_matches(['\'', '"']))
        .expect("the module manifest must declare a top-level version");
    assert_eq!(
        module_version, crate_version,
        "manifest.yaml and Cargo.toml disagree"
    );

    let setup_version = SETUP_CFG
        .lines()
        .find_map(|line| line.strip_prefix("version"))
        .and_then(|rest| rest.trim_start().strip_prefix('='))
        .map(str::trim)
        .expect("setup.cfg must declare a version");
    assert_eq!(
        setup_version, crate_version,
        "setup.cfg and Cargo.toml disagree"
    );

    let matrix: serde_json::Value = serde_json::from_str(MATRIX).expect("the matrix must be JSON");
    let matrix_pin = matrix["components"]
        .as_array()
        .expect("the matrix lists its components")
        .iter()
        .find(|component| component["name"] == "engineering-assurance")
        .and_then(|component| component["version"].as_str())
        .expect("the matrix pins engineering-assurance");
    assert_eq!(
        matrix_pin, crate_version,
        "the matrix's engineering-assurance pin and Cargo.toml disagree"
    );

    for (path, text) in JSON_MANIFESTS {
        let value: serde_json::Value = serde_json::from_str(text)
            .unwrap_or_else(|error| panic!("{path} is not JSON: {error}"));
        assert_eq!(
            value["version"].as_str(),
            Some(crate_version),
            "{path} and Cargo.toml disagree"
        );
    }
}
