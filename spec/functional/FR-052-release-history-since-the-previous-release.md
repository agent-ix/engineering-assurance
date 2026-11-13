---
id: FR-052
title: "List the change-assurance records merged since the previous release"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-047"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-063"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-168"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-068"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-043"
    type: "references"
---

# FR-052: List the change-assurance records merged since the previous release

## Description

Tracked by Linear EA-101. Engineering Assurance SHALL provide a `release-history` command that lists the
change-assurance records (`ix://agent-ix/quoin/FR-063`) merged between the previous release and a baseline commit.
This is the version history of the release. Only git can say what was merged, so Engineering Assurance runs the git
part and writes the answer as data; Quoin takes the file as input (`ix://agent-ix/quoin/FR-168`) and runs no git.

The previous release is named by its tag. The tag peels to a release commit, which
([FR-047](./FR-047-release-record-and-signed-release-tag.md)) has the previous baseline commit as its single
parent. The history is read from that previous baseline commit, so a release commit that was never merged into the
branch does not hide the commits after it. A first release names no previous tag, and every record present at the
baseline commit is listed.

A record is listed when a commit on the first-parent history of the baseline commit, after the previous baseline
commit, added the file `<records_dir>/<record_id>/<revision>.json` (the retained path of `ix://agent-ix/quoin/FR-068`),
and the file is present at the baseline commit. The path alone identifies the record; the command opens no record.

The path grammar is exact. `<record_id>` is one path segment of 1 to 256 bytes that is not `.` or `..` and contains no
`/` (the bounds of `ix://agent-ix/quoin/FR-063`). `<revision>` is a decimal integer of at least 1 written without a
sign, without leading zeros and without a fraction. Any other file under `records_dir`, including a deeper path, a
name that does not end in `.json`, a temporary or staging name such as `x.json.tmp`, and a non-integer revision, is
not a record and is ignored without an error. `revision` is reported as an integer and ordered numerically.

First-parent semantics: a commit adds a file when the file is in its tree and not in its first parent's tree. A merge
commit is therefore judged against its first parent only, and the commit reported for a record added by a merged
branch is the merge commit, not a commit of the side branch. When a path is added by more than one commit of the
range, the earliest is the one nearest the previous baseline in first-parent order. With no previous tag the walk
runs to the root of the first-parent history, and the reported commit is the first commit on that history whose
tree has the file, with a root commit judged against an empty tree. Tag and commit dates play no part.
Approval status of a record is not part of this list (the trail of
[FR-043](./FR-043-approval-trail-of-a-change-assurance-record.md) reports it).

Out of scope: verifying that the previous tag is a signed release (that is `release-verify`), reading a record's
content, and any git write.

## Inputs

- One request document `engineering-assurance.release-history-request/v1` with `protocol`, `repository` (absolute
  directory), `baseline_commit` (a full lowercase hexadecimal commit hash of 40 or 64 digits), `previous_release_tag` (a tag name or `null`) and `records_dir`
  (a repository-relative directory: non-empty, no `..` segment, no leading `/`, no glob character). Unknown members
  are rejected.

## Outputs

- One line of JSON, protocol `engineering-assurance.release-history/v1`, exit status 0, with `protocol`,
  `baseline_commit` (the request's value, byte for byte), `previous_release` (`{ tag, commit }`, the commit being the release commit the tag peels to, or
  `null`) and `records`, each `{ record_id, revision, path, commit }`, where `commit` is the commit that added the
  file. `records` is ordered by `record_id`, then `revision` ascending, then `commit`.
- A request that yields no result writes the `engineering-assurance.error/v1` machine error and exits 2, with the
  codes `release_history_request_invalid`, `release_history_baseline_commit_unknown`,
  `release_history_previous_tag_unknown`, `release_history_previous_not_a_release_commit`,
  `release_history_previous_not_ancestor` and `release_history_repository_shallow`. `revision` in the result is an integer.

## Behavior

- The command SHALL read history and files with git object access.
- The command SHALL NOT read the working tree.
- The command SHALL follow first-parent history from the baseline commit back to the previous baseline commit.
- If `previous_release_tag` is `null`, then the command SHALL list every matching file present at the baseline commit.
- The command SHALL resolve `previous_release_tag` only as `refs/tags/<name>`.
- If no such tag exists, then the command SHALL report `release_history_previous_tag_unknown`, even when a branch of that name exists.
- If the repository is shallow, then the command SHALL report `release_history_repository_shallow` before it resolves a tag.
- If the tag peels to a tree or a blob, or to a commit that does not have exactly one parent, then the command SHALL report
  `release_history_previous_not_a_release_commit`.
