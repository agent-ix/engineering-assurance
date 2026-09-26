// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-014-AC-11: one content-digest identity. No private hash helper may
//! reappear next to `ContentDigest`: every use of a hash crate in `src/` is
//! either the digest module itself or a documented different-purpose module.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use ix_trace_rs::trace;

/// Every file under `src/` that may name a hash crate or hash type, with the
/// reason. Each entry is a whole file that does nothing else, so the scope is
/// the specific site, not a large module. The comparison is exact: an entry
/// whose file no longer names a hash crate fails, so the list cannot go
/// stale, and an unlisted file that starts doing so fails.
const ALLOWED: [(&str, &str); 2] = [
    (
        "src/content_digest.rs",
        "the one content-digest module: the only place an evidence-byte hash is computed",
    ),
    (
        "src/git_object_id.rs",
        "Git blob object ids (SHA-1 and SHA-256 object formats) must equal what `git` \
         reports; that is not a content identity",
    ),
];

/// Hash crates that may be direct dependencies. A new hash dependency must be
/// triaged here (and, if it hashes evidence bytes, go through `ContentDigest`).
const ALLOWED_HASH_DEPENDENCIES: [&str; 2] = ["sha1", "sha2"];

/// Crate names and hash-type or trait identifiers. Matched as whole
/// identifier tokens on every non-comment line, so an alias (`use sha2 as
/// hashing;`), a leading-colon path, a `Digest` trait import and another
/// hash crate are all caught whatever they are renamed to.
const HASH_TOKENS: [&str; 23] = [
    "sha1",
    "sha2",
    "sha3",
    "blake2",
    "blake3",
    "md5",
    "md4",
    "ripemd",
    "crc32fast",
    "Digest",
    "Sha1",
    "Sha224",
    "Sha256",
    "Sha384",
    "Sha512",
    "Sha3_256",
    "Sha3_512",
    "Blake2b",
    "Blake2s",
    "Blake3",
    "Md5",
    "Md4",
    "Ripemd160",
];

fn rust_sources(directory: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("source directory reads") {
        let path = entry.expect("entry reads").path();
        if path.is_dir() {
            rust_sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

fn line_names_a_hash(line: &str) -> bool {
    let is_import = line.trim_start().starts_with("use ") || line.contains("extern crate");
    line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .any(|token| {
            if token.is_empty() || !HASH_TOKENS.contains(&token) {
                return false;
            }
            if token.starts_with(|c: char| c.is_ascii_uppercase()) {
                // A hash type or the `Digest` trait: any use is a use, except
                // a bare `Digest` that is not imported or path-qualified (a
                // local enum variant or field named Digest).
                token != "Digest" || is_import
            } else {
                // A crate name is a use only as a path root or an import, not
                // as a data field or a string (the corpus `blake3` field
                // records another system's identity and is not a hash use).
                is_import || line.contains(&format!("{token}::"))
            }
        })
}

fn names_a_hash(source: &str) -> bool {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .any(line_names_a_hash)
}

/// Returns (files naming a hash outside the allowlist, allowlisted files that
/// no longer name one) for the tree rooted at `root`.
fn audit(root: &Path) -> (Vec<String>, Vec<String>) {
    let mut files = Vec::new();
    rust_sources(&root.join("src"), &mut files);
    let mut observed = BTreeSet::new();
    for file in files {
        if names_a_hash(&fs::read_to_string(&file).expect("source reads")) {
            let relative = file
                .strip_prefix(root)
                .expect("source is under the root")
                .to_string_lossy()
                .replace('\\', "/");
            observed.insert(relative);
        }
    }
    let allowed = ALLOWED
        .iter()
        .map(|(path, _)| (*path).to_owned())
        .collect::<BTreeSet<_>>();
    (
        observed.difference(&allowed).cloned().collect(),
        allowed.difference(&observed).cloned().collect(),
    )
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("directory creates");
    for entry in fs::read_dir(from).expect("directory reads") {
        let path = entry.expect("entry reads").path();
        let target = to.join(path.file_name().expect("entry has a name"));
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            fs::copy(&path, &target).expect("file copies");
        }
    }
}

/// A scratch copy of `src/` with `append` added to the end of `file`.
fn mutated(file: &str, append: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("scratch directory");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &root.path().join("src"),
    );
    let path = root.path().join(file);
    let mut source = fs::read_to_string(&path).expect("mutated file reads");
    source.push_str(append);
    fs::write(&path, source).expect("mutated file writes");
    root
}

fn hash_dependencies(manifest: &str) -> BTreeSet<String> {
    let mut in_dependencies = false;
    let mut found = BTreeSet::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_dependencies = line == "[dependencies]";
        } else if in_dependencies && let Some((name, _)) = line.split_once('=') {
            let name = name.trim();
            if HASH_TOKENS
                .iter()
                .any(|token| token.eq_ignore_ascii_case(name))
                || matches!(name, "md-5" | "digest" | "sha3" | "blake2")
            {
                found.insert(name.to_owned());
            }
        }
    }
    found
}

