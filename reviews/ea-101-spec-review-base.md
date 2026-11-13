---
id: SR-20261113085805
title: "Base spec review of the release evidence snapshot requirements (engineering-assurance PR 226)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/engineering-assurance; spec/spec.md, spec/functional/FR-047, FR-048 (edited), FR-051, FR-052, FR-053 (new) (PR #226 diff, head f8669b66)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-051
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-052
    type: references
---

# SR-20261113085805: Base spec review of the release evidence snapshot requirements (engineering-assurance PR 226)

## Summary

Ticket: EA-101. Review set `subset`: base checklist (including the hash/digest/pin antipattern
check and the failure-mode questions for `release-history`) plus `ears-conformance` (separate
SpecReview). The other analyses were not selected. No AssuranceProfile in the repository requires
an enforced `spec-review` operation for this change, so the review ran directly.

Examined the FR-047 and FR-048 edits, FR-051, FR-052, FR-053 and the `spec/spec.md` index entries.
Checked them for consistency with FR-041, FR-042, FR-046 to FR-050 and NFR-007, with the fixed
findings of the EA-92 review (reviews/ea-92-spec-review-base.md), and with the cross-references
to the companion Quoin specification (quoin PR 861: FR-166, FR-167, FR-168; quoin FR-063, FR-068
on main). `quire validate --scope . "spec/**/*.md"` exits 0 and reports no warning on any file
this PR adds or edits. No standard text is quoted and no roadmap or research content was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-052 adds `release-history`, a second release interface that Quoin consumes (quoin FR-168). FR-049, which this PR does not edit, still says the `release-verify` result is the only release interface Engineering Assurance offers to a consumer, and its Outputs call it "the only interface Quoin reads from a release". The two requirements contradict each other, and the EA-92 FND-014 fix that set this wording no longer holds. Fix: amend FR-049 (statement, split table, Outputs and data contract) to name the `release-history` result as the second interface, or change the FR-049 wording. | spec/functional/FR-049-release-ownership-split-and-data-contract.md:49-56; spec/functional/FR-052-release-history-since-the-previous-release.md:21-24 |
| FND-002 | medium | "Adds `snapshot_path`" is not defined when that path already exists at the baseline commit. This happens when a fixed path such as `release/snapshot.json` is reused after an earlier release commit was merged back, or when the path is a Baseline item or the Baseline file. The release commit then modifies the file, and the only outcome that applies is `release_commit_snapshot_not_added`, which is a valid but late refusal. Neither FR-048 nor FR-051 says that `snapshot_path` must be absent at the baseline commit or unique to each release, and no AC covers the modify case. Fix: state the rule (absent at the baseline commit), have the workflow check it before it emits the Release document, and add an AC. | spec/functional/FR-051-release-evidence-snapshot-placement.md:63; spec/functional/FR-048-human-release-decision-workflow.md:63-64 |
| FND-003 | medium | FR-052 first-parent semantics are underspecified. (a) It does not say that a merge commit's addition is judged against its first parent, or that the reported `commit` is then the merge commit and not the side-branch commit. (b) "Earliest" does not say whether it means first-parent order or commit date. (c) With `previous_release_tag` `null` it does not say which commit is reported as `commit`, that is, whether the walk goes to the root. Two implementers would emit different `commit` values. No AC covers a record carried by a merge commit or the `commit` value in the `null` case. | spec/functional/FR-052-release-history-since-the-previous-release.md:31-33; spec/functional/FR-052-release-history-since-the-previous-release.md:61-69 |
| FND-004 | medium | FR-052 does not define the matching rule. Quoin FR-063 makes `revision` an integer ≥ 1, but FR-052 gives no type for `revision` in the output and no grammar for `<record_id>` or `<revision>` when matching a path. Under string ordering, revision `10` sorts before `9`, and quoin FR-168 orders the same field independently. It also does not say whether non-matching names under `records_dir` are ignored or refused, for example `x.json.tmp`, a non-integer revision, or FR-068 intake staging directories. No AC tests the invalid `records_dir` forms. | spec/functional/FR-052-release-history-since-the-previous-release.md:31; spec/functional/FR-052-release-history-since-the-previous-release.md:49-51 |
| FND-005 | medium | FR-052 failure modes are incomplete. (a) Shallow clone: a grafted tag commit has no parent and is misreported as `release_history_previous_not_a_release_commit`, and a missing previous baseline is misreported as `not_ancestor`. No reason names an incomplete history. (b) Tag resolution does not say `refs/tags/<name>` only, so a branch or another ref with the same name can be picked. (c) It does not say what happens when a tag peels to a tree or a blob. (d) The single-parent check accepts any ordinary one-parent commit as a release commit, so the records that commit added are pulled into the range without a word. Fix: add a shallow-repository refusal and `refs/tags/` resolution, and state the non-commit case and the accepted weakness of (d). | spec/functional/FR-052-release-history-since-the-previous-release.md:63-66; spec/functional/FR-052-release-history-since-the-previous-release.md:84 |
| FND-006 | medium | FR-053-AC-2 contradicts FR-047. FR-047's request schema is closed ("Unknown members are rejected"), so a request that offers an attestation fails with `release_verify_request_invalid` and exit 2. It never yields `not_released` with an FR-041 status, which is what AC-2 expects. Fix: drop the "request" case, or expect `release_verify_request_invalid` for it, and keep the fixture case for an attestation file in the repository. | spec/functional/FR-053-snapshot-and-attestations-never-replace-the-decision.md:45; spec/functional/FR-047-release-record-and-signed-release-tag.md:58-63 |
| FND-007 | medium | FR-053 places obligations on the attestation emitter of EA-97, which it declares out of scope and which this repository does not specify: "An attestation ... SHALL take the snapshot bytes from the release commit" and "SHALL NOT alter ...". FR-053-AC-4 inspects bytes that no specified component produces, so it cannot be verified until EA-97 exists. Fix: move these obligations to the EA-97 requirements and keep only the `release-verify` and workflow independence statements here. Otherwise, state the dependency and mark AC-4 as verified with EA-97. | spec/functional/FR-053-snapshot-and-attestations-never-replace-the-decision.md:34-38; spec/functional/FR-053-snapshot-and-attestations-never-replace-the-decision.md:47 |
| FND-008 | low | The first clause of FR-051-AC-6 ("two release commits whose snapshots differ by one byte have different commit hashes") tests git's object model, not Engineering Assurance, and no defect in this repository can make it fail. Fix: drop the clause and keep the "no computed member" clause. | spec/functional/FR-051-release-evidence-snapshot-placement.md:82 |
| FND-009 | low | FR-051-AC-5, and FR-052-AC-6 likewise, mix a behavioural test with a static-inspection clause ("the release module contains no code that opens ...") under the `Test` method. Fix: split each inspection clause into its own `Inspection` AC or constraint. | spec/functional/FR-051-release-evidence-snapshot-placement.md:81; spec/functional/FR-052-release-history-since-the-previous-release.md:85 |
| FND-010 | low | FR-051 Behavior restates two FR-047 Behavior bullets word for word: `release_commit_snapshot_not_added` and `release_commit_changes_other_paths`. That gives the same check two normative sources. Fix: keep the bullets in FR-047 and reference them from FR-051. | spec/functional/FR-051-release-evidence-snapshot-placement.md:63-65; spec/functional/FR-047-release-record-and-signed-release-tag.md:115-120 |
| FND-011 | low | FR-048 takes `snapshot_path` as an input, but leaves it out of the run-identity refusal (Behavior bullet, AC-8) and does not check the Baseline-item path rules before it writes a Release document that `release-verify` would then refuse as `record_invalid`. | spec/functional/FR-048-human-release-decision-workflow.md:106-108; spec/functional/FR-048-human-release-decision-workflow.md:123 |
| FND-012 | low | The FR-051 ownership table gives "Reading the snapshot's members" to neither repository and names "the dossier documents" as the reader. Neither specification defines those documents, so that consumer has no owner. | spec/functional/FR-051-release-evidence-snapshot-placement.md:34 |
| FND-013 | low | The FR-052 frontmatter lacks relationships to `ix://agent-ix/quoin/FR-068` (the retained path it matches on, cited in Description and Dependencies) and to FR-043 (cited in Description). | spec/functional/FR-052-release-history-since-the-previous-release.md:1-14 |
| FND-014 | low | The workflow presents the separate evidence-status, SOUP and anomaly files, but it never presents the snapshot that the owner then commits and signs. Nothing ties the bound snapshot to what the owner reviewed. Consider presenting the snapshot file, or stating that signing the tag is the owner's acceptance of its content. | spec/functional/FR-048-human-release-decision-workflow.md:58-66; spec/functional/FR-051-release-evidence-snapshot-placement.md:47-49 |

