//! FR-011-AC-9: the accepted corpus reproduces from its recorded sources.
//!
//! The builder lives in the pinned `agent-ix/qa-corpus` submodule and reads the
//! source repositories the corpus was built from, so it is invoked rather than
//! reimplemented. Skipped (stated, never silent) where the submodule or source
//! repositories are not checked out. Set `ASSURANCE_SOURCE_ROOT` to the
//! directory the source repositories sit in when running from a worktree.
use super::common::root;
use std::path::{Path, PathBuf};
use std::process::Command;

const SOURCE_REPOSITORIES: [&str; 3] = ["quire-contract-ir", "quire-code-rs", "quoin"];

/// Trace: FR-011-AC-9, TC-077.
#[test]
fn the_corpus_reproduces_from_its_recorded_sources() {
    let repo = root();
    let submodule = repo.join("corpus");
    if !submodule.join(".git").exists() {
        eprintln!(
            "SKIP: the qa-corpus submodule is not checked out; run \
             `git submodule update --init corpus`"
        );
        return;
    }
    let checkouts = std::env::var_os("ASSURANCE_SOURCE_ROOT")
        .filter(|v| !v.is_empty())
        .map_or_else(
            || repo.parent().expect("repo has a parent").to_path_buf(),
            PathBuf::from,
        );
    let missing: Vec<&str> = SOURCE_REPOSITORIES
        .into_iter()
        .filter(|r| !checkouts.join(r).join(".git").exists())
        .collect();
    if !missing.is_empty() {
        eprintln!(
            "SKIP: source repositories are not checked out: {}",
            missing.join(", ")
        );
        return;
    }
    let unreachable = unreachable_revisions(&submodule, &checkouts);
    if !unreachable.is_empty() {
        eprintln!(
            "SKIP: recorded source revisions are absent from the local checkouts: {}",
            unreachable.join(", ")
        );
        return;
    }
    let result = Command::new("python3")
        .args(["scripts/build_compatibility_corpus.py", "--check"])
        .current_dir(&submodule)
        .env("ASSURANCE_SOURCE_ROOT", &checkouts)
        .output()
        .expect("run python3");
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

/// `repo@revision` pairs recorded in the corpus whose commit the local
/// checkout does not hold (a stale or shallow clone cannot reproduce them).
fn unreachable_revisions(submodule: &Path, checkouts: &Path) -> Vec<String> {
    let manifest: serde_json::Value = serde_json::from_str(&super::common::read(
        &submodule.join("compatibility/corpus.json"),
    ))
    .expect("corpus.json is valid json");
    let mut absent = std::collections::BTreeSet::new();
    for case in manifest["cases"].as_array().into_iter().flatten() {
        let origin = &case["origin"];
        let (Some(repository), Some(revision)) =
            (origin["repository"].as_str(), origin["revision"].as_str())
        else {
            continue;
        };
        let name = repository.rsplit('/').next().unwrap_or(repository);
        let held = Command::new("git")
            .args(["-C"])
            .arg(checkouts.join(name))
            .args(["cat-file", "-e", &format!("{revision}^{{commit}}")])
            .output()
            .is_ok_and(|o| o.status.success());
        if !held {
            absent.insert(format!("{name}@{}", &revision[..revision.len().min(8)]));
        }
    }
    absent.into_iter().collect()
}
