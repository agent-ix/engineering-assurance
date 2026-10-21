---
id: SR-141
title: "Gap analysis — native onboarding report port"
type: SpecReview
analysis: gap-analysis
scope: "spec/functional/FR-001-inventory-before-proposal.md, spec/functional/FR-020*, spec/functional/FR-021*, spec/functional/FR-025*, src/onboard_report.rs, tests/python_port/onboarding_script.rs, tests/python_port/module.rs, tests/content_rights_parity.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-001"
    type: references
---

## Summary

Audited the STD-228 onboarding report diff against the computed Test Matrix, tagged Rust tests, and native implementation. The changed report behavior has owning requirements and real test binders; the repository-wide matrix has existing unbacked criteria outside this diff.

## Verdict

**CONDITIONAL** — the onboarding report diff has no uncovered criterion or unowned behavior, while 30 repository-wide criteria remain unbacked or method-only outside this PR's changed surface.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The computed repository matrix has 24 untagged and 6 method-only criteria outside the onboarding report diff; the port changes no criterion or test trace that causes these existing gaps. | spec/functional/FR-008; spec/functional/FR-014; spec/functional/FR-016; spec/functional/FR-018; spec/functional/FR-019; spec/non-functional/NFR-001..NFR-005 |

## Dispositions

| Finding | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred: outside this PR | These criteria govern other capabilities and are unchanged by the onboarding report port; their owning work remains in the broader assurance backlog. |

## Coverage

- Reconciliation: `quire matrix --scope . --format json` on the candidate worktree; 247 criteria across 32 documents, 217 tagged, 24 untagged, 6 method-only.
- Changed report criteria FR-001-AC-8 and FR-001-AC-9 have six and four native test binders respectively. Schema checklist criteria FR-020-AC-1, FR-021-AC-12, and FR-025-AC-7 also have native binders; the new order regression test adds another FR-020-AC-1 binder.
- Direct code-to-requirement review found no new stub, unowned behavior, or trace inflation in the diff. The measurement-record prose checklist remains the same content previously present in this repository's JavaScript report.
- Plan completion: not assessed
- Optional semantic review: skipped; scoped code-to-test alignment was reviewed in SR-140.
