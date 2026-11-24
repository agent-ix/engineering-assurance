---
id: SR-911
title: "Integrity review — approval gesture coverage and trail propagation (EA-113 PR 307)"
type: SpecReview
analysis: integrity
scope: "agent-ix/engineering-assurance; FR-085-AC-1; FR-086-AC-5..6; NFR-005-AC-5; spec/tests.md TC-1140..1162"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-085
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-086
    type: references
---

## Summary

Ticket: EA-113; integrity review of acceptance testability, changed Test Case Summary ids and the FR-086 approval-trail propagation criterion. The EA-147 restatement is consistent with the implementation guarantee. Two medium findings remain.

## Verdict

**CONDITIONAL** — the trail test omits the undeclared case and new TC ids are missing summary rows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-086-AC-6 does not exercise the undeclared software-key trail entry | tests/approval_trail_cli.rs:388-404 |
| FND-002 | medium | TC-1140 through TC-1159 have no Test Case Summary rows | spec/tests.md:1084-1086; tests/approval_tag_host.rs:536 |

## Examined scope

FR-085-AC-1 and TC-1160; FR-086-AC-5 and AC-6 with TC-1162; NFR-005-AC-5; the added TC-1140..1162 tags and their summary rows.
