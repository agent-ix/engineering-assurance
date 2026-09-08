---
id: SR-033
title: "Risk review of the Rust migration CI posture amendment"
type: SpecReview
analysis: risk-complexity
scope: "FR-017, NFR-005, TC-112, TC-127"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

## Summary

Automatic deterministic checks reduce stale-evidence risk. The principal new
risk is confusing those checks with externally consequential evaluation or
release work.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-068 | medium | A shared automatic job that can reach host agents, evidence-bearing evaluation, publication, or release turns routine pull-request updates into external side effects and couples fast qualification to unavailable hosts. | FR-017-CON-3; TC-112, TC-127 | wrong-requirement |

## Disposition

Fixed at the requirement boundary. Automatic checks are deterministic and
repository-scoped; manual-only operations are explicitly prohibited from those
jobs and remain independently invocable.
