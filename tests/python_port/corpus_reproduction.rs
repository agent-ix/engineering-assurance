//! FR-011-AC-9: the accepted corpus reproduces from its recorded sources.
//!
//! The builder lives in the pinned `agent-ix/qa-corpus` submodule and reads the
//! source repositories the corpus was built from, so it is invoked rather than
//! reimplemented. Skipped (stated, never silent) only where the submodule or a
//! source repository is not checked out at all. A checkout that is present but
//! stale -- one that does not hold a recorded revision -- fails, as the retired
//! Python test did: the corpus cannot be shown to reproduce from it. Set
//! `ASSURANCE_SOURCE_ROOT` to the directory the source repositories sit in when
//! running from a worktree.
use super::common::root;
use ix_trace_rs::trace;
use std::path::PathBuf;
use std::process::Command;

const SOURCE_REPOSITORIES: [&str; 3] = ["quire-contract-ir", "quire-code-rs", "quoin"];

#[test]
#[trace("TC-077", "FR-011-AC-9")]
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
