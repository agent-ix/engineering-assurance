---
id: SR-20261124033410
title: "spec-review (integrity) of per-signature gesture evidence and interactive approval-tag mode (engineering-assurance PR 305)"
type: SpecReview
analysis: integrity
scope: "agent-ix/engineering-assurance@18293e76 (PR 305, diff origin/main...HEAD); spec/functional/FR-042; FR-084; FR-085; FR-086; FR-087"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-085
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-042
    type: references
---

# Integrity review of EA PR 305

## Summary

Ticket: EA-113. This lens checked cross-document consistency, AC testability, id uniqueness and link integrity. All relative links in the five files resolve. Every frontmatter `relationships` target exists. Ids FR-084..087 and their AC/CON ids are new on every ref. FR-084-AC-8 and FR-086-AC-8 report the hardware run as "not executed" when no device is present, never as passed, which keeps them honest. The remaining findings are listed below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The edited FR-042 Behavior bullet still requires "a 60 second wall-clock timeout" for every request. The new `unless the request names a gesture` clause scopes only the non-interactive signer, so FR-042 and FR-085 (300 seconds with `gesture`) contradict each other. The FR-042 Description also still reads "the command gives git no terminal" after the new "Unless" sentence. Scope the timeout clause by the same `gesture` condition. | spec/functional/FR-042-create-signed-approval-tag-for-a-change-assurance-record.md:159-161; spec/functional/FR-085-approval-tag-gesture-signing-mode.md:62-63 |
| FND-002 | low | FR-085-AC-7 reads the timeout "from the timeout argument given to git when it is started", but git takes no timeout argument; the bound is the FR-019 process-host deadline. Say that the test reads the deadline the process host is given. | spec/functional/FR-085-approval-tag-gesture-signing-mode.md:99 |
| FND-003 | low | FR-085-AC-4 has a doubled word: "exit status 2 and and git not started". | spec/functional/FR-085-approval-tag-gesture-signing-mode.md:96 |

## Verdict

PASS-with-findings on this lens: one medium (FR-042 vs FR-085 timeout contradiction) and two lows. The AC testability problems of FR-084-AC-2..4 belong to the base review's FND-001, not repeated here.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-042 Behavior now scopes "no terminal and a 60 second wall-clock timeout" by "unless the request names a `gesture`", and defers to FR-085 for that case; the Description says the same. No contradiction with FR-085's 300 s remains. |
| FND-002 | fixed | FR-085-AC-7 now reads the deadline passed to the host that runs git. |
| FND-003 | fixed | FR-085-AC-4 reads "exit status 2 and git not started". |
