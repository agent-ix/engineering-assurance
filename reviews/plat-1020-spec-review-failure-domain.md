---
id: SR-16399
title: "Failure-domain review of plan-declared constant-predictor answer families (engineering-assurance PR 199)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/engineering-assurance@7efdaee; spec/functional/FR-021-measurement-plan-decision-rule.md; spec/functional/FR-020-measurement-plan-objective.md"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-021
    type: references
---

# Failure-domain review of plan-declared constant-predictor answer families (engineering-assurance PR 199)

## Summary

Ticket: PLAT-1020. Examined the identity and edge cases of `answer_families`: presence/absence by baseline, empty list, duplicates, empty strings, non-strings, retired current-shape plans, baseline switches, and definition-change identity. Collapsing (the dangerous direction) is caught by set equality with the producer's rows; splitting only raises the bar. Retired current-shape plans with `constant-predictor` become invalid without the list; that is intended prerelease behaviour, not a defect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The spec does not say whether `answer_families` is compared as a list or a set for the definition-change check. The baseline is order-independent (a size-weighted sum over families) and the checker compares sets, so reordering cannot change any result, yet FR-021-AC-17 ("adding, removing, or changing a family") leaves it open whether a reorder without a version bump is a finding. FR-024 states this explicitly for `protected_apparatus` ("compare the list as a set: reordering it is not a change"). State the same here and add the reorder case to AC-17/TC-241. | spec/functional/FR-021-measurement-plan-decision-rule.md:134-135; spec/functional/FR-021-measurement-plan-decision-rule.md:253 |

## Verdict

One medium identity gap; otherwise the failure domain is covered by AC-16's refusals and the producer-side equality.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit b8a5d3a: FR-021 states set comparison for AnswerFamilies and the definition-change check; AC-17 and TC-241 add the reorder-yields-no-finding case (and list add/remove). |
