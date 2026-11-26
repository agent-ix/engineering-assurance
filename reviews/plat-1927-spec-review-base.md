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

## Dispositions

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | At `0b433be1f127208a38d6bde77038a4d13c57271f`, both TC-991 rows in `spec/tests.md` and the Rust trace attribute name FR-070-AC-16 without the former FR-070-AC-1 association. The computed matrix resolves AC-16 to the changed test and preserves AC-1's separate reusable-workflow test. |

## Current review verdict

**PASS** at `0b433be1f127208a38d6bde77038a4d13c57271f`. The original traceability finding is fixed, and the full specification diff introduces no additional defect. The workflow inventory, AC-16, test description, documented invocation, and trace tag describe the same current no-caller condition. The reusable receipt workflow remains present and covered by its existing contract tests.

## Recheck scope

| Unit | Role | Excerpt |
|---|---|---|
| FR-070-AC-16 | examined | While Engineering Assurance lacks protected workflow identities, approval policy and sink configuration, it has no `.github/workflows/receipt.yml` main-branch caller; the reusable `change-receipt.yml` remains available to adopters with those protected inputs. |
| TC-991 (FR-070 case) | examined | With protected workflow identities, approval policy and sink configuration absent, Engineering Assurance has no main-branch receipt caller; its missing-receipt caller remains scheduled/on-demand with read-only permissions |
| FR-070-AC-1 | context_only | Engineering Assurance's tests parse its checked-in `.github/workflows/release-attestation.yml` and `.github/workflows/change-receipt.yml` reusable workflows and confirm their triggers, jobs and per-job permissions match the table; each adopter's tests parse only its own checked-in caller workflows. No parsed workflow declares `pull_request_target` or `workflow_run`. |