- The command SHALL NOT verify that the tag is a signed release; a tag that peels to any one-parent commit is accepted,
  and the records that commit added then fall in the range. The caller names the previous release, and `release-verify`
  is what verifies one.
- The command SHALL judge a merge commit's additions against its first parent.
- The command SHALL report, for a path added by more than one commit of the range, the earliest in first-parent order.
- The command SHALL ignore a file under `records_dir` that does not match the path grammar, without an error.
- If the previous baseline commit is not an ancestor of, or equal to, the baseline commit, then the command SHALL
  report `release_history_previous_not_ancestor`.
- If the previous baseline commit equals the baseline commit, then the command SHALL write an empty `records`.
- The command SHALL list a record once for each `(record_id, revision)`.
- The command SHALL order `records` by `record_id` bytewise, then `revision` numerically, then `commit` bytewise.
- The command SHALL NOT list a path that is absent at the baseline commit.
- The command SHALL NOT open, parse or hash a record file.
- The command SHALL NOT create, move, push or delete any ref.
- The command SHALL refuse a request with an unknown member, a relative `repository`, or a `-`-leading value with
  `release_history_request_invalid` before starting git.
- The command SHALL refuse a `baseline_commit` that is not 40 or 64 lowercase hexadecimal digits with
  `release_history_request_invalid`, before starting git.
- The command SHALL refuse a `records_dir` that is empty, absolute, contains a `..` segment or contains a glob character
  with `release_history_request_invalid`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-052-AC-1 | A history with a previous release tag, a baseline commit three first-parent commits later, and records added in two of them lists exactly those records with the adding commit of each, ordered by `record_id`, `revision`, `commit`. | Test |
| FR-052-AC-2 | A record added before the previous baseline commit is not listed, and a record added in the range then deleted before the baseline commit is not listed. | Test |
| FR-052-AC-3 | With `previous_release_tag` `null`, every matching file present at the baseline commit is listed and `previous_release` is `null`. | Test |
| FR-052-AC-4 | A previous release commit that was never merged into the baseline's branch still yields the records added after its parent. | Test |
| FR-052-AC-5 | A tag that peels to a two-parent commit, a tag that is unknown, a baseline commit that is unknown and a previous baseline that is not an ancestor yield `release_history_previous_not_a_release_commit`, `release_history_previous_tag_unknown`, `release_history_baseline_commit_unknown` and `release_history_previous_not_ancestor` with exit 2. | Test |
| FR-052-AC-6 | A record file edited in the working tree, or whose bytes are invalid JSON, does not change the result. | Test |
| FR-052-AC-7 | The command leaves refs, index and working tree unchanged, and a request with an unknown member, a relative path or a `-`-leading value is refused with `release_history_request_invalid` before git starts. | Test |
| FR-052-AC-8 | Running the command twice over the same repository state yields byte-identical output. | Test |
| FR-052-AC-9 | A record added by a side branch and merged into the baseline's first-parent history is reported with the merge commit as `commit`; a path added in two commits of the range is reported with the earlier one in first-parent order, whatever the commit dates. | Test |
| FR-052-AC-10 | With `previous_release_tag` `null`, a record present from the root commit is reported with the first first-parent commit that holds it. | Test |
| FR-052-AC-11 | Revisions 9 and 10 of one record are listed in the order 9, 10; `x.json.tmp`, a `v2.json` name, a deeper path and a staging directory under `records_dir` are ignored without an error. | Test |
| FR-052-AC-12 | A `baseline_commit` that is a branch name, a short hash or an upper-case hash, and a `records_dir` that is empty, absolute, contains `..` or contains a glob character, are each refused with `release_history_request_invalid` before git starts. | Test |
| FR-052-AC-13 | A shallow repository yields `release_history_repository_shallow`; a branch with the name of a missing tag yields `release_history_previous_tag_unknown`; a tag that peels to a tree or a blob yields `release_history_previous_not_a_release_commit`. | Test |
| FR-052-AC-14 | Static inspection shows no code in this command that opens or hashes a record file. | Inspection |

## Dependencies

- **Upstream**: [FR-047](./FR-047-release-record-and-signed-release-tag.md),
  [FR-014](./FR-014-versioned-rust-boundary.md); `ix://agent-ix/quoin/FR-063` and `ix://agent-ix/quoin/FR-068` for the
  record and its retained path.
- **Downstream**: `ix://agent-ix/quoin/FR-168`, which takes the result as data.
