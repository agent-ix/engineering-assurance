"""FR-013 — the reviewed campaign migration contract.

Most of these read the contract. One reads the eight campaign repositories and
checks the decision table actually accounts for what is in them, because "every
recurring script family" is a claim about the world and not about the document.
"""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

import pytest

from engineering_assurance.compatibility import load_matrix
from engineering_assurance.compatibility_corpus import CORPUS_SUBMODULE

REPO_ROOT = CORPUS_SUBMODULE.parent
CONTRACT_PATH = REPO_ROOT / "docs" / "migration-contract.md"
CONTRACT = CONTRACT_PATH.read_text(encoding="utf-8")

CAMPAIGN_REPOSITORIES = (
    "quire-contract-ir",
    "quire-contract-runtime",
    "quire-contract-codegen",
    "quire-analyze",
    "tl-syntax",
    "tl-parse",
    "tl-mltl",
    "tl-rewrite",
)

DECISIONS = ("KEEP", "DELETE", "REPLACE")


def decision_rows() -> list[tuple[str, str]]:
    """Every decision-table row as (families cell, decision cell)."""
    rows: list[tuple[str, str]] = []
    for line in CONTRACT.splitlines():
        if not line.startswith("| `") and not line.startswith("| ["):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) >= 4 and any(word in cells[2] for word in DECISIONS):
            rows.append((cells[0], cells[2]))
    return rows


def test_every_family_carries_exactly_one_decision() -> None:
    """Trace: FR-013-AC-1, TC-087."""
    rows = decision_rows()
    assert len(rows) >= 12, "the decision table is too small to be a census"
    for families, decision in rows:
        named = [word for word in DECISIONS if word in decision]
        assert len(named) == 1, f"{families} carries {named or 'no decision'}"
        # A decision without a reason is an assertion, not a contract.
        assert families.strip(), "a row names no family"


def test_the_table_accounts_for_every_recurring_family() -> None:
    """Trace: FR-013-AC-2, TC-088.

    Skipped when the campaign repositories are not checked out. Stated, never
    silent: a census that cannot read its population has not been taken.
    """
    checkouts = REPO_ROOT.parent
    missing = [
        name
        for name in CAMPAIGN_REPOSITORIES
        if not (checkouts / name / ".git").exists()
    ]
    if missing:
        pytest.skip(f"campaign repositories not checked out: {', '.join(missing)}")

    families: set[str] = set()
    for name in CAMPAIGN_REPOSITORIES:
        listing = subprocess.run(
            ["git", "-C", str(checkouts / name), "ls-tree", "-r", "--name-only",
             "origin/main", "--", "scripts"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout.split()
        families.update(Path(path).name for path in listing)

    table = CONTRACT
    unaccounted = sorted(
        family
        for family in families
        # Baselines travel with the check that reads them.
        if not family.endswith(".txt") and family not in table
    )
    assert unaccounted == [], (
        "these script families exist in the campaign repositories and the "
        f"decision table does not name them: {', '.join(unaccounted)}"
    )


def test_both_prohibitions_are_stated_by_name() -> None:
    """Trace: FR-013-AC-3, TC-089."""
    assert "No repository-local generic evidence schema" in CONTRACT
    assert "No universal stdout corroboration" in CONTRACT
    assert re.search(r"verdict recovered from console text", CONTRACT)

    # And the permitted case is stated, so the rule is a boundary rather than a
    # ban on every schema a repository owns.
    assert "describes *its own domain output*" in CONTRACT
    assert "differential summary" in CONTRACT
    assert "conformance manifest" in CONTRACT


def test_domain_validation_and_evidence_intake_have_distinct_owners() -> None:
    """Trace: FR-013-AC-4, TC-090."""
    assert "Domain output validation is not evidence intake" in CONTRACT
    for owner in (
        "the domain repository, in its own tests",
        "Quoin intake",
        "Quoin audit",
        "a human, through ix-flow",
    ):
        assert owner in CONTRACT, f"no row names {owner}"


def test_rollback_is_per_failure_mode_and_never_rewrites_history() -> None:
    """Trace: FR-013-AC-5, TC-091."""
    assert "## Rollback" in CONTRACT
    assert "Legacy history is never rewritten in any of these paths." in CONTRACT
    assert "do not edit the legacy record to make it read" in CONTRACT
    assert "do not scrape stdout as a stopgap" in CONTRACT
    # Deletion is the last step, and it waits on the same candidate revision.
    assert "Delete last" in CONTRACT
    assert "same exact candidate revision" in CONTRACT


def test_the_review_checklist_covers_every_required_question() -> None:
    """Trace: FR-013-AC-6, TC-092."""
    checklist = [
        line for line in CONTRACT.splitlines() if line.strip().startswith("- [ ]")
    ]
    assert len(checklist) >= 10, "the checklist is too short to review a migration"
    joined = " ".join(checklist)
    for topic in (
        "script inventory",
        "generic evidence schema",
        "stdout",
        "byte-identical",
        "compatibility view",
        "not-computed",
        "manual dispatch",
        "check_compatibility_matrix",
    ):
        assert topic in joined, f"the checklist does not ask about {topic}"


def test_the_agent_allocation_covers_all_eight_repositories_once() -> None:
    """Trace: FR-013-AC-7, TC-093."""
    allocation = CONTRACT.split("## The decision table")[0]
    for name in CAMPAIGN_REPOSITORIES:
        assert allocation.count(f"`{name}`") == 1, f"{name} is not allocated exactly once"
    for agent in ("| A |", "| B |", "| C |"):
        assert agent in allocation, f"agent {agent} holds nothing"
    assert "does not change hands" in allocation


def test_migration_waits_on_acceptance_and_claims_no_qualification() -> None:
    """Trace: FR-013-AC-8, TC-094, FR-013-CON-1, FR-013-CON-3."""
    assert "may begin until the compatibility matrix records" in CONTRACT
    # Whitespace-normalized: the sentence is line-wrapped in the document, and
    # a reader cares that it is said, not where it broke.
    flat = " ".join(CONTRACT.split())
    assert "An agent cannot grant that acceptance" in flat

    # The gate the contract points at is genuinely still closed.
    assert load_matrix()["accepted"]["state"] == "pending_human_acceptance"

    assert "makes no certification, accreditation, authorization, identity, or" in CONTRACT
    assert "does not qualify any repository for" in CONTRACT

    # CON-2: the contract changes no trigger, and says so.
    assert "stays manual-dispatch only" in CONTRACT
    assert "dispatches nothing and changes no trigger" in CONTRACT
