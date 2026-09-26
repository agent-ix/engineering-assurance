//! Port of `tests/test_migration_contract.py`: the reviewed campaign migration
//! contract defined by FR-013.
//!
//! Most of these read the contract. One reads the eight campaign repositories
//! and checks the decision table accounts for what is in them.
use ix_trace_rs::trace;
use std::path::Path;
use std::process::Command;
use std::sync::LazyLock;

use super::common::{read, root, truthy};

const CAMPAIGN_REPOSITORIES: [&str; 8] = [
    "quire-contract-ir",
    "quire-contract-runtime",
    "quire-contract-codegen",
    "quire-analyze",
    "tl-syntax",
    "tl-parse",
    "tl-mltl",
    "tl-rewrite",
];

const DECISIONS: [&str; 3] = ["KEEP", "DELETE", "REPLACE"];

static CONTRACT: LazyLock<String> =
    LazyLock::new(|| read(&root().join("docs").join("migration-contract.md")));

/// Every decision-table row as (families cell, decision cell).
fn decision_rows() -> Vec<(String, String)> {
    let mut rows = Vec::new();
    for line in CONTRACT.lines() {
        if !line.starts_with("| `") && !line.starts_with("| [") {
            continue;
        }
        let cells: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() >= 4 && DECISIONS.iter().any(|word| cells[2].contains(word)) {
            rows.push((cells[0].to_owned(), cells[2].to_owned()));
        }
    }
    rows
}

#[test]
#[trace("TC-087", "FR-013-AC-1")]
fn every_family_carries_exactly_one_decision() {
    let rows = decision_rows();
    assert!(
        rows.len() >= 12,
        "the decision table is too small to be a census"
    );
    for (families, decision) in rows {
        let named: Vec<&str> = DECISIONS
            .iter()
            .copied()
            .filter(|word| decision.contains(word))
            .collect();
        assert_eq!(named.len(), 1, "{families} carries {named:?}");
        // A decision without a reason is an assertion, not a contract.
        assert!(!families.trim().is_empty(), "a row names no family");
    }
}

#[test]
#[trace("TC-088", "FR-013-AC-2")]
fn the_table_accounts_for_every_recurring_family() {
    // Skipped when the campaign repositories are not checked out. Stated,
    // never silent: a census that cannot read its population has not been taken.
    let root = root();
    let checkouts = root.parent().expect("repo root has a parent");
    let missing: Vec<&str> = CAMPAIGN_REPOSITORIES
        .iter()
        .copied()
        .filter(|name| !checkouts.join(name).join(".git").exists())
        .collect();
    if !missing.is_empty() {
        eprintln!(
            "SKIP: campaign repositories not checked out: {}",
            missing.join(", ")
        );
        return;
    }

    let mut families = std::collections::BTreeSet::new();
    for name in CAMPAIGN_REPOSITORIES {
        let output = Command::new("git")
            .arg("-C")
            .arg(checkouts.join(name))
            .args([
                "ls-tree",
                "-r",
                "--name-only",
                "origin/main",
                "--",
                "scripts",
            ])
            .output()
            .expect("git runs");
        assert!(output.status.success(), "git ls-tree failed for {name}");
        for path in String::from_utf8_lossy(&output.stdout).split_whitespace() {
            if let Some(file) = Path::new(path).file_name() {
                families.insert(file.to_string_lossy().into_owned());
            }
        }
    }

    // Baselines travel with the check that reads them.
    let unaccounted: Vec<&str> = families
        .iter()
        .map(String::as_str)
        .filter(|family| {
            Path::new(family).extension().is_none_or(|ext| ext != "txt")
                && !CONTRACT.contains(family)
        })
        .collect();
    assert!(
        unaccounted.is_empty(),
        "these script families exist in the campaign repositories and the \
         decision table does not name them: {}",
        unaccounted.join(", ")
    );
}

#[test]
#[trace("TC-089", "FR-013-AC-3")]
fn both_prohibitions_are_stated_by_name() {
    assert!(CONTRACT.contains("No repository-local generic evidence schema"));
    assert!(CONTRACT.contains("No universal stdout corroboration"));
    assert!(CONTRACT.contains("verdict recovered from console text"));

    // And the permitted case is stated, so the rule is a boundary rather than
    // a ban on every schema a repository owns.
    assert!(CONTRACT.contains("describes *its own domain output*"));
    assert!(CONTRACT.contains("differential summary"));
    assert!(CONTRACT.contains("conformance manifest"));
}

