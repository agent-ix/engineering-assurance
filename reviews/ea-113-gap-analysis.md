---
id: SR-908
title: "Gap analysis — approval gesture review scope (EA-113 PR 307)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/engineering-assurance; FR-085-AC-1..11; FR-086-AC-1..6; changed src/ and tests/ approval gesture paths; spec/tests.md TC-1140..1162"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance
    type: references
---

## Summary

Ticket: EA-113; disposition review of PR #307 at f3e299e6 against origin/main a04da1e1. FND-001 was rejected under the supplied owner ruling that TC summary rows and IDs are ceremony; the affected tests use direct AC trace attributes. FND-002 was fixed by a command-level FR-085-AC-13 regression test that verifies error propagation and removal of the created tag ref. No findings remain open.

## Verdict

**PASS** — FND-001 is rejected under the owner ruling and FND-002 is fixed; no findings remain open.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-1140 through TC-1159 have no matching Test Case Summary rows | tests/approval_tag_host.rs:536; tests/signature_verify_cli.rs:1428; spec/tests.md:1084 |

## Coverage

- Reconciliation: quoin matrix (quoin 0.36.8; quire 0.52.0, engine 0.62.2); quire coverage (0.52.0).
- Changed criteria: FR-085-AC-1..8, AC-10 and AC-11 are tagged; AC-9 and CON-1/2 are inspection-only. FR-086-AC-1..6 and AC-8 are tagged; AC-7 is untagged in the unchanged FR-086 context.
- The TC-1140..TC-1159 summary-row concern is rejected under the owner ruling. Direct AC trace attributes are the traceability record; TC summary rows and TC IDs are not required.
- The repository-wide matrix also reports 149 untagged and 18 ignored-only criteria outside this PR-diff review scope; no run evidence is present for the matrix.
- Changed-surface reverse check: inventoried behavior groups: approval-tag declaration/signing, terminal handoff/restoration, signature gesture evidence, approval result projection and trail propagation; untraced behavior: 0; source stubs: 0; test stubs: 0.
- Plan completion: not assessed
- Semantic review: skipped (not opted in).

## Examined scope

FR-085-AC-1..11; FR-086-AC-1..6; TC-1140..TC-1162; new source/test paths named in scope.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | FR-085-AC-13's test exercises the retry helper, not process-host failure propagation and approval-tag ref cleanup | src/producer_execution.rs:1877-1898; src/approval_tag_host.rs:484-492 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | rejected | The owner ruling supplied for EA-113 treats Test Case Summary rows and TC IDs as ceremony; current tests directly trace applicable acceptance criteria, which is sufficient. |
| FND-002 | fixed | The AC-13 test injects restoration failure after signing, checks approval_tag_create_failed, confirms the signer ran and verifies the tag ref is absent. |

### FND-002 evidence at disposition pass 1 (superseded)

At disposition pass 1, the AC-13 test exercised `ForegroundGroup::restore_with` directly with an injected closure. It did not exercise process-host propagation or approval-tag ref cleanup. That evidence was accurate before the command-level regression test was added and is superseded by disposition pass 2.

### FND-002 after_excerpt

> assert_eq!(error.code(), "approval_tag_create_failed");
> assert_eq!(reference.status.code(), Some(1), "the ref must be removed");

### FND-002 evidence at disposition pass 2

The direct FR-085-AC-13 test `restore_failure_propagates_through_approval_tag_and_removes_the_ref` in `src/approval_tag_host.rs` injects a restore failure after the signer runs. It asserts `approval_tag_create_failed` and verifies that `refs/tags/restore-failure` is absent. The exact filtered test passed (1 passed, 0 failed).

## Disposition pass 2 summary

FND-001 remains rejected under the owner ruling. FND-002 is fixed by the command-level AC-13 regression test. No findings remain open.
