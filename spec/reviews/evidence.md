---
id: SR-005
title: "Evidence-method review of assurance onboarding"
type: SpecReview
analysis: evidence
scope: "StR-001; US-001..US-004; FR-001..FR-007; NFR-001..NFR-003; IT-001..IT-004; TM-001"
review_set: all
---

## Summary

`quoin advise` reports zero uncatalogued and zero inconclusive methods. Its nine
remaining mismatches recommend performance benchmarking solely because NFR rows
use numeric thresholds; the selected catalogued methods verify parity, agent
behavior, and package membership, none of which is a performance property.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | Resolved: documentation ordering/separation criteria use executable tests instead of inspection. | FR-003-AC-6; FR-007-AC-3; TC-019; TC-037 | wrong-requirement |
| FND-002 | low | Confirmed: integration, property, architecture-conformance, agent-behaviour, contract, and integration methods are intentional for exact-count conformance thresholds; performance benchmarking would not test the stated properties. | NFR-001-M-1..M-3; NFR-002-M-1..M-3; NFR-003-M-1..M-3 | correct-requirement-no-evidence |
| FND-003 | high | Resolved: retained evaluation evidence now includes immutable versions, transcript digest, effort counters, fixture revision, outcome, and terminal event. | FR-006-AC-2; TC-032 | missing-requirement |
