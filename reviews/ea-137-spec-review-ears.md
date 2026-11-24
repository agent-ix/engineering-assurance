---
id: SR-20261124025465
title: "EARS conformance review of EA-137 behaviour statements"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/engineering-assurance; PR #304 (git diff origin/main...HEAD at d5b0d7dd5): Behavior bullets added to FR-043, FR-070, FR-081, FR-082, FR-083"
review_set: subset
---

## Summary

Ticket: EA-137. Checked each added Behavior bullet for EARS form (named system, SHALL,
one response, explicit trigger or condition). All bullets name their system and use
SHALL or SHALL NOT with one response. One defect: three negative bullets depend on the
previous bullet through the pronoun "it".

## Verdict

PASS with one low finding.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | low | "The tag comparison SHALL NOT judge it.", "The command SHALL NOT list it as `approved`." and "The command SHALL NOT drop it." are not standalone: "it" refers to the previous bullet. Name the subject (a receipt that does not verify; a receipt whose tag is absent or changed). | FR-082 Behavior, FR-043 Behavior |

## Dispositions

Round 1, fix commit 881b7852f.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The three bullets now name their subject: "SHALL NOT list a receipt whose tag is absent or changed as `approved`", "SHALL NOT drop a receipt that does not verify", "SHALL NOT judge a receipt that does not verify as missing". |