## Verdict

FAIL until FND-001 is fixed. It is a direct contradiction between FR-049 and FR-052, and it
reopens EA-92 FND-014. FND-002 to FND-007 should be fixed in the same round. FND-008 to FND-014
are wording or traceability fixes.

The following hold and are recorded as clean:

- The amended release-commit rule is sound. The release commit stays the single child of the
  baseline commit and adds only the record and the snapshot. The baseline is untouched, so FR-046
  needs no change, and the tag's tree is the baseline plus two files. The reasons are listed and
  testable (FR-047-AC-6, FR-051-AC-1 to AC-3), and the FR-047-AC-12 property still covers every
  combination.
- The EA-92 fixes still hold: no sha256 (FND-002), the tagged tree is bounded (FND-004), the
  closed schemas carry no denylist (FND-009), and `release_commit_record_not_added` stays its own
  reason (FND-019).
- No digest, hash, manifest, pin or file tracking is added. The snapshot is bound only by the
  release commit hash, FR-047-CON-3 still forbids hashing code, and FR-052 opens and hashes no
  record.
- Ownership is consistent with Quoin FR-166: Quoin writes the snapshot and runs no git, and
  Engineering Assurance resolves the tag and checks presence only.
- Attestations never stand in for the signed decision. FR-053 behaviour and NFR-007 agree.
- The cross-references to quoin PR 861 agree: the protocol id
  `engineering-assurance.release-history/v1`, the members `previous_release {tag, commit}` and
  `records {record_id, revision, path, commit}`, the ordering, the uniqueness of
  `(record_id, revision)`, the full baseline commit hash, and the retained path
  `records/<record_id>/<revision>.json` (quoin FR-063, FR-068).
- EA-101's content criteria are addressed: ownership split, version history, the EA-97
  relationship and `quire validate`. The snapshot contents and format are on the Quoin side.
