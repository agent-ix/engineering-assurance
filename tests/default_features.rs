// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-014-AC-9: the `default` feature set is empty.
//!
//! Deliberately needs no feature (no `serde_json`), so a bare `cargo test`
//! builds and runs it. Cargo unifies features across a whole build; a default
//! that reached `serde_json/arbitrary_precision` would switch it on for every
//! crate in a consumer's workspace.

use std::{collections::BTreeSet, path::Path, process::Command};

use ix_trace_rs::trace;

/// Crates only the binary (`full`) may resolve.
const BINARY_ONLY_CRATES: [&str; 6] = ["cap-std", "clap", "tar", "zip", "flate2", "tempfile"];

/// `cargo tree` stdout for `args` in the crate root, or a failure naming them.
fn tree(root: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO"))
        .arg("tree")
        .args(args)
        .args(["--prefix", "none", "--locked"])
        .current_dir(root)
        .output()
        .expect("cargo tree must launch");
    assert!(
        out.status.success(),
        "cargo tree {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf-8 tree")
}

/// The crate names on the normal-dependency graph for `args`.
fn crates(root: &Path, args: &[&str]) -> BTreeSet<String> {
    let mut all = vec!["-e", "normal"];
    all.extend_from_slice(args);
    tree(root, &all)
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

/// The default graph is the same as the graph with default features off, so
/// `default` names no feature; it resolves none of the binary-only crates and
/// no `serde_json` at all, hence no `serde_json/arbitrary_precision`; and it
/// is the resolution a consumer that depends on the crate without naming a
/// feature gets for this package. The binary still needs `full`.
#[test]
#[trace("TC-197", "FR-014-AC-9")]
fn tc_197_default_features_are_empty_and_resolve_nothing_optional() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let default = crates(root, &[]);
    assert_eq!(
        default,
        crates(root, &["--no-default-features"]),
        "the default feature set must add nothing to the empty feature set"
    );
    for banned in BINARY_ONLY_CRATES {
        assert!(
            !default.contains(banned),
            "the default graph must not resolve {banned}: {default:?}"
        );
    }
    assert!(
        !default.contains("serde_json"),
        "the default graph must not resolve serde_json, so it cannot switch on \
         arbitrary_precision for a consumer's workspace: {default:?}"
    );
    // Every feature edge: no feature is enabled by default, the binary's
    // `full` included.
    assert_eq!(
        tree(root, &["-e", "features"]),
        tree(root, &["-e", "features", "--no-default-features"]),
        "the default feature set must enable no feature"
    );
    let full = crates(root, &["--features", "full"]);
    assert!(
        full.contains("clap") && full.contains("serde_json"),
        "`--features full` must still resolve the binary's crates: {full:?}"
    );
}
