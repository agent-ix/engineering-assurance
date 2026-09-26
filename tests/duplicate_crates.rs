// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-014-AC-10: this crate's own dependency graph, for the default features,
//! each capability feature alone and `full`, carries no duplicate crate version
//! except the named residue.
//!
//! This measures EA's graph in isolation, not a consumer's union with its other
//! dependencies: Quoin's `cargo deny check bans` is the example of a
//! consumer-side measurement, and it can differ (a second consumer dependency on
//! another release of the same crate adds a duplicate this test cannot see).
//! Normal and build edges over every target are counted, which is what
//! `cargo deny check bans` counts by default (dev-dependencies excluded), so no
//! `cargo-deny` is needed.
//!
//! `cargo tree --target all` needs the manifests of Windows-only crates that a
//! Linux build never downloads. On a cold `CARGO_HOME` this test therefore
//! fetches from the network, and offline (or with `CARGO_NET_OFFLINE`) it stops
//! with a message saying so rather than reporting a duplicate. It was not run
//! against a cold `CARGO_HOME` when written.
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

/// `campaign` hashes git blob object ids with SHA-1, so it keeps `sha1` 0.10,
/// which is on `digest` 0.10 and `cpufeatures` 0.2 while `sha2` 0.11 is on
/// `digest` 0.11 and `cpufeatures` 0.3. `sha1` 0.11 would unify them but gives
/// Quoin (its `gix` pin resolves `sha1-checked` on `sha1` 0.10) a second `sha1`.
const CAMPAIGN_RESIDUE: [&str; 4] = ["block-buffer", "cpufeatures", "crypto-common", "digest"];

/// Residue only `full` adds, all in third-party subtrees this crate does not
/// control: `jsonschema` (`syn`) and `cap-std` (`io-lifetimes`, `windows-*`).
/// See "Known duplicate crate versions" in the README for each owner.
const FULL_ONLY_RESIDUE: [&str; 12] = [
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

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

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
        "cargo tree {args:?} failed, which is not a duplicate finding: `--target all` needs \
         the manifests of Windows-only crates, so a cold CARGO_HOME needs the network and \
         offline it errors, and `--locked` needs an up-to-date Cargo.lock: {}",
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

/// The residue allowed for a graph. Everything not named here must have no
/// duplicate at all, so a consumer with only `source-audit` resolves none.
fn allowed(feature: &str) -> BTreeSet<String> {
    match feature {
        // `jsonschema -> strum_macros` (and `-> ahash -> zerocopy-derive`) is
        // on syn 2; the crate's own `syn = 3` and serde/thiserror derives are on
        // syn 3. Neither side is ours to move.
        "manifest" => names(&["syn"]),
        "campaign" => names(&CAMPAIGN_RESIDUE),
        "full" => names(&CAMPAIGN_RESIDUE)
            .union(&names(&FULL_ONLY_RESIDUE))
            .cloned()
            .collect(),
        _ => BTreeSet::new(),
    }
}

/// Fails naming what to change: `new` crates are duplicates nobody listed,
/// `stale` crates are listed but no longer duplicated.
fn assert_residue(graph: &str, actual: &BTreeSet<String>, expected: &BTreeSet<String>) {
    let new: Vec<_> = actual.difference(expected).collect();
    let stale: Vec<_> = expected.difference(actual).collect();
    assert!(
        new.is_empty() && stale.is_empty(),
        "duplicate crate versions under {graph} differ from the listed residue: \
         new duplicates (resolve, or list with a reason): {new:?}; stale residue \
         (remove from the list): {stale:?}"
    );
}

/// The default graph, every capability feature alone with defaults off, and
/// `full` have exactly the listed residue: an unlisted new duplicate fails, and
/// so does residue that has since been resolved but is still listed.
#[test]
#[trace("TC-198", "FR-014-AC-10")]
fn tc_198_feature_graphs_have_no_unlisted_duplicate_crates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert_residue(
        "the default features",
        &duplicated(root, &[]),
        &BTreeSet::new(),
    );
    for feature in capability_features(root) {
        assert_residue(
            &format!("`--no-default-features --features {feature}`"),
            &duplicated(root, &["--no-default-features", "--features", &feature]),
            &allowed(&feature),
        );
    }
    assert_residue(
        "`--features full`",
        &duplicated(root, &["--features", "full"]),
        &allowed("full"),
    );
}
