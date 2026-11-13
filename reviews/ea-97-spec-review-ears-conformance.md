---
id: SR-20261113170515
title: "EARS conformance review of release attestations and per-change receipts (engineering-assurance PR 232)"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/engineering-assurance; Behavior statements and ACs of spec/functional/FR-066..FR-079 and spec/non-functional/NFR-011 (PR #232 diff, head 5bab3f1)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-070
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-076
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-078
    type: references
---

# SR-20261113170515: EARS conformance review of release attestations and per-change receipts (engineering-assurance PR 232)

## Summary

Ticket: EA-97 (EA-98). The automated grammar pass (`quire validate` over the 18 changed files) reports zero
EARS and quality warnings. Manual pass over every Behavior bullet: each has a named subject (workflow,
writer, verifier, builder or command) and one SHALL, and negative requirements are separate bullets. The
findings below are statements that conform to the grammar but cannot be tested as written.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | "Each signing job SHALL declare the least permissions it needs and no more" has no measurable target, and FR-070-AC-2 checks against "this requirement's list", which the requirement never gives per job. Fix: replace it with the exact `permissions` block for each job of each file (see security FND-002). | spec/functional/FR-070-release-and-receipt-workflow-files.md:39, 47 |
| FND-002 | medium | Errors and reasons are not named: FR-076-AC-2 says "refused with a named error", and FR-078-AC-5 and the Behavior bullet say "`unverifiable` with a reason" / "a named reason" without the closed set. FR-064 enumerates its reasons. Fix: name `receipt_note_too_large`, and enumerate the `unverifiable` reasons of `receipt-verify-result/v1` (for example `note_absent`, `note_unreadable`, `trust_root_unreadable`, `policy_unreadable`). | spec/functional/FR-076-receipt-storage-in-git-notes.md:38; spec/functional/FR-078-receipt-verify.md:20, 41 |
| FND-003 | low | "The receipt workflow SHALL NOT block the push" is vacuous: a workflow triggered by `push` runs after the ref has moved and cannot block it, so FR-075-AC-3's second half always passes. Fix: drop the bullet and that half of AC-3, or restate it as "a failed receipt run SHALL NOT revert, rewrite or reset the default branch". | spec/functional/FR-075-push-to-main-receipt-workflow.md:36 |
| FND-004 | low | "SHALL fail visibly" (FR-067) uses an untestable adverb; the AC tests the named error. Fix: "SHALL fail with the named error ...". | spec/functional/FR-067-release-attestations-bind-to-the-signed-release-tag.md |

## Verdict

Conforms to EARS. FND-001 and FND-002 need fixing for the ACs to be testable; FND-003 and FND-004 are
wording.
