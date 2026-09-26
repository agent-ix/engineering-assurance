// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-014-AC-11: one content-digest identity. No private SHA-256 helper may
//! reappear next to `ContentDigest`; every remaining `sha2` use in `src/` is
//! either the digest module itself or a documented different-purpose site.

use std::{collections::BTreeSet, fs, path::Path};

use ix_trace_rs::trace;

/// Every file under `src/` that may name `sha2` or `Sha256`, with the reason.
/// The comparison is exact: an entry whose file no longer uses `sha2` fails,
/// so the list cannot go stale, and an unlisted file that starts using it
/// fails, so a private helper cannot reappear.
const ALLOWED: [(&str, &str); 2] = [
    (
        "src/content_digest.rs",
        "the one content-digest module: the only place an evidence-byte hash is computed",
    ),
    (
        "src/producer_execution.rs",
        "campaign source projection computes the Git object id of a blob in Git's SHA-256 \
         object format, which must equal what `git` reports; that is not a content identity",
    ),
];

fn rust_sources(directory: &Path, found: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(directory).expect("source directory reads") {
        let path = entry.expect("entry reads").path();
        if path.is_dir() {
            rust_sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

fn uses_sha2(source: &str) -> bool {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .any(|line| line.contains("Sha256") || line.contains("sha2::"))
}

#[trace("TC-200", "FR-014-AC-11")]
#[test]
fn tc_200_sha256_is_used_only_by_the_digest_module_and_documented_sites() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_sources(&root.join("src"), &mut files);
    files.sort();
    let mut observed = BTreeSet::new();
    for file in files {
        let source = fs::read_to_string(&file).expect("source reads");
        if uses_sha2(&source) {
            let relative = file
                .strip_prefix(root)
                .expect("source is under the manifest directory")
                .to_string_lossy()
                .replace('\\', "/");
            observed.insert(relative);
        }
    }
    let allowed = ALLOWED
        .iter()
        .map(|(path, _)| (*path).to_owned())
        .collect::<BTreeSet<_>>();
    let unlisted = observed
        .iter()
        .filter(|path| !allowed.contains(*path))
        .collect::<Vec<_>>();
    let stale = allowed
        .iter()
        .filter(|path| !observed.contains(*path))
        .collect::<Vec<_>>();
    assert!(
        unlisted.is_empty() && stale.is_empty(),
        "sha2 use outside the allowlist (route it through ContentDigest): {unlisted:?}; \
         allowlist entries that no longer use sha2: {stale:?}"
    );
    for (path, reason) in ALLOWED {
        assert!(!reason.is_empty(), "{path} needs a documented reason");
    }
}

#[trace("TC-200", "FR-014-AC-11")]
#[test]
fn tc_200_the_scan_detects_a_private_helper() {
    assert!(uses_sha2("let mut hasher = Sha256::new();"));
    assert!(uses_sha2("use sha2::Digest;"));
    assert!(!uses_sha2("// Sha256 is mentioned only in a comment"));
    assert!(!uses_sha2("let digest = ContentDigest::of_bytes(bytes);"));
}
