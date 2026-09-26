// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-014: every capability feature compiles alone with default features off.

use std::{collections::BTreeSet, fs, path::Path, process::Command};

use ix_trace_rs::trace;

/// Feature names declared in the `[features]` table of `Cargo.toml`, read
/// directly from the manifest text (independent of `cargo metadata`, which the
/// Makefile uses), excluding `default` and the `full` umbrella.
fn declared_capability_features(manifest: &str) -> BTreeSet<String> {
    let mut in_features = false;
    let mut names = BTreeSet::new();
    for line in manifest.lines() {
        let trimmed = line.trim_end();
        if trimmed.starts_with('[') {
            in_features = trimmed == "[features]";
        } else if in_features
            && !line.starts_with([' ', '#', ']'])
            && let Some((name, _)) = line.split_once('=')
        {
            names.insert(name.trim().to_owned());
        }
    }
    names.remove("default");
    names.remove("full");
    names
}

/// The Makefile's feature list is exactly what `Cargo.toml` declares, and
/// `make rust-features` compiles the library with no default features and with
/// each capability feature alone; it fails if any of them stops compiling.
#[test]
#[trace("TC-190", "FR-014-AC-7")]
fn tc_190_each_capability_feature_compiles_alone() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("Cargo.toml");
    let declared = declared_capability_features(&manifest);
    assert!(
        declared.len() >= 5,
        "manifest parse found too few features: {declared:?}"
    );

    let listed = Command::new("make")
        .args(["--no-print-directory", "-s", "rust-feature-list"])
        .current_dir(root)
        .output()
        .expect("make rust-feature-list must launch");
    assert!(listed.status.success(), "rust-feature-list failed");
    let listed: BTreeSet<String> = String::from_utf8(listed.stdout)
        .expect("utf-8 feature list")
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        listed, declared,
        "Makefile feature list drifted from Cargo.toml"
    );

    let status = Command::new("make")
        .arg("rust-features")
        .current_dir(root)
        .status()
        .expect("make rust-features must launch");
    assert!(
        status.success(),
        "a capability feature no longer compiles alone"
    );
}
