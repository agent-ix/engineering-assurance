---
id: SR-013
title: "Failure-domain review of verification semantics"
type: SpecReview
analysis: failure-domain
scope: "FR-008..FR-010; NFR-004; IT-005"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-009"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-010"
    type: reviews
---

# Failure-domain review of verification semantics

## Summary

Unknown versions, missing provenance, absent links, malformed inputs, failed or
unavailable producers, stale/suspect/vacuous evidence, tampering, ambiguity, and
unreadable legacy records all have explicit non-success behavior.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Resolved: ambiguous PGM-01 input could have been guessed; the mapper must now return unmapped/lossy or unreadable. | FR-010-AC-3 |
| FND-002 | high | Resolved: an unknown version could have appeared empty; it now fails explicitly. | FR-009-AC-3 |

