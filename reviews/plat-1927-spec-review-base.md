---
id: SR-2026112016223475846355063
title: "Base specification review of PLAT-1927 receipt caller contract"
type: SpecReview
analysis: base
scope: "FR-070-AC-1, FR-070-AC-16, TC-991; spec/functional/FR-070-release-and-receipt-workflow-files.md and spec/tests.md"
review_set: subset
---

## Summary

The review checked the new FR-070 criterion, the TC-991 row and its criterion matrix bindings. The new no-caller condition is stated clearly, but the matrix retains a stale TC-991 to FR-070-AC-1 association after the corresponding assertion was removed.

## Verdict

**CONDITIONAL** — one medium-severity traceability finding remains.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | TC-991 no longer verifies FR-070-AC-1's parsing of the reusable release and receipt workflows, yet `spec/tests.md` still maps TC-991 to AC-1. Remove the stale matrix binding and the matching test trace tag; retain the new AC-16 binding. | spec/tests.md:351,939; tests/receipt_workflow_contract.rs:382; FR-070-AC-1, FR-070-AC-16 |

## Coverage and consistency

- FR-070-AC-16 states the no-main-caller condition while protected workflow identities, approval policy, and sink configuration are absent, and states that the reusable workflow remains available to adopters.
- The edited TC-991 test checks absence of those three files, absence of `receipt.yml`, and the existing missing-receipt workflow shape. It does not test the reusable-workflow contract described by AC-1.
- The reusable `change-receipt.yml` remains present and is inspected by other receipt contract tests; this review finding concerns TC-991's obsolete AC-1 association.
