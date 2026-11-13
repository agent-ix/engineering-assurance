---
id: SR-20261113170523
title: "Integrity review of the advisory scan requirement (PR 234)"
type: SpecReview
analysis: integrity
scope: "agent-ix/engineering-assurance; FR-044 relationships, research-status table, and acceptance criteria"
review_set: all
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-044
    type: references
---

## Summary

Reviewed requirement linkage, consistency, atomicity, and criterion testability. The new criterion conflicts with its research table, and the requirement has no stakeholder-requirement relationship.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | AC-13 requires two status values, but the table uses a third and misclassifies the partial-scan question | spec/functional/FR-044-osv-scanner-advisory-scan-producer-request.md:52-55,174 | wrong-requirement |
| FND-002 | medium | FR-044 has no relationship to a stakeholder requirement | spec/functional/FR-044-osv-scanner-advisory-scan-producer-request.md:5-17 | missing-requirement |

## Dispositions

| FND | outcome | reason / after_excerpt |
| --- | --- | --- |
| FND-001 | still-open | Row 5 now explicitly classifies partial lookup signaling as UNVERIFIED, but row 4 still says `VERIFIED absent`, which is outside AC-13's required VERIFIED/UNVERIFIED values. |
| FND-002 | fixed | `- target: "ix://agent-ix/engineering-assurance/StR-002"` / `type: "traces_to"` |
