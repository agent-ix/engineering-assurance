---
id: SR-20261113170518
title: "Failure-domain review of release attestations and per-change receipts (engineering-assurance PR 232)"
type: SpecReview
analysis: failure-domain
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-066..FR-078 (PR #232 diff, head 5bab3f1)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-067
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-069
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-076
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-077
    type: references
---

# SR-20261113170518: Failure-domain review of release attestations and per-change receipts (engineering-assurance PR 232)

## Summary

Ticket: EA-97 (EA-98). Unstated failure modes, identity confusion and topological edges.

Assessment of author decision 4 (release roles are evaluated over the PR that lands the release commit,
and the person-signed tag stays the only release decision): it is sound as an additional precondition on
attesting. It does not touch `release-verify`, so NFR-007 and FR-053 hold. It is not complete; see FND-001.

Assessment of decision 5 (fail closed for a public repository on a non-public sink): it is sound and
consistent with FR-057. It leaves the release `released` (FR-053), so failing the attestation workflow cannot
be mistaken for failing the release, provided FR-071's `failed` verdict is resolved (integrity FND-003).

Checked clean: failing changes still get receipts; unknown status counts as missing; a valid failed receipt is
`failed_receipt`, not missing; tag tampering after attestation is caught through the summary's commit subject.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Decision 4 is incomplete. FR-067 assumes "the release commit lands through a pull request" but does not define (a) how the PR is found: the approval statements whose `merged_commit` equals the commit the tag peels to; (b) the case of no such PR: a direct push, or a rebase-merged PR where the release commit is not the tip and so has no statements (FR-060); (c) the race in which the tag is pushed before the statement writer has run; (d) a repository on sink `none`, where FR-064 returns `statements_not_retained`, so no release can ever be attested; (e) that FR-047 requires the release commit's single parent to be the baseline commit, so a squash merge must produce the tagged commit and the tag must point at the merged commit, not the PR head. FR-065-AC-5 also requires that no code in the release module reads an approval statement, so the role step must sit outside that module. Fix: specify (a) to (e) with named errors (for example `release_roles_no_pull_request`, and `release_roles_unverifiable` as retryable) and ACs, and state that the role step lives in the attestation workflow code, not the release module. | spec/functional/FR-067-release-attestations-bind-to-the-signed-release-tag.md:24, 33 |
| FND-002 | medium | FR-069 carries "failed and stale results ... exactly as the export states them", but the in-toto Test Result v0.1 predicate has only `PASSED`/`WARNED`/`FAILED` and the `passedTests`/`warnedTests`/`failedTests` lists, and FR-066 forbids adding members. "Stale" has no representation, so FR-069-AC-1 cannot be built. Fix: define the mapping (a stale result is listed in `failedTests`, and the overall `result` is `FAILED` when any test failed or is stale) in the Quoin export requirement (scope-boundary FND-001), and cite it in FR-069. | spec/functional/FR-069-test-result-export-is-consumed-as-data.md:16, 22, 30 |
| FND-003 | medium | Notes push race. Two pushes to `main` in quick succession run two receipt workflows that each add a note and push `refs/notes/assurance-receipts`, and the second push is rejected as non-fast-forward, so a receipt is lost. FR-076-AC-1 covers sequential pushes only. Fix: require fetch, `git notes merge` (or `cat_sort_uniq`) and a bounded retry, with a named error on exhaustion, and an AC with two concurrent writers. | spec/functional/FR-076-receipt-storage-in-git-notes.md:20, 36 |
| FND-004 | medium | Sink `none` is not handled for receipts. FR-075 reads the receipt input and the approval statements from the private store, which does not exist for `none` (FR-055, FR-064), so every receipt silently becomes `contract_missing` or `approvals_incomplete` instead of naming the cause. Fix: state the outcome for `none` (refuse with a named code, for example `receipt_private_store_required`, or define what the receipt records) and add an AC. | spec/functional/FR-075-push-to-main-receipt-workflow.md:22; spec/functional/FR-074-per-change-receipt-bundle-records-failures-too.md:26 |
| FND-005 | medium | FR-071-AC-4 assumes every private repository can attest through the platform's private signing instance. That depends on the hosting plan, and when it is unavailable the spec says nothing, which risks an implementer falling back to the public log. Fix: when the private path is unavailable, fail with a named code (for example `release_attestation_private_signing_unavailable`) and never fall back to the public path; add an AC. | spec/functional/FR-071-release-attestations-publish-through-the-sinks.md:24, 42 |
| FND-006 | low | The scheduled missing-receipt check reports as `no_note` a commit whose receipt workflow is still running, and does not handle an `adopted_at` that is not on the first-parent history. Fix: exclude commits whose receipt run is still in progress (or newer than a stated window), and refuse an `adopted_at` that is not a first-parent ancestor with a named error. | spec/functional/FR-077-missing-receipt-check.md:20 |

## Verdict

Mergeable on failure-domain grounds once FND-001 to FND-005 are specified. None needs a redesign, but each is
a branch the implementation would otherwise have to invent.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | `git notes merge -s cat_sort_uniq` concatenates and sorts the notes when both sides have a note for the same commit: a re-run of the receipt workflow, or a retry after `receipt_note_conflict`. That gives a six-line note with the line order lost, while FR-076 specifies three lines and FR-078 reads one receipt. Fix: the `notes` job skips a commit that already has a verified receipt; FR-078 identifies the three statements by predicate type, not line position, and specifies the result for more than three lines (for example: group by `record_id`, the latest verified receipt wins); add an AC for a re-run on the same commit. | spec/functional/FR-076-receipt-storage-in-git-notes.md:22 |
| FND-008 | medium | The `grace_minutes` exclusion has no defined clock, and the in-progress exclusion has no bound. Commit dates are set by the author, so a bypass push with a future committer date would never be listed. "In progress, or newer than `grace_minutes`" also lets a stuck run hide a commit forever. Fix: measure age from the time the commit became reachable from the default branch, as the platform's push event reports it, never from commit dates; exclude an in-progress run only while it is within `grace_minutes`; add an AC with a future-dated commit. | spec/functional/FR-077-missing-receipt-check.md:29 |
| FND-009 | low | FR-067's resolution has no outcome for a failed source lookup of the merged pull request, the analogue of FR-063's `approval_source_unavailable`. An implementer would report `release_roles_no_pull_request` for a transient failure. Fix: add a retryable `release_roles_source_unavailable` with an AC. | spec/functional/FR-067-release-attestations-bind-to-the-signed-release-tag.md:30 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | medium | FR-078 groups a note's lines "by `record_id`", but only the change-record predicate has a `record_id`. The verification summary may carry no custom member (FR-068/FR-074 use the third-party VSA as defined), and the test-result statement has none either, so two of the three statements cannot be assigned to a group. (The round-1 fix suggested this key; the suggestion was wrong.) Fix: group by the summary: each summary's input attestations name its change-record and test-result statements by digest, so a group is a summary plus the two statements whose digests it lists; update FR-078-AC-7. | spec/functional/FR-078-receipt-verify.md:20 |
| FND-011 | medium | FR-077 measures age "from the push time of the workflow run", but the commits this check exists for (a ruleset bypass, or a push whose receipt workflow never ran) have no receipt run to report a push time. The age source is therefore undefined exactly where it matters, and an implementer may treat such commits as young. Fix: take the push time from the platform's record of pushes to the default branch, independent of any workflow run (for GitHub, the repository activity record); list a commit whose push time cannot be determined as `status_unknown`; add an AC for a bypass push with no run. | spec/functional/FR-077-missing-receipt-check.md:29 |

## Dispositions

Round 1 rows (findings of the review pass) at head 428c2bd; round 2 rows (findings of disposition pass 1, listed after them) at head a6895e7.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | fixed | - |
| FND-009 | fixed | - |
