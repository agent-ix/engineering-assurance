---
id: SR-912
title: "Failure-domain review — interactive terminal handoff (EA-113 PR 307)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/engineering-assurance; FR-085 terminal handoff/restoration Behavior and AC-2/AC-11"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-085
    type: references
---

## Summary

Ticket: EA-113; failure-domain analysis of the interactive terminal handoff and restoration contract. The requirements cover success, signer failure and timeout, but do not define the outcome when acquiring or restoring the controlling terminal's foreground group fails.

## Verdict

**CONDITIONAL** — the terminal handoff failure path has no specified refusal or recovery behavior.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-085 does not define behavior when terminal foreground-group acquisition or restoration fails | spec/functional/FR-085-approval-tag-gesture-signing-mode.md:68-73 |

## Examined scope

FR-085 Behavior bullets for foreground-group acquisition/restoration; FR-085-AC-2, AC-11; process-host terminal failure handling.

