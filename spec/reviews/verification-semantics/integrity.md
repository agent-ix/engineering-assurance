---
id: SR-014
title: "Integrity review of verification semantics"
type: SpecReview
analysis: integrity
scope: "FR-008..FR-010; IT-005"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-008"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-010"
    type: reviews
---

# Integrity review of verification semantics

## Summary

The model keeps source records authoritative, carries record and field-path
references, preserves exact producer/config/source/environment/definition
premises, and proves legacy source bytes unchanged across compatibility reads.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Resolved: a copied projection could drift from its source; FR-009-AC-4 now requires explicit source record and field-path references. | FR-009 |
| FND-002 | high | Resolved: legacy mapping lacked a mutation guard; FR-010-AC-1 compares source bytes before and after. | FR-010 |

