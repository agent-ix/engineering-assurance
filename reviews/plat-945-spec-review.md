---
id: SR-16309
title: "spec-review of FR-010 withdrawal and compatibility removal (engineering-assurance PR 196)"
type: SpecReview
analysis: integrity
scope: "agent-ix/engineering-assurance@e0cc38d1; spec/spec.md; spec/tests.md; spec/functional/FR-010 (deleted); spec/functional/FR-013; spec/functional/FR-015; spec/functional/FR-018; spec/integration/IT-005; spec/non-functional/NFR-004; spec/stakeholder/StR-002; spec/usecase/US-005; plan/PLAN-002-verification-semantics/**; docs/migration-contract.md; docs/verification-semantics/ADR-001-ownership-and-type-fit.md"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-015
    type: references
  - target: ix://agent-ix/engineering-assurance/US-005
    type: references
---

# Spec review of the FR-010 withdrawal and compatibility removal (engineering-assurance PR 196)

## Summary

Ticket: PLAT-945. `quire validate --scope . "spec/**/*.md" "plan/**/*.md" "reviews/**/*.md"` (quire 0.36.2, engine 0.50.2) exits 0 with 14 pre-existing grammar warnings, none in a file this PR edits. Historical review records under `reviews/` and `spec/reviews/` that cite FR-010 are kept unedited, as required; their `relationships` targets to the deleted FR-010 do not fail validation (same as the existing FR-011 precedent). Outside those historical reviews, FR-010 now appears only in the `spec/spec.md` withdrawal line and the FR-015 withdrawal note. Every `relationships` edge to FR-010 (StR-002, FR-015, IT-005, PLAN-002, TASK-009) is removed, FR-013's upstream dependency is cleared, and the US-005/TC-054/TC-055 and FR-010 matrix rows are removed from both the TC and requirement-to-TC tables. FR-015-CON-1 is withdrawn with a not-reused note.

Examined and clean: FR-013 statement edit, FR-015 title/inputs/behaviour/AC-1/AC-3 edits, IT-005 procedure renumbering, NFR-004 and StR-002 dependency edits, ADR-001 compatibility-decision removal, docs/migration-contract.md checklist edit (matched by the migration_contract test edit), TASK-009 acceptance range.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-015-AC-2 still requires `lossy` to "remain distinguishable", and the TC-100/TC-102/TC-103 variation row still lists `lossy` as a semantic-state variant. `lossy` existed only as a `Pgm01Outcome`; after this PR it appears in no contract, fixture, source or test (the non-success-state vocabulary in verification-semantics-ownership-v1.json has no `lossy`). The AC now names a state the system does not have, and no test can back that clause. Remove `lossy` from FR-015-AC-2 and from the variation row. | spec/functional/FR-015-semantic-and-identity-parity.md:72; spec/tests.md:503 |
| FND-002 | medium | Withdrawn ids are not recorded where the repo records them. The tests.md Coverage Gaps paragraph still says "TC-052..TC-068 are backed natively" and its withdrawn-ids sentence (which lists FR-011, TC-069..TC-078 and others) omits TC-054, TC-055, TC-064..TC-067, FR-010, US-005-AC-2/AC-3 and FR-015-CON-1. PLAN-002's TASK-011 row and completion gate, and TASK-011's acceptance, still require "TC-052..TC-068", a range that now contains six deleted ids. Update the range claims and add the new withdrawals to the not-reused sentence. | spec/tests.md:633-638; plan/PLAN-002-verification-semantics/plan.md:59; plan/PLAN-002-verification-semantics/plan.md:63; plan/PLAN-002-verification-semantics/tasks/TASK-011-fixtures-and-gate.md:30 |
| FND-003 | low | US-005 drops AC-2 and AC-3 with no withdrawal note, unlike FR-015 and spec.md, which state that withdrawn identifiers are not reused. | spec/usecase/US-005-correlate-definition-result-evidence.md:24 |
| FND-004 | low | Stale compatibility/report wording left behind: TC-100's description still says Rust agrees with "the accepted identity and report reference"; PLAN-002's title and heading still read "Verification semantics and compatibility implementation"; FR-018's edited sentence reads "native Rust semantic-reference and fixture validation, ownership-registry validation all pass locally", which lost its conjunction. | spec/tests.md:335; plan/PLAN-002-verification-semantics/plan.md:3; spec/functional/FR-018-staged-runtime-migration.md:60 |

## Verdict

The withdrawal is structurally sound: quire validate is green, no live relationship points at FR-010, historical reviews are preserved, and no replacement ledger or compatibility layer is introduced. Two medium findings block: FR-015-AC-2 still mandates a `lossy` state that no longer exists, and the Coverage Gaps / PLAN-002 range claims and withdrawn-id record are inconsistent with the removed TCs and ACs. Not mergeable until FND-001 and FND-002 are fixed; the lows are one-line edits in the same files.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit d2836165 removes `lossy` from FR-015-AC-2 and from the TC-100/TC-102/TC-103 variation row; no `lossy` state reference remains in spec/. |
| FND-002 | fixed | Fix commit d2836165 lists the surviving ids (TC-052, TC-053, TC-056..TC-063, TC-068) consistently in the spec/tests.md Coverage Gaps paragraph, the INT-005 row, the PLAN-002 TASK-011 row and TASK-011 acceptance; the PLAN-002 completion gate says "every surviving" row; the not-reused sentence now records TC-054, TC-055, TC-064..TC-067 with FR-010, US-005-AC-2, US-005-AC-3 and FR-015-CON-1. |
| FND-003 | fixed | Fix commit d2836165 adds the US-005-AC-2/AC-3 withdrawn, not-reused note. |
| FND-004 | fixed | Fix commit d2836165 drops "and report reference" from TC-100, renames PLAN-002 to "Verification semantics implementation", and restores the conjunction in FR-018. |
