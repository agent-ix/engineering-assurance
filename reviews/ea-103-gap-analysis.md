---
id: SR-20261113141842
title: "Gap analysis of the release-history command (engineering-assurance PR 229)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/engineering-assurance@199828e; PR 229 (EA-103); spec FR-052, FR-051, FR-049, FR-053, NFR-007; quoin FR-168; src/release_history.rs; src/release_history_host.rs; tests/release_history.rs; tests/release_history_cli.rs; spec/tests.md"
review_set: subset
---

# Gap analysis of the release-history command (engineering-assurance PR 229)

## Summary

Ticket: EA-103. Planless gap analysis over the PR diff at 199828e. Read `quoin matrix --json` (quoin 0.30.1): FR-052-AC-1..AC-13 each have at least one tagged test binder (TC-500..TC-514; AC-11 and AC-12 also have unit binders); FR-052-AC-14 is an Inspection criterion with no binder (method-without-symbol), the same as FR-051-AC-9 on main, and was inspected here (no record file opened, parsed or hashed). The tagged tests ran and passed; two mutations were killed (TC-501, TC-508), so the tests can fail. FR-051 (snapshot presence in release-verify) was already merged under EA-100 and is untouched; EA-103's snapshot ACs are context only. No production code without an owning requirement; no stubs; no coverage inflation found.

## Verdict

**PASS with gaps** - every Test criterion of FR-052 is bound to a test that asserts it, but two behaviours the code implements have no criterion and no test: the range when the previous baseline is reachable only through a merge (FND-001, wrong result, see code-review SR-20261113141841 FND-001), and exact `refs/tags/` resolution and git-isolation (replace ref, graft) for this command (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No criterion and no test for a previous baseline that is an ancestor of the baseline but not on its first-parent history. commits_in_range has a dedicated branch for it (merge-base --is-ancestor, then rev-list of the previous baseline's ancestors) that no test reaches, and it yields previously released records again at the merge commit (code-review FND-001). FR-052 needs a criterion for this case and TC-50x needs a fixture for it. | src/release_history_host.rs:247-266 |
| FND-002 | low | No test for the hardening FR-052 inherits from release-verify: a replace ref or graft not changing the list (measured clean here by hand), and a ref outside refs/tags/ never resolving as the previous tag (fails, code-review FND-002). TC-512 tests only a plain branch named like the tag. FR-047-AC-13 has this for release-verify; FR-052 has no equivalent criterion. | tests/release_history_cli.rs:490-528 |

## Coverage

- Criteria examined: FR-052-AC-1 .. FR-052-AC-14 (AC-1..13 bound to TC-500..TC-514, AC-14 by inspection). Context only: FR-051-AC-1..AC-9 (EA-100, on main), FR-049 Outputs/Behavior (release-history as the second consumer interface), FR-053, NFR-007, quoin FR-168 Inputs/Behavior.
- Semantic review: skipped (reviewer subagent, not opted in).
- Plan completion: not assessed

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | round 1 (c51b922): AC-15 and TC-515 added; test kills the Ok(false) mutation. |
| FND-002 | fixed | round 1 (c51b922): TC-516 covers git failure, a replace ref and a graft; TC-512 covers refs/tags/ guessing. |