#[test]
#[trace("TC-090", "FR-013-AC-4")]
fn domain_validation_and_evidence_intake_have_distinct_owners() {
    assert!(CONTRACT.contains("Domain output validation is not evidence intake"));
    for owner in [
        "the domain repository, in its own tests",
        "Quoin intake",
        "Quoin audit",
        "a human, through ix-flow",
    ] {
        assert!(CONTRACT.contains(owner), "no row names {owner}");
    }
}

#[test]
#[trace("TC-091", "FR-013-AC-5")]
fn rollback_is_per_failure_mode_and_never_rewrites_history() {
    assert!(CONTRACT.contains("## Rollback"));
    assert!(CONTRACT.contains("Legacy history is never rewritten in any of these paths."));
    assert!(CONTRACT.contains("do not edit the legacy record to make it read"));
    assert!(CONTRACT.contains("do not scrape stdout as a stopgap"));
    // Deletion is the last step, and it waits on the same candidate revision.
    assert!(CONTRACT.contains("Delete last"));
    assert!(CONTRACT.contains("same exact candidate revision"));
}

#[test]
#[trace("TC-092", "FR-013-AC-6")]
fn the_review_checklist_covers_every_required_question() {
    let checklist: Vec<&str> = CONTRACT
        .lines()
        .filter(|line| line.trim().starts_with("- [ ]"))
        .collect();
    assert!(
        checklist.len() >= 10,
        "the checklist is too short to review a migration"
    );
    let joined = checklist.join(" ");
    for topic in [
        "script inventory",
        "generic evidence schema",
        "stdout",
        "byte-identical",
        "compatibility view",
        "not-computed",
        "manual dispatch",
        "compatibility-observe",
    ] {
        assert!(
            joined.contains(topic),
            "the checklist does not ask about {topic}"
        );
    }
}

#[test]
#[trace("TC-093", "FR-013-AC-7")]
fn the_agent_allocation_covers_all_eight_repositories_once() {
    let allocation = CONTRACT
        .split("## The decision table")
        .next()
        .expect("split yields a first part");
    for name in CAMPAIGN_REPOSITORIES {
        assert_eq!(
            allocation.matches(&format!("`{name}`")).count(),
            1,
            "{name} is not allocated exactly once"
        );
    }
    for agent in ["| A |", "| B |", "| C |"] {
        assert!(allocation.contains(agent), "agent {agent} holds nothing");
    }
    assert!(allocation.contains("does not change hands"));
}

#[test]
#[trace(
    "TC-094",
    "FR-013-AC-8",
    "FR-013-CON-1",
    "FR-013-CON-2",
    "FR-013-CON-3"
)]
fn migration_waits_on_acceptance_and_claims_no_qualification() {
    assert!(CONTRACT.contains("may begin until the compatibility matrix records"));
    // Whitespace-normalized: the sentence is line-wrapped in the document, and
    // a reader cares that it is said, not where it broke.
    let flat = CONTRACT.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("An agent cannot grant that acceptance"));

    // Asserting the gate is still *closed* would freeze this test at the moment
    // it was written; asserting it cannot open anonymously is the property the
    // contract actually depends on.
    let matrix_path = super::common::package_root().join("compatibility-matrix.json");
    let matrix: serde_json::Value = serde_json::from_str(&read(&matrix_path)).expect("valid json");
    let acceptance = &matrix["accepted"];
    let state = acceptance["state"].as_str().expect("state is a string");
    assert!(
        ["pending_human_acceptance", "accepted"].contains(&state),
        "unexpected acceptance state {state}"
    );
    if state == "accepted" {
        assert!(
            truthy(&acceptance["accepted_by"]),
            "the gate opened with nobody on record"
        );
        assert!(
            truthy(&acceptance["accepted_at"]),
            "the gate opened with no date on record"
        );
    }

    assert!(
        CONTRACT.contains("makes no certification, accreditation, authorization, identity, or")
    );
    assert!(CONTRACT.contains("does not qualify any repository for"));

    // CON-2: the contract changes no trigger, and says so.
    assert!(CONTRACT.contains("stays manual-dispatch only"));
    assert!(CONTRACT.contains("dispatches nothing and changes no trigger"));
}
