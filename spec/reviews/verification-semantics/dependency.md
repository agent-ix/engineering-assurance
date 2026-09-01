---
id: SR-015
title: "Dependency review of verification semantics"
type: SpecReview
analysis: dependency
scope: "ADR-001; FR-008..FR-010; NFR-004"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-008"
    type: reviews
---

# Dependency review of verification semantics

## Summary

The dependency graph is acyclic: Engineering Assurance defines vocabulary;
Quire exports static facts; native tools execute; Quoin transcribes, retains,
audits, and reports; ix-flow records human decisions. FR-009 depends on FR-008,
and FR-010 depends on both. No upstream record family is reparented.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved dependency or ownership cycle remains after reconciliation with quire-rs#384 and quoin#267/#281/#282. | ADR-001; FR-008..FR-010 |

