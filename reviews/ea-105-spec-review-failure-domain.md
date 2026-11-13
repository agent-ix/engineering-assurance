---
id: SR-20261113163614
title: "Failure-domain review of the GitHub approval source and role gate (engineering-assurance PR 231)"
type: SpecReview
analysis: failure-domain
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-054..FR-065, spec/non-functional/NFR-008..NFR-010, spec/spec.md (PR #231 diff, head dd5a124)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-059
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-060
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-063
    type: references
---

# SR-20261113163614: Failure-domain review of the GitHub approval source and role gate (engineering-assurance PR 231)

## Summary

Ticket: EA-105 and PLAT-1685. Examined review-state, identity, commit-binding and lookup failure modes of FR-059 to FR-064. Covered well: dismissed and non-approved reviews, missing or empty or mid-line meaning, a push after review (FR-060-AC-3), unmerged pull requests, squash and merge-commit binding, tree similarity never inferred (FR-060-AC-5), unclassifiable identity treated as not human, unstaffed role with no statements.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | An approval is "a review in the `APPROVED` state that has not been dismissed". On GitHub an earlier APPROVED review keeps that state when the same reviewer later submits CHANGES_REQUESTED on the same head; the reviewer's effective verdict is their latest review. As written, a withdrawn approval still yields a statement. Fix: count only the reviewer's latest non-COMMENTED review on the merged head, and add an AC (APPROVED then CHANGES_REQUESTED on the same head yields no approval). | spec/functional/FR-059-source-adapter-interface-and-github-review-adapter.md:28-30, 43 |
| FND-002 | medium | The approver is "the account login together with the account's numeric id", but nothing says which of the two the policy's account lists and the distinct-individual check key on, nor how the pair is encoded in the single `approver` string FR-058 copies byte for byte. A login can be renamed and later re-registered by someone else, so a policy that lists logins grants the role to whoever holds the name. Fix: key policy matching and distinctness on source plus stable numeric id, and define the `approver` encoding. | spec/functional/FR-059-source-adapter-interface-and-github-review-adapter.md:29, 47 |
| FND-003 | medium | An unstaffed role is one whose listed accounts are none or all not human, reported even with no statements. For listed accounts that never reviewed, classification needs a source lookup of the account type; a failed lookup would then classify the account as not human (FR-062) and report `approval_role_unstaffed`, a configuration error, for what is a transient outage. Fix: state the classification inputs for listed accounts and give a lookup failure its own result. | spec/functional/FR-063-unstaffed-required-role-fails-visibly.md:15-18 |
| FND-004 | low | For a rebase merge of several commits the source reports one merge-result commit, so only the tip is the statement subject and the other rebased commits on the base branch carry no approval. Fix: state that the tip is the only subject for a rebase merge, or make every rebased commit a subject. | spec/functional/FR-060-approval-binds-to-the-merged-head-commit.md:32, 41 |

## Verdict

Four real edges are unspecified; FND-001 to FND-003 should be fixed in this PR, FND-004 may be.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
