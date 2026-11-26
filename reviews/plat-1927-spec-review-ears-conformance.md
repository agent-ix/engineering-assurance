---
id: SR-2026112016223475846355065
title: "EARS review of PLAT-1927 receipt caller criterion"
type: SpecReview
analysis: ears-conformance
scope: "FR-070-AC-16; spec/functional/FR-070-release-and-receipt-workflow-files.md"
review_set: subset
---

## Summary

The new criterion uses a continuous state condition (“While Engineering Assurance lacks …”) and identifies observable workflow state. No EARS-specific defect was found in the changed requirement statement.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | low | No EARS-specific findings. | - |

## Review notes

- The condition is state-based rather than an event trigger, so “While” fits the stated absence condition.
- The response can be checked by inspecting the protected-policy inputs and workflow inventory.

## Current review verdict

**PASS** at `0b433be1f127208a38d6bde77038a4d13c57271f`. Independent recheck of the complete specification diff found no additional EARS defect. The TC-991 coverage-map correction removes the stale AC-1 association without changing the new state-based AC-16 obligation. The original no-finding row records a clean analysis and is not an unresolved defect.

## Recheck scope

| Unit | Role | Excerpt |
|---|---|---|
| FR-070-AC-16 | examined | While Engineering Assurance lacks protected workflow identities, approval policy and sink configuration, it has no `.github/workflows/receipt.yml` main-branch caller; the reusable `change-receipt.yml` remains available to adopters with those protected inputs. |
| FR-070-AC-1 | context_only | Engineering Assurance's tests parse its checked-in `.github/workflows/release-attestation.yml` and `.github/workflows/change-receipt.yml` reusable workflows and confirm their triggers, jobs and per-job permissions match the table; each adopter's tests parse only its own checked-in caller workflows. No parsed workflow declares `pull_request_target` or `workflow_run`. |
