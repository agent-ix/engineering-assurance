---
id: SR-20261113085803
title: "Base spec review of the Baseline/Release requirements (engineering-assurance PR 225)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/engineering-assurance; README.md, spec/spec.md, spec/functional/FR-046..FR-050, spec/non-functional/NFR-007 (PR #225 diff, head 55e222ae)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-047
    type: references
---

# SR-20261113085803: Base spec review of the Baseline/Release requirements (engineering-assurance PR 225)

## Summary

Ticket: EA-92. Review set `subset`: base checklist (including the hash/digest/pin antipattern
check) plus `ears-conformance` (separate SpecReview). Not selected: failure-domain, integrity,
dependency, evidence, risk-complexity, scope-boundary; the cross-requirement consistency checks
against FR-005, FR-036, FR-037, FR-041 and FR-042 were done in this base review. No
AssuranceProfile in the repository requires an enforced `spec-review` operation for this change,
so the review ran directly.

Examined FR-046, FR-047, FR-048, FR-049, FR-050, NFR-007, the README stance sentence and the
`release-verify` row, and the `spec/spec.md` index entries. `quire validate --scope . "spec/**/*.md"`
exits 0 with no errors and 10 grammar warnings on the new files (see the ears-conformance review).
No standard text is quoted and no roadmap or research content was found.