#[trace("TC-200", "FR-014-AC-11")]
#[test]
fn tc_200_hash_crates_are_named_only_by_the_digest_module_and_the_git_object_id_module() {
    let (unlisted, stale) = audit(Path::new(env!("CARGO_MANIFEST_DIR")));
    assert!(
        unlisted.is_empty() && stale.is_empty(),
        "hash use outside the allowlist (route it through ContentDigest): {unlisted:?}; \
         allowlist entries that no longer name a hash: {stale:?}"
    );
    for (path, reason) in ALLOWED {
        assert!(!reason.is_empty(), "{path} needs a documented reason");
    }
    let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("manifest reads");
    assert_eq!(
        hash_dependencies(&manifest),
        ALLOWED_HASH_DEPENDENCIES
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<BTreeSet<_>>(),
        "a hash crate dependency was added or removed; triage it against ContentDigest"
    );
}

/// Mutations that must fail the guard, each proven on a scratch copy.
#[trace("TC-200", "FR-014-AC-11")]
#[test]
fn tc_200_mutations_are_detected() {
    for (label, file, append) in [
        (
            "M1 private helper in a domain module",
            "src/evidence.rs",
            "\nfn private(b: &[u8]) { let _ = Sha256::digest(b); }\n",
        ),
        (
            "M2 private helper appended to the large producer_execution module",
            "src/producer_execution.rs",
            "\nfn private(b: &[u8]) { let _ = sha2::Sha256::digest(b); }\n",
        ),
        (
            "M3 crate alias and another SHA-2 type",
            "src/evidence.rs",
            "\nuse sha2 as hashing;\nfn private(b: &[u8]) { let _ = hashing::Sha512::digest(b); }\n",
        ),
        (
            "M4 leading-colon path",
            "src/evaluation.rs",
            "\nfn private(b: &[u8]) { let _ = ::sha2::Sha224::digest(b); }\n",
        ),
        (
            "M5 Digest trait import only",
            "src/semantics/pgm01.rs",
            "\nuse digest::Digest as _;\n",
        ),
        (
            "M6 another hash crate under an alias",
            "src/compatibility_corpus.rs",
            "\nuse blake3 as h;\nfn private(b: &[u8]) { let _ = h::hash(b); }\n",
        ),
        ("M7 md5 alias", "src/campaign.rs", "\nuse md5 as m;\n"),
    ] {
        let scratch = mutated(file, append);
        let (unlisted, stale) = audit(scratch.path());
        assert_eq!(unlisted, [file.to_owned()], "{label} must fail the guard");
        assert!(stale.is_empty(), "{label}: {stale:?}");
    }
    // The allowlisted modules are not exempt from staleness: with the hash
    // code gone the entry is reported.
    let scratch = mutated("src/git_object_id.rs", "");
    fs::write(scratch.path().join("src/git_object_id.rs"), "//! empty\n").expect("file writes");
    assert_eq!(
        audit(scratch.path()),
        (Vec::new(), vec!["src/git_object_id.rs".to_owned()])
    );
    assert!(names_a_hash("let mut hasher = Sha256::new();"));
    assert!(!names_a_hash("// Sha256 is mentioned only in a comment"));
    assert!(!names_a_hash(
        "let digest = ContentDigest::of_bytes(bytes);"
    ));
    assert!(!names_a_hash("let value = \"sha256-jcs\";"));
    assert!(!names_a_hash("    pub blake3: String,"));
    assert!(!names_a_hash("    Digest {"));
    assert!(names_a_hash("use blake3 as h;"));
    assert!(names_a_hash("let _ = ::md5::compute(b);"));
}
