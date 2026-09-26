// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-014: every capability feature compiles alone with default features off.
//!
//! Needs `make` and `python3` on `PATH` (the Makefile derives the list with
//! the latter). Covers `--lib` only, as `make rust-features` does.

use std::{
    collections::BTreeSet,
    io::{Read, Seek, SeekFrom},
    path::Path,
    process::{Command, Stdio},
    thread::sleep,
    time::{Duration, Instant},
};

use ix_trace_rs::trace;
use serde_json::Value;

/// Generous ceiling for the nested cold-cache `cargo check` runs.
const MAKE_TIMEOUT: Duration = Duration::from_mins(30);

/// The capability features per `cargo metadata`: the crate's features minus
/// `default` and the `full` umbrella. Implicit optional-dependency features
/// appear here too, and the manifest syntax (quoting, comments, tabs) is
/// cargo's problem, not a hand parser's.
fn metadata_features(root: &Path) -> (BTreeSet<String>, BTreeSet<String>) {
    let out = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--format-version", "1", "--locked"])
        .current_dir(root)
        .output()
        .expect("cargo metadata must launch");
    assert!(out.status.success(), "cargo metadata failed");
    let meta: Value = serde_json::from_slice(&out.stdout).expect("metadata JSON");
    let package = meta["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .find(|p| p["name"] == "engineering-assurance")
        .expect("engineering-assurance package");
    let features = package["features"].as_object().expect("features table");
    let capability: BTreeSet<String> = features
        .keys()
        .filter(|k| !matches!(k.as_str(), "default" | "full"))
        .cloned()
        .collect();
    // Plain feature names `full` turns on (not `dep:x` or `pkg/feat`).
    let full_members: BTreeSet<String> = features["full"]
        .as_array()
        .expect("full members")
        .iter()
        .filter_map(|v| v.as_str())
        .filter(|m| !m.contains(':') && !m.contains('/'))
        .map(str::to_owned)
        .collect();
    (capability, full_members)
}

/// Runs `make <args>` in `root`, killing it at `MAKE_TIMEOUT`; returns
/// success and the child's stderr.
fn run_make(root: &Path, args: &[&str]) -> (bool, String) {
    let stderr_file = tempfile::tempfile().expect("stderr capture");
    let mut child = Command::new("make")
        .args(args)
        .current_dir(root)
        .stdout(Stdio::null())
        .stderr(Stdio::from(stderr_file.try_clone().expect("clone stderr")))
        .spawn()
        .expect("make must launch");
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait on make") {
            break Some(status);
        }
        if start.elapsed() > MAKE_TIMEOUT {
            child.kill().expect("kill timed-out make");
            child.wait().expect("reap make");
            break None;
        }
        sleep(Duration::from_millis(200));
    };
    // The clone handed to the child shares this offset; rewind before reading.
    let mut stderr_file = stderr_file;
    let mut text = String::new();
    stderr_file.seek(SeekFrom::Start(0)).expect("rewind stderr");
    stderr_file.read_to_string(&mut text).ok();
    (status.is_some_and(|s| s.success()), text)
}

/// The Makefile's feature list equals the metadata-derived capability set, that
/// set covers everything `full` enables, and `make rust-features` compiles the
/// library with no default features and with each capability feature alone; it
/// fails if any of them stops compiling alone.
///
/// The nested make shares this build's target dir, so a stale artifact is
/// only a hazard for the binaries `CARGO_MANIFEST_DIR` runs, not for `check`.
#[test]
#[trace("TC-190", "FR-014-AC-7")]
fn tc_190_each_capability_feature_compiles_alone() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (declared, full_members) = metadata_features(root);
    assert!(
        declared.len() >= 5,
        "metadata found too few features: {declared:?}"
    );
    assert!(
        full_members.is_subset(&declared),
        "`full` enables features the matrix does not check: {:?}",
        full_members.difference(&declared).collect::<Vec<_>>()
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

    let (ok, stderr) = run_make(root, &["rust-features"]);
    assert!(
        ok,
        "a capability feature no longer compiles alone (or make timed out):\n{stderr}"
    );
}

/// An empty or failed derivation is an error rather than a vacuous pass, and
/// `make -n` prints one check per feature without running any.
#[test]
#[trace("TC-190", "FR-014-AC-7")]
fn tc_190_an_empty_derivation_fails_and_dry_run_lists_every_feature() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (ok, stderr) = run_make(root, &["rust-features", "EA_FEATURES="]);
    assert!(!ok, "an empty feature list must fail rust-features");
    assert!(
        stderr.contains("could not derive the feature list"),
        "missing diagnostic: {stderr}"
    );

    let dry = Command::new("make")
        .args(["-n", "rust-features"])
        .current_dir(root)
        .output()
        .expect("make -n must launch");
    assert!(dry.status.success(), "make -n rust-features failed");
    let dry = String::from_utf8(dry.stdout).expect("utf-8 dry run");
    let (declared, _) = metadata_features(root);
    for feature in &declared {
        assert!(dry.contains(feature.as_str()), "dry run omits {feature}");
    }
}

/// The normal-dependency graph of one feature alone (default features off),
/// as the sorted, de-duplicated crate names `cargo tree` prints.
fn normal_graph(root: &Path, feature: &str) -> BTreeSet<String> {
    let out = Command::new(env!("CARGO"))
        .args([
            "tree",
            "--no-default-features",
            "--features",
            feature,
            "-e",
            "normal",
            "--prefix",
            "none",
            "--locked",
        ])
        .current_dir(root)
        .output()
        .expect("cargo tree must launch");
    assert!(out.status.success(), "cargo tree failed for {feature}");
    String::from_utf8(out.stdout)
        .expect("utf-8 tree")
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

/// The `source-audit` feature alone resolves none of the crates the archive
/// and CLI capabilities need, and a capability feature that names another
/// capability pulls that capability's dependencies but nothing beyond it
/// (`manifest` reaches `yaml_serde` through `structured-yaml`; neither reaches
/// the archive crates).
#[test]
#[trace("TC-196", "FR-014-AC-8")]
fn tc_196_source_audit_graph_excludes_archive_and_cli_crates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let audit = normal_graph(root, "source-audit");
    assert!(
        audit.contains("syn"),
        "source-audit graph lost syn: {audit:?}"
    );
    for banned in ["cap-std", "clap", "tar", "zip", "flate2"] {
        assert!(
            !audit.contains(banned),
            "source-audit must not resolve {banned}: {audit:?}"
        );
    }

    let manifest = normal_graph(root, "manifest");
    assert!(
        manifest.contains("yaml_serde") && manifest.contains("jsonschema"),
        "manifest graph lost a dependency: {manifest:?}"
    );
    for banned in ["cap-std", "clap", "tar", "zip", "flate2"] {
        assert!(
            !manifest.contains(banned),
            "manifest must not resolve {banned}: {manifest:?}"
        );
    }
}