All six spec content criteria of EA-92 are addressed. Problems: the named tag writer
(FR-042 `approval-tag`) refuses a Release record, Engineering Assurance gains a sha256 computation
that duplicates the identity the signed tag's commit already fixes, one acceptance criterion
describes a case that cannot happen, and the ancestor rule lets the tagged tree differ from the
baseline.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-047 and FR-048 say `approval-tag` (FR-042) writes the release tag. FR-042 only accepts a change-assurance JSON record (`schema_version` 2, `record_type` `change_assurance`, `review_workflow.signed_approval`) and refuses any other file with `approval_tag_record_unresolved` (FR-042-AC-7). It refuses a Markdown Release document, so the specified path cannot create a release tag. Fix: amend FR-042 in this PR to accept a Release record, or specify a release-tag writer. | spec/functional/FR-047-release-record-and-signed-release-tag.md:31; spec/functional/FR-048-human-release-decision-workflow.md:64; spec/functional/FR-042-create-signed-approval-tag-for-a-change-assurance-record.md:100 |
| FND-002 | high | Hash antipattern. FR-047 makes Engineering Assurance compute `sha256:` of the release record bytes for `Approved-Record-Digest`. FR-047 reads the record only from the commit the signed tag names, so that commit hash already fixes every byte of the record. The sha256 is a second identity for content git already fixes. It also reverses FR-042's stated boundary that Engineering Assurance has no hashing and never computes a digest. The claim that it is not a baseline digest is correct, but it is still ceremony. Fix: remove the computation. Bind the release by `Approved-Record-Id` and the signed tag's target commit, either through an id-only release binding in FR-041 or by passing the record identity git already reports for `<tagged commit>:<record_path>`, with no hashing in Engineering Assurance. | spec/functional/FR-047-release-record-and-signed-release-tag.md:44; spec/functional/FR-047-release-record-and-signed-release-tag.md:85 |
| FND-003 | high | FR-047-AC-2 can never fail as written. "Changing one byte of the release record after the tag was signed" either edits the working tree, which AC-7 says leaves the verdict unchanged, or makes a new commit, which the signed tag does not point at. In both cases the record read at the tagged commit still matches the digest. `wrong_record` arises only when the signer tags a commit whose record differs from the digest in the request. This AC falls with FND-002. | spec/functional/FR-047-release-record-and-signed-release-tag.md:116 |
| FND-004 | medium | Ancestor rule. The tagged commit may be any descendant of the baseline commit, so the tree at the release tag can change any baseline item after the baseline commit. A consumer who checks out or builds the tag gets content that is not the baseline. Fix: require the tagged commit's only parent to be the baseline commit and its only change to be `record_path`, or state that the tagged tree is not the release and consumers use `baseline.commit`. | spec/functional/FR-047-release-record-and-signed-release-tag.md:39; spec/functional/FR-047-release-record-and-signed-release-tag.md:119 |
| FND-005 | medium | FR-047 does not say at which commit the Baseline file (`baseline.path`) is read, where the output `baseline.name` comes from, or which reason is given when the Baseline file is absent or invalid at the baseline commit. Only `baseline_item_absent` exists. FR-046 makes the Baseline file's own absence an "absent path" with no items to list, so the result shape is undefined in that case. | spec/functional/FR-047-release-record-and-signed-release-tag.md:60; spec/functional/FR-046-baseline-archetype.md:56 |
| FND-006 | medium | FR-047 Outputs say the result always carries `approval`, the unchanged FR-041 result. For `record_unreadable` and `record_invalid` there is no record `id` to pass, and FR-041 requires a non-empty `record_id`, so the FR-041 verification cannot run and the "always" member cannot exist. Fix: state that `approval` is absent, and FR-041 is not run, when the record is unreadable or invalid. | spec/functional/FR-047-release-record-and-signed-release-tag.md:74 |
| FND-007 | medium | FR-048 sequencing gap. The workflow writes the Release document and emits "the exact `approval-tag` request". FR-047 needs the tag to point at a commit that already holds the record, and the digest is taken from the bytes at that commit. No requirement says who commits the record, and the request's `revision` cannot name a commit that does not yet exist. Fix: state that the owner commits the record, how the request's revision is then supplied, and add an AC for it. | spec/functional/FR-048-human-release-decision-workflow.md:62 |
| FND-008 | medium | FR-049 says the evidence-status, SOUP-report and anomaly-list formats "are Quoin's and are read as given". FR-048 still requires Engineering Assurance to show each evidence item's FR-004 state and to decide which anomalies are unresolved, which means parsing those files. No contract (protocol id or producing command) and no behaviour for a malformed file is named, so two implementers would differ. Fix: name the Quoin output each file comes from and the refusal for unreadable input, without copying the schema. | spec/functional/FR-049-release-ownership-split-and-data-contract.md:33; spec/functional/FR-048-human-release-decision-workflow.md:32 |
| FND-009 | medium | Checks over tracking. FR-046 rejects a fixed list of member names (`commit`, `digest`, `hash`, `sha`, `checksum`, `files`) in its Behavior and FR-046-AC-2. FR-047-AC-6 adds a "digest, hash or manifest member" check, and NFR-007 counts digest members. These lists are easy to bypass (`sha256`, `checksums` and `manifest` are not in FR-046's list) and are checks over tracking themselves. The repository's frontmatter schemas are already closed (`additionalProperties: false`). Fix: specify closed Baseline and Release schemas that reject unknown members, and delete the denylist behaviour, FR-046-AC-2, the second clause of FR-047-AC-6 and NFR-007 metric 3. | spec/functional/FR-046-baseline-archetype.md:49; spec/functional/FR-047-release-record-and-signed-release-tag.md:120; spec/non-functional/NFR-007-no-release-without-a-person.md:42 |
| FND-010 | medium | This spec-only PR edits the public README to list a `release-verify` command and to state a release-recording capability that `src/` does not implement. Readers are told about a command that does not exist. Fix: land the README edits with the FR-047 and FR-050 implementation. FR-050 stays as the requirement. | README.md:11; README.md:298 |
| FND-011 | medium | FR-046 resolution has no entry point. The text says only "when a Baseline is resolved" and names no command, request protocol or library boundary, yet FR-046-AC-4 to AC-6 and FR-047/FR-048 depend on it. Fix: name the interface, for example an internal function used by `release-verify` and the workflow, or a command with its request and result protocol. | spec/functional/FR-046-baseline-archetype.md:53 |
| FND-012 | low | The new `Baseline` archetype name collides with the existing MeasurementPlan `baseline` member and the `Baseline` enum in `src/measurement.rs`, which mean a reference value in a decision rule. Consider a distinct name, or state the distinction. | spec/functional/FR-046-baseline-archetype.md:10; src/measurement.rs:433 |
| FND-013 | low | Two wording problems in FR-047 Outputs. The `reasons` sentence ("each one of the FR-041 statuses other than `approved`, `record_unreadable`, ...") reads as if it excludes the release reasons. Exit 2 names no error codes, while FR-041 lists its own. The "that commit or" branch of the ancestor rule can never be reached, because a record cannot name the commit that holds it, as FR-046 itself states. | spec/functional/FR-047-release-record-and-signed-release-tag.md:70 |
| FND-014 | low | FR-049 says "Engineering Assurance SHALL be the only component that reads git objects to verify a release". That constrains components this repository does not own and cannot be verified here. Fix: restate it as the `release-verify` result being the only release interface Engineering Assurance offers. | spec/functional/FR-049-release-ownership-split-and-data-contract.md:51 |
| FND-015 | low | FR-048 does not say what happens when the owner names an accepted anomaly id that is not in the supplied anomaly list. | spec/functional/FR-048-human-release-decision-workflow.md:45 |
| FND-016 | low | FR-050-AC-1 "in a way that rules out recording one" is a subjective test. FR-050-AC-2 already states the positive check. | spec/functional/FR-050-record-named-persons-release-decision-stance.md:38 |

## Verdict

FAIL. Not ready for implementation until FND-001 to FND-003 are fixed. FND-002 is a P0 hash
finding and the fix is removal. FND-004 to FND-011 should be fixed in the same round. FND-012 to
FND-016 are wording.

The following hold and are recorded as clean:

- The baseline is identified by its commit hash alone, and no baseline digest or manifest is
  recorded.
- `release-verify` reuses FR-041 and adds no signature or trailer code (CON-2).
- The person is derived only from the FR-041 `approver`.
- The workflow cannot sign, push or choose, and it keeps the `hitl` gate and run-id refusal of
  FR-005.
- Quoin runs no git and consumes the result as data, and Engineering Assurance embeds no Quoin
  schema copy.
- No standard text is quoted and no roadmap content is present.
