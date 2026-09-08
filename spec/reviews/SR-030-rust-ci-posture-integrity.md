---
id: SR-030
title: "Integrity review of the Rust migration CI posture amendment"
type: SpecReview
analysis: integrity
scope: "FR-017, NFR-005, TC-112, TC-127"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/NFR-005"
    type: reviews
---

## Summary

A green workflow name alone is insufficient when it may describe another
revision or an unbound manual run.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-065 | high | TC-127 did not require each status to bind the current pull-request head. A successful run for an earlier revision or an operator's local claim could be presented after the candidate changed. | NFR-005-AC-6; TC-127 | correct-requirement-no-evidence |

## Disposition

Fixed in the proposed amendment by naming all six statuses, binding them to the
current pull-request head, and refusing missing, failed, stale, or manually
substituted results.
