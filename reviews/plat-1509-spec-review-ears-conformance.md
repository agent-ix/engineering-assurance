---
id: SR-143
title: "EARS review of engineering-assurance PR 214 (FR-041)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/engineering-assurance@b957ab8; spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md"
review_set: subset
---

# EARS review of engineering-assurance PR 214

## Summary

Ticket: PLAT-1509. `quire validate` (quire 0.44.0) reports three EARS warnings on FR-041,
all compound statements with two `shall`s. The other Behavior bullets are ubiquitous and
singular.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Behavior bullet "derive `approver` only from the FR-037 `principal`, and SHALL NOT derive a person from the tagger..." has two `shall`s (warned at line 111: unclassifiable, missing subject). Split into two bullets. | FR-041 Behavior line 110-112 |
| FND-002 | low | Behavior bullet "SHALL NOT default a missing trailer, and SHALL NOT report `approved`..." has two `shall`s (line 115). Split. | FR-041 Behavior line 115-116 |
| FND-003 | low | FR-041-CON-2 has two `shall`s (no trailer-parsing code; no duplicated rule) (line 128). Split or join into one prohibition. | FR-041-CON-2 |
