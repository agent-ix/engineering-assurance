// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-014-AC-10: the dependency graph a consumer resolves carries no duplicate
//! crate versions, except the named residue.
//!
//! A consumer whose workspace bans duplicate versions (`cargo deny` with
//! `multiple-versions = "deny"`, as Quoin's does) cannot accept a capability
//! that drags a second version of a crate into its graph. This measures each
//! graph with `cargo tree`, so it needs no `cargo-deny`: normal and build
//! edges over every target, which is what `cargo deny check bans` counts by
//! default (dev-dependencies excluded).
//!
//! Deliberately needs no feature, so a bare `cargo test` runs it. Needs `make`
//! and `python3` on `PATH` for the capability-feature list, as
//! `feature_matrix` does.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    process::Command,
};

use ix_trace_rs::trace;

/// Crates that resolve at more than one version under `--features full`, all
/// reached through third-party subtrees this crate does not control. See
/// "Known duplicate crate versions" in the README for the owner of each. A
/// name appears here only while it is really duplicated: the test compares for
/// equality, so a resolved duplicate must be removed from this list too.
const FULL_RESIDUE: [&str; 12] = [
    "io-lifetimes",
    "syn",
    "windows-sys",
    "windows-targets",
    "windows_aarch64_gnullvm",
    "windows_aarch64_msvc",
    "windows_i686_gnu",
    "windows_i686_gnullvm",
    "windows_i686_msvc",
    "windows_x86_64_gnu",
    "windows_x86_64_gnullvm",
    "windows_x86_64_msvc",
];

/// Duplicated crate names for the feature arguments `args`: names that resolve
/// at more than one distinct version on normal and build edges over all
/// targets. The same version reached with different feature sets is not a
/// duplicate to `cargo deny`, so nodes are deduplicated on name and version.
fn duplicated(root: &Path, args: &[&str]) -> BTreeSet<String> {
    let out = Command::new(env!("CARGO"))
        .args([
            "tree",
            "--duplicates",
            "--locked",
            "--target",
            "all",
            "-e",
            "normal,build",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ])
        .args(args)
        .current_dir(root)
        .output()
        .expect("cargo tree must launch");
    assert!(
        out.status.success(),
        "cargo tree {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut versions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for line in String::from_utf8(out.stdout).expect("utf-8 tree").lines() {
        let mut parts = line.split_whitespace();
        if let (Some(name), Some(version)) = (parts.next(), parts.next()) {
            versions
                .entry(name.to_owned())
                .or_default()
                .insert(version.to_owned());
        }
    }
    versions
        .into_iter()
        .filter(|(_, seen)| seen.len() > 1)
        .map(|(name, _)| name)
        .collect()
}

/// The capability features, from the Makefile (which reads `cargo metadata`).
fn capability_features(root: &Path) -> Vec<String> {
    let out = Command::new("make")
        .args(["-s", "rust-feature-list"])
        .current_dir(root)
        .output()
        .expect("make must launch");
    assert!(out.status.success(), "make rust-feature-list failed");
    let features: Vec<String> = String::from_utf8(out.stdout)
        .expect("utf-8 feature list")
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    assert!(!features.is_empty(), "empty capability feature list");
    features
}

/// The residue allowed per single-feature graph. Everything not named here
/// must have no duplicate at all, so a consumer with only `source-audit` (the
/// Quoin case) resolves none.
fn allowed(feature: &str) -> BTreeSet<String> {
    match feature {
        // `jsonschema -> strum_macros` (and `-> ahash -> zerocopy-derive`) is
        // on syn 2; the crate's own `syn = 3` and serde/thiserror derives are on
        // syn 3. Neither side is ours to move.
        "manifest" => BTreeSet::from(["syn".to_owned()]),
        _ => BTreeSet::new(),
    }
}

/// The default graph, and every capability feature alone with defaults off,
/// have no duplicate crate version except the named residue; `full` has
/// exactly `FULL_RESIDUE`. Equality both ways: a new duplicate fails, and so
/// does residue that has since been resolved but is still listed.
#[test]
#[trace("TC-198", "FR-014-AC-10")]
fn tc_198_feature_graphs_have_no_unlisted_duplicate_crates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(
        duplicated(root, &[]),
        BTreeSet::new(),
        "the default graph must resolve no duplicate crate version"
    );
    for feature in capability_features(root) {
        assert_eq!(
            duplicated(root, &["--no-default-features", "--features", &feature]),
            allowed(&feature),
            "duplicate crate versions under `--no-default-features --features {feature}` \
             differ from the listed residue"
        );
    }
    let source_audit = duplicated(
        root,
        &["--no-default-features", "--features", "source-audit"],
    );
    assert!(
        source_audit.is_empty(),
        "a consumer with only `source-audit` must resolve no duplicate: {source_audit:?}"
    );
    let full_residue: BTreeSet<String> =
        FULL_RESIDUE.iter().map(|name| (*name).to_owned()).collect();
    assert_eq!(
        duplicated(root, &["--features", "full"]),
        full_residue,
        "duplicate crate versions under `--features full` differ from the listed residue"
    );
}
