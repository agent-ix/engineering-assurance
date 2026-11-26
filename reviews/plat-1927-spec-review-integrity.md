---
id: SR-2026112016223475846355064
title: "Integrity analysis of PLAT-1927 receipt workflow contract"
type: SpecReview
analysis: integrity
scope: "FR-070-AC-16, TC-991; spec/functional/FR-070-release-and-receipt-workflow-files.md and spec/tests.md"
review_set: subset
---

## Summary

The spec change adds a testable condition for withholding Engineering Assurance's main receipt caller and documents continued reusable-workflow availability. The criterion is structurally consistent with the workflow table; the test-matrix mismatch with the changed TC-991 assertion is recorded in the base review.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | low | No integrity-specific findings. | - |

## Traceability matrix

| Scope | Stakeholder/system relation | Verification |
|---|---|---|
| FR-070-AC-16 | Remains within FR-070's existing workflow scope; no new relationship edge is introduced. | TC-991 is specified as an integration test; its test row binds to AC-16. |

## Consistency and atomicity

- The deleted caller is removed from the workflow inventory while the reusable receipt workflow and missing-receipt check remain listed.
- AC-16 describes the condition and observable absence of the main-branch caller, and identifies adopter availability of the reusable workflow.
- No duplicate workflow implementation or new domain object is introduced.

## Current review verdict

**PASS** at `0b433be1f127208a38d6bde77038a4d13c57271f`. Independent recheck of the complete specification diff found no additional integrity defect. The TC-991 coverage-map correction removes the stale AC-1 association without changing the new state-based AC-16 obligation. The original no-finding row records a clean analysis and is not an unresolved defect.

## Recheck scope

| Unit | Role | Excerpt |
|---|---|---|
| FR-070-AC-16 | examined | While Engineering Assurance lacks protected workflow identities, approval policy and sink configuration, it has no `.github/workflows/receipt.yml` main-branch caller; the reusable `change-receipt.yml` remains available to adopters with those protected inputs. |
| TC-991 (FR-070 case) | examined | With protected workflow identities, approval policy and sink configuration absent, Engineering Assurance has no main-branch receipt caller; its missing-receipt caller remains scheduled/on-demand with read-only permissions |
| FR-070-AC-1 | context_only | Engineering Assurance's tests parse its checked-in `.github/workflows/release-attestation.yml` and `.github/workflows/change-receipt.yml` reusable workflows and confirm their triggers, jobs and per-job permissions match the table; each adopter's tests parse only its own checked-in caller workflows. No parsed workflow declares `pull_request_target` or `workflow_run`. |
