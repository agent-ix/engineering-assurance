---
id: SR-20261114084862
title: "EA-108 base specification review"
type: SpecReview
analysis: base
scope: "spec/functional/FR-070-release-and-receipt-workflow-files.md and linked FR-075/FR-076"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-070"
    type: reviews
---

## Summary

Reviewed the changed FR-070 action-reference criterion and its links to FR-075/FR-076, including verification inventory consistency. The requirement now specifies a structural check and allows major tags by default, but the test inventory still says SHA pinning is required.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-559 still requires every action in an `id-token: write` job to end in a 40-hex SHA, contradicting updated FR-070-AC-3 and the removed pin assertions; the test catalog now misstates the contract. | spec/tests.md:919; FR-070-AC-3 |

## Coverage Review

FR-070-AC-3 now states that action refs are structurally valid and a full commit SHA is not required absent a concrete reason. The newly added structural test is not identified by a TC row, while the stale TC-559 still describes the superseded pin requirement. Existing pins in the signing workflow are not addressed by this diff and have a concrete OIDC signing-boundary rationale; no change is called for in this review.
