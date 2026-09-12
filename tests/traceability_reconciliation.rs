// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Reconciliation of first-party Rust criterion markers against the test matrix.
//!
//! NFR-005-AC-3 requires Quire to reconcile every Rust criterion marker with no
//! missing, orphaned, or duplicate binding. This exercises the real Quire
//! reconciliation over this repository rather than a fixture, because a marker
//! form Quire cannot parse is exactly the failure the criterion guards.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use ix_trace_rs::trace;
use serde_json::Value;

fn repository() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Submodule directories declared by the repository root `.gitmodules`.
///
/// A marker inside a submodule belongs to that submodule's own repository. The
/// qa-corpus detection fixtures deliberately carry ids that bind to nothing —
/// that is what they exist to test — so they are not this repository's markers.
fn submodule_paths(root: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(root.join(".gitmodules")) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| line.trim().strip_prefix("path"))
        .filter_map(|rest| rest.trim().strip_prefix('='))
        .map(|value| value.trim().trim_end_matches('/').to_owned())
        .filter(|value| !value.is_empty())
        .collect()
}

fn is_first_party(path: &str, submodules: &[String]) -> bool {
    let path = path.trim_start_matches("./");
    !submodules
        .iter()
        .any(|prefix| path == prefix || path.starts_with(&format!("{prefix}/")))
}

fn coverage(root: &Path) -> Value {
    let quire = std::env::var_os("QUIRE").unwrap_or_else(|| "quire".into());
    let output = Command::new(&quire)
        .args(["coverage", "--scope"])
        .arg(root)
        .arg("--json")
        .current_dir(root)
        .output()
        .expect("quire must be available to reconcile Rust markers");
    assert!(
        output.status.success(),
        "quire coverage failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("quire coverage must emit one JSON document")
}

fn entries<'a>(document: &'a Value, key: &str) -> &'a [Value] {
    document
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn text(entry: &Value, key: &str) -> String {
    entry
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn is_rust(path: &str) -> bool {
    PathBuf::from(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
}

#[test]
#[trace("TC-118", "NFR-005-AC-3")]
fn tc_118_quire_reconciles_every_rust_marker_without_missing_orphaned_or_duplicate_bindings() {
    let root = repository();
    let submodules = submodule_paths(root);
    assert!(
        !submodules.is_empty(),
        "the repository declares submodules whose markers are not reconciled here"
    );
    let document = coverage(root);

    // Orphaned: a marker in this repository naming a target the matrix never minted.
    let orphaned = entries(&document, "untracked_symbols")
        .iter()
        .filter(|entry| {
            let path = text(entry, "path");
            is_first_party(&path, &submodules) && is_rust(&path)
        })
        .map(|entry| {
            format!(
                "{}:{} {} -> {}",
                text(entry, "path"),
                entry.get("line").and_then(Value::as_u64).unwrap_or(0),
                text(entry, "symbol"),
                text(entry, "trace_id"),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        orphaned.is_empty(),
        "Rust markers bind to nothing the matrix minted:\n  {}",
        orphaned.join("\n  ")
    );

    // Missing: a tag Quire found but could not bind to a test symbol, so the
    // criterion it names carries no evidence despite appearing to.
    let unmatched = entries(&document, "unmatched_tags")
        .iter()
        .filter(|entry| {
            let path = text(entry, "path");
            is_first_party(&path, &submodules) && is_rust(&path)
        })
        .map(|entry| {
            format!(
                "{}:{} {}",
                text(entry, "path"),
                entry.get("line").and_then(Value::as_u64).unwrap_or(0),
                text(entry, "trace_id"),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        unmatched.is_empty(),
        "Rust tags Quire could not bind to a test symbol:\n  {}",
        unmatched.join("\n  ")
    );

    // Duplicate: one symbol claiming the same trace id more than once. A trace
    // id shared across distinct tests is legitimate evidence, not a duplicate.
    let mut seen = BTreeSet::new();
    let mut duplicates = Vec::new();
    for shared in entries(&document, "shared_trace_ids") {
        let trace_id = text(shared, "trace_id");
        for symbol in entries(shared, "symbols") {
            let path = text(symbol, "path");
            if !is_first_party(&path, &submodules) || !is_rust(&path) {
                continue;
            }
            let binding = (trace_id.clone(), path, text(symbol, "symbol"));
            if !seen.insert(binding.clone()) {
                duplicates.push(format!("{} {} -> {}", binding.1, binding.2, binding.0));
            }
        }
    }
    assert!(
        duplicates.is_empty(),
        "one Rust symbol claims the same trace id more than once:\n  {}",
        duplicates.join("\n  ")
    );

    // The reconciliation must have observed this repository's Rust markers at
    // all; an empty population would satisfy every assertion above vacuously.
    let observed = entries(&document, "binding_census")
        .iter()
        .find(|entry| text(entry, "language") == "rust")
        .and_then(|entry| entry.get("bound").and_then(Value::as_u64))
        .unwrap_or(0);
    assert!(
        observed > 0,
        "quire bound no Rust markers, so this reconciliation proved nothing"
    );
}
