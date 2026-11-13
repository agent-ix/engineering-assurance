---
id: FR-047
title: "Verify a Release as a Baseline plus a named person's signed release tag"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-046"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-036"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-037"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-041"
    type: "requires"
---

# FR-047: Verify a Release as a Baseline plus a named person's signed release tag

## Description

Engineering Assurance SHALL provide a `Release` archetype and a `release-verify`
command. A Release is a Baseline ([FR-046](./FR-046-baseline-archetype.md)) read
at one commit plus a named person's release decision. The decision is a signed
annotated git tag. The `Release` document is the release record: it names the
Baseline file, the baseline commit hash, the person who is to decide
(`decision_owner`), and the anomalies that person accepted (`accepted_anomalies`).

The tag is the approval object of
[FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md). Its
message is a subject paragraph followed by `Approved-Record-Id` (the record
`id`), `Approved-Record-Digest` and a `Signed-off-meaning:` trailer that states
what the person decided (FR-037). `release-verify` adds no cryptography and
restates no signature rule: it runs the `approval-verify` verification of
FR-041 and adds only what a release needs.

The release tag is written by the person with `git tag -s` and their own
signing configuration, from the message the release-decision workflow
([FR-048](./FR-048-human-release-decision-workflow.md)) supplies. This reuses
git and adds no writer: `approval-tag`
([FR-042](./FR-042-create-signed-approval-tag-for-a-change-assurance-record.md))
accepts only a change-assurance record, and amending it to accept a Release
would add a second record kind to a writer that exists to compose one message,
which the plain git command and the verifier already cover. FR-042 is unchanged.

The release record is committed after the baseline commit, in a release commit
whose single parent is the baseline commit and whose only changes are the
addition of the record file at `record_path` and of the release evidence
snapshot at `snapshot_path`
([FR-051](./FR-051-release-evidence-snapshot-placement.md), which gives the
reason the snapshot rides in the same commit). The tag points at that release
commit, so the tag's tree is the baseline plus the record and the snapshot and
nothing else.
The record digest that binds the tag to the record is the object id git reports
for `<release commit>:<record_path>`, written `git-blob:` followed by that id;
Engineering Assurance reads it from git and computes no hash. The commit hash
of the release commit already fixes the record and snapshot bytes, so the baseline is
identified by its commit hash alone.

Out of scope: creating the tag, the release commit or any ref, deciding who
may release or which meaning suffices, and any release without a person's
signature.

## Inputs

- One request document `engineering-assurance.release-verify-request/v1` with
  `protocol`, `repository` (absolute directory), `tag`, `record_path`
  (repository-relative path of the release record), `allowed_signers` (absolute
  file) and optionally `gitsign`, with the bounds FR-041 sets for them. Unknown
  members are rejected.
- One Release document with a closed frontmatter schema: `id`, `title`,
  `type: Release`, `name`, `baseline` (an object with `path`, the
  repository-relative path of the Baseline file, and `commit`, the full commit
  hash), `snapshot_path` (the repository-relative path of the release evidence
  snapshot, with the path rules of a Baseline item and different from the
  request's `record_path`), `decision_owner` (a non-empty name that must equal the principal the
  trust input maps the signing key to) and optional `accepted_anomalies` (a
  list of strings).

## Outputs

- One line of JSON, protocol `engineering-assurance.release-verify-result/v1`,
  exit status 0 for every status. `status` is `released` or `not_released`.
  For `not_released`, `reasons` lists each failed check, every one of: an
  FR-041 status other than `approved`, `record_unreadable`, `record_invalid`,
  `baseline_commit_unknown`, `baseline_file_absent`, `baseline_file_invalid`,
  `release_commit_not_child_of_baseline`, `release_commit_changes_other_paths`, `release_commit_record_not_added`,
  `release_commit_snapshot_not_added`, `snapshot_path_present_at_baseline`, `baseline_item_absent`, `decision_owner_mismatch`.
- The `approval` member, the unchanged FR-041 result for the tag, whenever the
  record was readable and valid; it is absent, and FR-041 is not run, when the
  reasons include `record_unreadable` or `record_invalid`.
- On `released` only: `release` (record `id`, `name`, `snapshot_path`), `baseline` (`name`,
  `commit`, and each item `path`, `kind`), `approver`, `meaning`, `signed_at`
  with its source, and `accepted_anomalies`.
- On `not_released` with a resolved Baseline, `absent_items` lists the path of
  each Baseline item absent at the baseline commit; it is absent otherwise.
- A request that yields no verdict writes the `engineering-assurance.error/v1`
  machine error and exits 2, with the codes
  `release_verify_request_unreadable`, `release_verify_request_invalid`,
  `release_verify_allowed_signers_invalid`, `release_verify_git_failed` and
  `release_verify_result_encoding_failed`.

## Behavior

- The command SHALL read the release record from the commit the tag peels to,
  with git object access.
- The command SHALL NOT read the working tree.
- If git cannot answer a read the verdict needs (it cannot start, times out,
  exceeds its output bound, or exits with a status other than the one that
  means the object does not exist), then the command SHALL write the machine
  error `release_verify_git_failed` and exit 2, and SHALL NOT report a reason
  for the failure.
- The command SHALL read objects without replace refs and without a graft
  file.
- The command SHALL obtain `record_digest` from git as `git-blob:` followed by
  the object id of `<release commit>:<record_path>`.
- The command SHALL pass the record `id` and that digest as `record_id` and
  `record_digest` to the FR-041 verification.
