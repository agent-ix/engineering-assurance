---
id: SR-034
title: "Scope review of the Rust migration CI posture amendment"
type: SpecReview
analysis: scope-boundary
scope: "FR-017, NFR-005, TC-112, TC-127"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

## Summary

The original no-dispatch constraint joined two different authority classes:
internal qualification of an Engineering Assurance change and external or
release action.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-069 | high | Prohibiting all hosted CI prevents Engineering Assurance from applying its accepted strict qualification policy to its own code. Permitting all hosted work would instead exceed the ticket's authority. | ADR-002 qualification isolation; FR-017-CON-3; NFR-005-AC-6 | wrong-requirement |

## Disposition

Fixed by allocating automatic repository qualification to Engineering
Assurance while retaining explicit human dispatch for real-agent, release,
publication, and other externally consequential operations.
