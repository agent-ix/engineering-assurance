---
id: SR-910
title: "EARS review — FR-085 no-gesture behavior (EA-113 PR 307)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/engineering-assurance; spec/functional/FR-085 Behavior, no-gesture clause"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-085
    type: references
---

## Summary

Ticket: EA-113; EARS review of the edited no-gesture Behavior statement. The deterministic validator reported no EARS-form warning on this statement; semantic inspection found two SHALL obligations bundled into one bullet.

## Verdict

**CONDITIONAL** — one medium atomicity issue degrades requirement traceability.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The no-gesture Behavior bullet combines process-group selection, deadline selection and a second SHALL for foreground-group separation | spec/functional/FR-085-approval-tag-gesture-signing-mode.md:83-86 |

## Examined scope

The FR-085 Behavior bullet for a missing gesture, including its conditional foreground-group clause; the related FR-085-AC-1 and TC-1160.
