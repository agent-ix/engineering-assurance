---
id: SR-20261113170516
title: "Integrity review of release attestations and per-change receipts against wave 1 (engineering-assurance PR 232)"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-056, FR-066..FR-079, spec/non-functional/NFR-011 (PR #232 diff, head 5bab3f1), checked against FR-046..FR-065 and NFR-007..NFR-010"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-064
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-056
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-066
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-072
    type: references
---

# SR-20261113170516: Integrity review of release attestations and per-change receipts against wave 1 (engineering-assurance PR 232)

## Summary

Ticket: EA-97 (EA-98). Consistency of the new requirements with the merged Baselines/Release FRs (FR-046 to
FR-053) and the wave-1 sink and approval FRs (FR-054 to FR-065).

Checked clean: role evaluation always goes through the one FR-061 function (FR-067, FR-075, FR-078); policies
are read from the protected base branch everywhere (FR-067-AC-3, FR-075-AC-4, FR-078-AC-6); the
private store is written first (FR-071, FR-074, matching FR-054); the leak check precedes any public-log
write (FR-071, FR-076, matching FR-057); `release-verify` stays independent of attestations (FR-066-AC-4,
matching FR-053 and FR-065-AC-1); the change-record `ticket` reuses the FR-056 `linear_issue` grammar; the
record kinds `release_attestation` and `change_record` already exist in FR-056; FR-079 is consistent with
the FR-058 instruction to admit the prefix before writing the literal URI.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-068 and FR-078 trust "the workflow identity at the ref listed in the workflow-identity policy", but FR-064's closed policy schema allows only `role` `statement_writer` or `record_publisher`, and fixes `ref` to the protected base branch ref `refs/heads/<branch>`. The release-attestation and change-receipt identities cannot be listed, and a reusable workflow called from a `v*` tag push in the consumer repo signs with a Build Signer URI at the EA ref the caller pinned and a source ref `refs/tags/...`, which FR-064's rule would always reject. Fix: amend FR-064 in this PR: add roles `release_attester` and `receipt_writer`; state which certificate fields are matched for a reusable workflow (signer workflow = EA repository, `workflow_path`, signer ref as listed; source repository = the consumer; source ref = `refs/tags/v*` for releases, `refs/heads/<default>` for receipts); add ACs; and update FR-065's "each role is a separate identity" list. | spec/functional/FR-064-verify-approval-statement-with-workflow-identity-policy.md:33; FR-068:20-22; FR-078:20 |
| FND-002 | high | The public forms cannot be built. FR-071 says the public form carries "the digests of the attestation statements and the release commit" (four statement digests) and FR-076 publishes a three-statement receipt, but FR-056 closes `hashes` to the names `private_form`, `statement` and `subject_commit`, one value each. Fix: put only the verification-summary digest in `hashes.statement` (the summary names the other statements by digest) plus `subject_commit`, and say so in FR-071 and FR-076; or amend FR-056's `hashes` names. | spec/functional/FR-071-release-attestations-publish-through-the-sinks.md:22; spec/functional/FR-056-public-and-private-record-split.md:34 |
| FND-003 | medium | Verdict values do not match. FR-071 gives the `release` verdict as `released` or `failed`, but FR-067 writes nothing unless `release-verify` reports `released`, so `failed` has no trigger, and FR-047 names the other status `not_released`. FR-076 publishes a `change_record` verdict without mapping receipt `passed`/`failed` to FR-056's values (`recorded`, `failed`; there is no `passed`). Fix: state that a `release_attestation` record is published only with `released`, and that a receipt maps `passed` to `recorded` and `failed` to `failed`. | spec/functional/FR-071-release-attestations-publish-through-the-sinks.md:22; spec/functional/FR-076-receipt-storage-in-git-notes.md:22; spec/functional/FR-056-public-and-private-record-split.md:37 |
| FND-004 | medium | FR-066 Behavior "SHALL give all four statements the same subject set of artifact files" contradicts FR-068 (the summary also has the release commit as a subject) and FR-066-AC-1. Fix: "the same artifact-file subjects; the verification summary also carries the release commit subject of FR-068". | spec/functional/FR-066-release-attestation-set-and-subjects.md:36 |
| FND-005 | medium | The digest bytes are not defined and one is circular. `statement_digest`, `test_results`, `agent_bom`, `private_form` and the summary's input attestations say "the digest of" without naming the bytes (DSSE envelope, bundle, or RFC 8785 canonical statement), so two implementations disagree and the verifier cannot recompute. The FR-056 amendment puts "the full change record" in the private form, while the change-record predicate carries the digest of the private form. Fix: define each digest as `sha256:` over the RFC 8785 canonical bytes of the named JSON document (statement, BOM, private form), and define the private form's `change_record` as the text-bearing record without the predicate's `private_form` member. | spec/functional/FR-072-change-record-predicate.md:30-33; spec/functional/FR-056-public-and-private-record-split.md:48-52 |
| FND-006 | medium | FR-053 passes the obligations of an attestation that carries the release evidence snapshot to EA-97 ("belong to the attestation requirements of EA-97 and are not stated here"). FR-066's closed set of four has no snapshot statement and says nothing about the snapshot, so that handoff is left open. Fix: one sentence in FR-066 stating that the set does not carry the snapshot and that the release commit subject binds it, or add the obligation. | spec/functional/FR-053-snapshot-and-attestations-never-replace-the-decision.md:38; spec/functional/FR-066-release-attestation-set-and-subjects.md:24-26 |
| FND-007 | medium | FR-072 `approvals` entries are `{role, statement_digest, verdict}` for every required role, but a `missing` role has no statement to digest. The closed `failures` set has no code for a role evaluation that is `unavailable` (FR-063 `approval_source_unavailable`) or for statements that are `unverifiable` (FR-064). Fix: make `statement_digest` present if and only if `verdict` is `covered`; add the failure code `approvals_unavailable` and map FR-063/FR-064 unavailable outcomes to it; add an AC. | spec/functional/FR-072-change-record-predicate.md:30, 34 |
| FND-008 | low | The FR-056 amendment adds the private members `change_record` and `agent_bom` but leaves the Behavior bullet "SHALL build the private form from the record's ticket text, criteria text, model names and statement" unchanged. NFR-010's scope still lists only the approval items, although FR-068 and FR-074 rely on it. Fix: extend the bullet and NFR-010's Applies-to to FR-066 to FR-078. | spec/functional/FR-056-public-and-private-record-split.md; spec/non-functional/NFR-010-no-cryptography-of-our-own.md |

## Verdict

Not mergeable: FND-001 and FND-002 make the identity policy and the public record unbuildable as written.
Both have small, local fixes. FND-003 to FND-007 belong to the same fix round.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | Sink `none` now has competing error codes. A public repository on `none` gets `release_attestation_public_log_required` (FR-071 Behavior, FR-071-AC-3) and also `release_attestation_private_store_required` ("a repository whose resolved sink is `none`", FR-071-AC-5), and FR-067 adds a third, `release_roles_statements_not_retained` (FR-067-AC-7). FR-074 has the same pair for receipts (`receipt_public_log_required` and `receipt_private_store_required`; FR-074-AC-7 asserts both), and the evaluation order is unspecified, so the code reported depends on the implementation. Fix: check `none` first, whatever the visibility, with one code per feature (`release_attestation_private_store_required`, `receipt_private_store_required`); limit the `*_public_log_required` rules to a public repository on `private_store`; replace `release_roles_statements_not_retained` with FR-071's code, or state that it is never reached; update FR-071-AC-3 and FR-074-AC-7. | spec/functional/FR-071-release-attestations-publish-through-the-sinks.md:24 |
| FND-010 | medium | "Every digest in this specification is `sha256:` ... over the RFC 8785 canonical bytes of the named JSON document" overreaches. It contradicts FR-047 (the record digest is a git blob id and the release module computes no hash), FR-056 `subject_commit` (a git object id) and FR-066 (artifact files hashed over their raw bytes). `reviews[].artifact_digest` is a digest of a Markdown SpecReview file, not a JSON document, so the rule cannot apply to it and that member's old grammar line was removed. Fix: "every digest in this predicate"; define `artifact_digest` as `sha256:` plus 64 lowercase hex over the file's raw bytes; add it to FR-072-AC-7. | spec/functional/FR-072-change-record-predicate.md:26 |
| FND-011 | medium | FR-067 says "a squash or merge-commit merge of the release pull request must produce" the release commit. FR-047 requires the release commit to have exactly one parent, the baseline commit, and a merge commit has two, so `release-verify` always reports `release_commit_not_child_of_baseline` for that method. Fix: name squash, or a rebase merge whose tip is the release commit, as the methods that yield a valid release commit; state that a merge-commit merge yields `not_released`; add it to FR-067-AC-8. | spec/functional/FR-067-release-attestations-bind-to-the-signed-release-tag.md:28 |
| FND-012 | low | The new FR-064-AC-13 and FR-064-AC-14 have no tests.md rows. No wave-1 FR-054..FR-065 AC has one on main either, so this continues an existing gap rather than introducing a new one. Fix: add TC rows for the two new ACs (and, separately, for the wave-1 set). | spec/functional/FR-064-verify-approval-statement-with-workflow-identity-policy.md:97 |

## Dispositions

Round 1, head 428c2bd (fix commits 5bab3f1..428c2bd).

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
