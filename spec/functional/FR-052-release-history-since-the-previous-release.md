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
Approval status of a record is not part of this list (the trail of
[FR-043](./FR-043-approval-trail-of-a-change-assurance-record.md) reports it).

Out of scope: verifying that the previous tag is a signed release (that is `release-verify`), reading a record's
content, and any git write.

## Inputs

- One request document `engineering-assurance.release-history-request/v1` with `protocol`, `repository` (absolute
  directory), `baseline_commit` (full commit hash), `previous_release_tag` (a tag name or `null`) and `records_dir`
  (a repository-relative directory, no `..`, no leading `/`, no glob). Unknown members are rejected.

## Outputs

- One line of JSON, protocol `engineering-assurance.release-history/v1`, exit status 0, with `protocol`,
  `baseline_commit`, `previous_release` (`{ tag, commit }`, the commit being the release commit the tag peels to, or
  `null`) and `records`, each `{ record_id, revision, path, commit }`, where `commit` is the commit that added the
  file. `records` is ordered by `record_id`, then `revision` ascending, then `commit`.
- A request that yields no result writes the `engineering-assurance.error/v1` machine error and exits 2, with the
  codes `release_history_request_invalid`, `release_history_baseline_commit_unknown`,
  `release_history_previous_tag_unknown`, `release_history_previous_not_a_release_commit` and
  `release_history_previous_not_ancestor`.

## Behavior

- The command SHALL read history and files with git object access.
- The command SHALL NOT read the working tree.
- The command SHALL follow first-parent history from the baseline commit back to the previous baseline commit.
- If `previous_release_tag` is `null`, then the command SHALL list every matching file present at the baseline commit.
- If the tag peels to a commit that does not have exactly one parent, then the command SHALL report
  `release_history_previous_not_a_release_commit`.
- If the previous baseline commit is not an ancestor of, or equal to, the baseline commit, then the command SHALL
  report `release_history_previous_not_ancestor`.
- If the previous baseline commit equals the baseline commit, then the command SHALL write an empty `records`.
- The command SHALL list a record once for each `(record_id, revision)`.
- If the same path is added in two commits of the range, then the command SHALL report the earliest commit.
- The command SHALL NOT list a path that is absent at the baseline commit.
- The command SHALL NOT open, parse or hash a record file.
- The command SHALL NOT create, move, push or delete any ref.
- The command SHALL refuse a request with an unknown member, a relative `repository`, or a `-`-leading value with
  `release_history_request_invalid` before starting git.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-052-AC-1 | A history with a previous release tag, a baseline commit three first-parent commits later, and records added in two of them lists exactly those records with the adding commit of each, ordered by `record_id`, `revision`, `commit`. | Test |
| FR-052-AC-2 | A record added before the previous baseline commit is not listed, and a record added in the range then deleted before the baseline commit is not listed. | Test |
| FR-052-AC-3 | With `previous_release_tag` `null`, every matching file present at the baseline commit is listed and `previous_release` is `null`. | Test |
| FR-052-AC-4 | A previous release commit that was never merged into the baseline's branch still yields the records added after its parent. | Test |
| FR-052-AC-5 | A tag that peels to a two-parent commit, a tag that is unknown, a baseline commit that is unknown and a previous baseline that is not an ancestor yield `release_history_previous_not_a_release_commit`, `release_history_previous_tag_unknown`, `release_history_baseline_commit_unknown` and `release_history_previous_not_ancestor` with exit 2. | Test |
| FR-052-AC-6 | A record file edited in the working tree, or whose bytes are invalid JSON, does not change the result, and the release module contains no code that opens a record in this command. | Test |
| FR-052-AC-7 | The command leaves refs, index and working tree unchanged, and a request with an unknown member, a relative path or a `-`-leading value is refused with `release_history_request_invalid` before git starts. | Test |
| FR-052-AC-8 | Running the command twice over the same repository state yields byte-identical output. | Test |

## Dependencies

- **Upstream**: [FR-047](./FR-047-release-record-and-signed-release-tag.md),
  [FR-014](./FR-014-versioned-rust-boundary.md); `ix://agent-ix/quoin/FR-063` and `ix://agent-ix/quoin/FR-068` for the
  record and its retained path.
- **Downstream**: `ix://agent-ix/quoin/FR-168`, which takes the result as data.
