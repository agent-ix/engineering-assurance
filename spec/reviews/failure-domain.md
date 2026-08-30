---
id: SR-002
title: "Failure-domain review of assurance onboarding"
type: SpecReview
analysis: failure-domain
scope: "StR-001; US-001..US-004; FR-001..FR-007; NFR-001..NFR-003; IT-001..IT-004; TM-001"
review_set: all
---

## Summary

The review exercised malformed/conflicting artifacts, failed staged validation,
path escape, incomplete provenance, conflicting availability states, run-id
collision, absent executables, and missing or synthesized terminal decisions.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Resolved: malformed or conflicting applicable artifacts remain unchanged and require an explicit human resolution. | FR-001-AC-6; TC-044 | missing-requirement |
| FND-002 | high | Resolved: failed validation and path/symlink escape cannot publish an artifact or load unowned package content. | FR-001-AC-7; FR-003-AC-5; TC-018; TC-045 | missing-requirement |
| FND-003 | high | Resolved: missing/mutable provenance and conflicting availability labels cannot become observed evidence. | FR-004-AC-5; FR-004-AC-7; TC-024; TC-046 | missing-requirement |
| FND-004 | high | Resolved: cross-boundary run-id reuse and inferred terminal outcomes fail closed, while explicit acceptance and rejection are both retained. | FR-005-AC-6..AC-7; FR-006-AC-5; TC-047..TC-049 | missing-requirement |
