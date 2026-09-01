---
id: SR-028
title: "Code review — campaign migration contract"
type: SpecReview
analysis: code-review
scope: "docs/migration-contract.md, tests/test_migration_contract.py, tests/test_module.py, spec/functional/FR-013, spec/tests.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-013"
    type: "references"
---

# SR-028: Code review — campaign migration contract

## Summary

Reviews the FR-013 migration contract added for #10: a decision table built
from a census of all eight campaign repositories, two prohibitions, the
domain/intake boundary, rollback handling, and the pull-request checklist a
migration is judged against.

## Verdict

**CONDITIONAL** — no high findings. One medium finding records what the census
does not cover, and one low finding records a test I had to weaken.

## Gates

- `make lint` (ruff) — clean.
- `make validate-docs` — clean; FR-013 raises no EARS or quality warning.
- `scripts/check_content_rights.py --tree` — passes, unmodified.
- `python3 -m pytest` — 182 pass, 0 fail.

## Findings

| ID      | Severity | Summary                                                              | Refs                                    |
| ------- | -------- | -------------------------------------------------------------------- | --------------------------------------- |
| FND-001 | medium   | The census covers `scripts/`, not every place generic machinery can hide | tests/test_migration_contract.py:76      |
| FND-002 | low      | TC-093's allocation check counts backticked names, so prose can break it | tests/test_migration_contract.py:159     |

## Finding detail

### FND-001 — the census reads `scripts/`, and generic machinery is not only there

TC-088 lists every file under `scripts/` in the eight repositories at
`origin/main` and fails if the decision table does not name it. That is a real
census, and it caught the shape of the table.

It does not cover generic machinery living elsewhere: a `Makefile` target that
collects evidence inline, a `build.rs` that writes a manifest, a `justfile`, or
a workflow step. `schemas/` was read by hand while writing the table — the
repository-local evidence schemas are named in it — but no test enforces that
second tree.

Failure scenario: a repository moves its collector from `scripts/collect.sh`
into a Makefile target, and the census reports the table complete.

Accepted for now, and named rather than left implicit: the decision table's own
rule covers it ("a family not in this table is domain logic until somebody
argues otherwise, in writing, on the migration issue"), and the PR checklist
requires the inventory in the PR body. Extending TC-088 to `schemas/`,
`Makefile`, and workflows is the obvious next step and belongs to whichever
migration first trips over it.

### FND-002 — the allocation check is sensitive to prose

TC-093 asserts each repository name appears exactly once, in backticks, in the
section above the decision table. If a future edit mentions a repository in
prose there, the test fails for a reason that has nothing to do with the
allocation being wrong.

Failure scenario: someone adds a sentence naming `tl-mltl` in the intro and the
allocation test goes red.

Accepted: the alternative — parsing the table — would pass a document whose
allocation table was deleted entirely, which is the failure that matters. A
brittle test that fails loudly beats a permissive one that passes silently, and
the fix when it fires is a one-line edit.

## Notes

- TC-088 is the load-bearing test and the reason the table is a census rather
  than a guess. It reads the eight repositories at `origin/main`, not the
  working trees, so a dirty checkout cannot make the table look complete.
- It skips when the campaign repositories are absent, and says which are
  missing. A census that cannot read its population has not been taken, and
  saying so is better than passing.
- Baselines (`unsafe_comment_baseline.txt`) are excluded from the census with
  a stated reason: they travel with the check that reads them, and listing them
  separately would pad the table without adding a decision.
- The two prohibitions are stated as rules with their permitted case attached.
  "No repository-local generic evidence schema" would be unusable without
  "a schema that describes a repository's own domain output stays" — and TC-089
  asserts both halves, so a future edit cannot quietly drop the exception and
  turn the rule into a ban on domain schemas.
- The contract points at a gate that is genuinely closed: TC-094 asserts the
  contract says migration waits on acceptance, *and* reads the matrix to
  confirm acceptance is still unrecorded. A document claiming to wait on a gate
  that had already opened would pass the first half alone.
