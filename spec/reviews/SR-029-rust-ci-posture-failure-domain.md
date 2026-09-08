---
id: SR-029
title: "Failure-domain review of the Rust migration CI posture amendment"
type: SpecReview
analysis: failure-domain
scope: "FR-017, NFR-005, TC-112, TC-127"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

## Summary

Broadening the existing workflow trigger without constraining its selected
targets could automatically start expensive or evidence-bearing operations.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-064 | high | A pull-request trigger applied to an undifferentiated workflow could invoke real-agent evaluation, release evaluation, publication, or release work. That would spend external resources or create release evidence without the manual action the existing policy protects. | FR-017 behavior and CON-3; TC-112, TC-127 | missing-requirement |

## Disposition

Fixed in the proposed amendment: automatic jobs are limited to deterministic
repository qualification and are tested to select none of the four manual-only
operation classes.