- The command SHALL resolve the Baseline with the FR-046 resolution, at
  `baseline.commit` and `baseline.path`, and take `baseline.name` from the
  Baseline file.
- If the baseline commit is not in the repository, then the command SHALL
  report `baseline_commit_unknown`.
- If the release commit does not have exactly one parent equal to the baseline
  commit, then the command SHALL report `release_commit_not_child_of_baseline`.
- If the release commit changes any path other than `record_path` and `snapshot_path`, then the
  command SHALL report `release_commit_changes_other_paths`.
- If the release commit does not add `record_path`, then the command SHALL
  report `release_commit_record_not_added`.
- If the release commit does not add `snapshot_path`, then the command SHALL
  report `release_commit_snapshot_not_added`.
- If `snapshot_path` exists at the baseline commit, then the command SHALL
  report `snapshot_path_present_at_baseline`.
- The command SHALL report `released` only when the FR-041 status is
  `approved` and every other check passes.
- The command SHALL report every failing check it can evaluate.
- If the FR-041 `approver` differs from `decision_owner` by any byte, then the
  command SHALL report `decision_owner_mismatch`.
- The command SHALL derive the person only from the FR-041 `approver`.
- The command SHALL NOT derive a person from the record, the tagger, the
  author or an email.
- The command SHALL leave the `Signed-off-meaning` rule to FR-041.
- The command SHALL NOT decide what a meaning permits.
- The command SHALL NOT record, compute or accept a digest, hash or manifest of
  the baseline.
- The command SHALL NOT create, move, push or delete any ref.
- The command SHALL NOT report `released` from a record, a commit signature, a
  merge commit, a note or a lightweight tag.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-047-CON-1 | A result SHALL NOT be `released` unless the FR-041 status for the tag is `approved`. | Safety | Test |
| FR-047-CON-2 | The command SHALL contain no signature, certificate or trailer-parsing code of its own and no rule of FR-036, FR-037, FR-040 or FR-041 restated. | Architecture | Inspection |
| FR-047-CON-3 | The command SHALL contain no hashing code. | Architecture | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-047-AC-1 | A tag signed by the key the allowed-signers file maps to the record's `decision_owner`, whose message carries the record id, the `git-blob:` id of the record at the release commit and a `Signed-off-meaning` trailer, on a release commit that is the single child of the baseline commit and adds only the record and the snapshot, with all baseline items present, yields `released` with `approver`, `meaning`, `signed_at` and the baseline items. | Test |
| FR-047-AC-2 | A tag whose message carries the `git-blob:` id of a different version of the record (a record edited and recommitted after the message was composed, with the tag moved to the new release commit) yields `not_released` with FR-041 status `wrong_record` and `binding.record_digest` `mismatch`. | Test |
| FR-047-AC-3 | A tag signed by a mapped principal that differs from `decision_owner` yields `not_released` with reason `decision_owner_mismatch` and no `release` member. | Test |
| FR-047-AC-4 | An unsigned annotated tag, a lightweight tag, a bad signature, an unmapped key and a tag with no `Signed-off-meaning` each yield `not_released` carrying the corresponding FR-041 status. | Test |
| FR-047-AC-5 | A baseline commit not in the repository yields `baseline_commit_unknown`; a baseline item path absent at the baseline commit yields `baseline_item_absent` naming the path; a Baseline file absent or invalid at the baseline commit yields `baseline_file_absent` or `baseline_file_invalid`. | Test |
| FR-047-AC-6 | A release commit with two parents, a different parent, or a parent that is a descendant of the baseline commit yields `release_commit_not_child_of_baseline`; a release commit that also changes a path other than the record and the snapshot yields `release_commit_changes_other_paths`; a release commit that does not add `record_path` yields `release_commit_record_not_added`; a release commit that does not add `snapshot_path` yields `release_commit_snapshot_not_added`; a `snapshot_path` that already exists at the baseline commit, as a Baseline item, as the Baseline file or as an earlier snapshot, yields `snapshot_path_present_at_baseline`. | Test |
| FR-047-AC-7 | A record whose frontmatter is invalid, or that carries any member outside the closed schema, yields `record_invalid` with no `approval` member; a missing record path at the release commit yields `record_unreadable` with no `approval` member. | Test |
| FR-047-AC-8 | A record edited only in the working tree after tagging does not change the verdict, which is read from the release commit. | Test |
| FR-047-AC-9 | A signed commit, a merge commit or a note carrying the record trailers yields `not_released` and never `released`. | Test |
| FR-047-AC-10 | The `approval` member equals the output of `approval-verify` for the same tag, record id and digest, and the command refuses a request with an unknown member, a relative path or a `-`-leading value with `release_verify_request_invalid` and exit 2 before starting git. | Test |
| FR-047-AC-11 | Static inspection shows no signature, certificate, trailer-parsing or hashing code in the release module (CON-2, CON-3). | Inspection |
| FR-047-AC-12 | The decision from the FR-041 status and the record checks is exercised without any process, over every combination, and `released` is returned for exactly one. | Property |
| FR-047-AC-13 | A git that fails every call yields `release_verify_git_failed` at exit 2 and no verdict, and a replace ref or a graft in the repository changes neither the record read nor the parent compared. | Test |

## Dependencies

- **Upstream**: [FR-046](./FR-046-baseline-archetype.md),
  [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md),
  [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md),
  [FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md).
- **Downstream**: [FR-048](./FR-048-human-release-decision-workflow.md),
  [FR-051](./FR-051-release-evidence-snapshot-placement.md),
  [FR-049](./FR-049-release-ownership-split-and-data-contract.md).
