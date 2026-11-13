---
id: FR-051
title: "Place the release evidence snapshot in the release commit and check it is there"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-047"
    type: "extends"
  - target: "ix://agent-ix/engineering-assurance/FR-049"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-166"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-167"
    type: "references"
---

# FR-051: Place the release evidence snapshot in the release commit and check it is there

## Description

Tracked by Linear EA-101. A release carries a release evidence snapshot: one file that Quoin writes
(`ix://agent-ix/quoin/FR-166`, format `ix://agent-ix/quoin/FR-167`) stating the evidence state of the baseline.
The person commits that file into the release commit beside the release record, and the signed release tag binds
it through git's own commit hash. No digest, hash, manifest or pin of the snapshot is recorded anywhere, because the
commit hash of the release commit already fixes every byte of the snapshot.

The ownership split, which extends that of [FR-049](./FR-049-release-ownership-split-and-data-contract.md):

| Concern | Engineering Assurance | Quoin |
| --- | --- | --- |
| Snapshot contents, format, determinism | none | owns |
| Writing the snapshot file | none | owns, runs no git |
| Resolving the release tag to its commit | owns, runs git | none |
| Checking the snapshot is present at that commit | owns | none |
| Reading the snapshot's members | none | none (the dossier documents read them) |
| Committing the snapshot and signing the tag | the person, with their own git | none |

### Why the release commit carries the snapshot

[FR-047](./FR-047-release-record-and-signed-release-tag.md) fixed the release commit as the single child of the
baseline commit that adds only the record. That rule is amended here to the record and the snapshot, for this
reason: the tag can bind only what its commit contains. A snapshot placed in the baseline commit would change the
baseline it describes and would have to be listed as a configuration item of the baseline it reports on. A snapshot
placed in a second commit after the release commit would break the single-parent rule and leave the tag pointing
before it. A snapshot bound by a digest in the record would add the digest this repository refuses to keep. A single
release commit that adds the record and the snapshot keeps one parent, leaves the baseline untouched, and lets the
tag's tree equal the baseline plus those two files and nothing else.

The snapshot is written from the baseline state. Quoin needs the baseline commit hash, not the release commit
hash, so no step needs a hash that does not yet exist. The order is: the snapshot is written, the person commits it
and the record, then signs the tag.

`release-verify` checks presence, never content. It does not parse the snapshot, so it embeds no Quoin schema
([FR-049](./FR-049-release-ownership-split-and-data-contract.md)).

Out of scope: the content and format of the snapshot, parsing it, deciding whether its evidence suffices, and any
git write.

## Behavior

- `release-verify` SHALL require `snapshot_path` in the Release document, as FR-047 defines it.
- `release-verify` SHALL resolve the release tag to its commit with git.
- `release-verify` SHALL test `snapshot_path` at that commit with git object access.
- If the release commit does not add `snapshot_path`, then `release-verify` SHALL report `release_commit_snapshot_not_added`.
- If the release commit changes a path other than `record_path` and `snapshot_path`, then `release-verify` SHALL
  report `release_commit_changes_other_paths`.
- If `snapshot_path` equals `record_path`, then `release-verify` SHALL report `record_invalid`.
- `release-verify` SHALL NOT parse the snapshot.
- `release-verify` SHALL NOT read the snapshot from the working tree.
- `release-verify` SHALL NOT record, compute or accept a digest, hash or manifest of the snapshot.
- The `released` result SHALL carry `snapshot_path` under `release`.
- The release-decision workflow SHALL NOT write, commit or stage the snapshot.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-051-AC-1 | A release commit that is the single child of the baseline commit and adds exactly the record and the snapshot yields `released` with `release.snapshot_path`, and the tag's tree differs from the baseline tree by those two paths only. | Test |
| FR-051-AC-2 | A release commit that adds the record but not `snapshot_path` yields `release_commit_snapshot_not_added`, with the other checks still evaluated. | Test |
| FR-051-AC-3 | A release commit that adds the record, the snapshot and one further path yields `release_commit_changes_other_paths`; a snapshot present only in the working tree, or added in a later commit, yields `release_commit_snapshot_not_added`. | Test |
| FR-051-AC-4 | A Release document whose `snapshot_path` equals the request's `record_path`, is absolute, contains `..` or a glob, or is missing is refused as `record_invalid`. | Test |
| FR-051-AC-5 | A snapshot whose bytes are not valid JSON, committed at `snapshot_path`, does not change the verdict, and the release module contains no code that opens the snapshot's members. | Test |
| FR-051-AC-6 | Two release commits whose snapshots differ by one byte have different commit hashes, and the result carries no member that Engineering Assurance computed from the snapshot's content. | Test |
| FR-051-AC-7 | The release-decision workflow run to a `release` choice leaves the repository unchanged and writes no snapshot. | Test |

## Dependencies

- **Upstream**: [FR-047](./FR-047-release-record-and-signed-release-tag.md),
  [FR-049](./FR-049-release-ownership-split-and-data-contract.md); `ix://agent-ix/quoin/FR-166` and
  `ix://agent-ix/quoin/FR-167` for the file.
- **Downstream**: [FR-053](./FR-053-snapshot-and-attestations-never-replace-the-decision.md).
