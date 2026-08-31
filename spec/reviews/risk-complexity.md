---
id: SR-006
title: "Risk and complexity review of assurance onboarding"
type: SpecReview
analysis: risk-complexity
scope: "StR-001; US-001..US-004; FR-001..FR-007; NFR-001..NFR-003; IT-001..IT-004; TM-001"
review_set: all
---

## Summary

The highest-risk work is inventory judgment, provenance admission, and human-gate
preservation. Packaging/discovery is medium complexity with high compatibility
impact; the 28-cell agent evaluation is high execution cost but low semantic
volatility once its envelope and fixture rights are fixed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Mitigated: no-write paths, immutable provenance, pinned executable selection, state exclusivity, and explicit terminal-event tests protect the primary epistemic boundary. | TC-002; TC-007; TC-024; TC-046..TC-050 |
| FND-002 | medium | Mitigated: canonical digests, allowlisted packages, escaping-target refusal, and compatibility equivalence constrain host/package drift. | TC-010; TC-018; TC-036; TC-038; TC-040..TC-043 |
