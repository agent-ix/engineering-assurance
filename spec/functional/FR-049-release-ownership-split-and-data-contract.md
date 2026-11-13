---
id: FR-049
title: "Own the release record and its verification; hand Quoin the result as data"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-047"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-048"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-052"
    type: "requires"
---

# FR-049: Own the release record and its verification; hand Quoin the result as data

## Description

Engineering Assurance SHALL own the Baseline and Release archetypes, the release
record and its verification
([FR-046](./FR-046-baseline-archetype.md),
[FR-047](./FR-047-release-record-and-signed-release-tag.md)). Quoin consumes the
release result as data and runs no git, as it already does for the change
approvals of
[FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md). The
split is:

| Concern | Engineering Assurance | Quoin |
| --- | --- | --- |
| Baseline and Release archetypes, schemas, skeletons | owns | none |
| Reading a commit, signature and tag verification, `release-verify` | owns, runs git | runs no git |
| The `release-verify` result | produces | retains it as data, never re-verifies |
| The `release-history` result ([FR-052](./FR-052-release-history-since-the-previous-release.md)) | produces, runs git | takes it as data, runs no git |
| SOUP report, evidence status, unresolved anomalies | reads as supplied files | produces them and supplies them as files |
| The release decision | records the person's signed decision | none |

There are two interfaces to a consumer. The first is the `release-verify` result of FR-047, which Quoin retains. The
second is the `release-history` result of FR-052, a list of change-assurance records that Quoin copies into the
release evidence snapshot (`ix://agent-ix/quoin/FR-168`). The data contract of the first is: the
`status`, the `reasons` list, the `approval` member, on `not_released` the
`absent_items` paths, and on `released` the `release`, `baseline`, `approver`,
`meaning`, `signed_at` and `accepted_anomalies` members. The files
Engineering Assurance reads from Quoin are the evidence-status, SOUP-report and
anomaly-list files named in
[FR-048](./FR-048-human-release-decision-workflow.md): the evidence
status of `ix://agent-ix/quoin/FR-115`, the SOUP report of
`ix://agent-ix/quoin/FR-144` and the anomaly report of
`ix://agent-ix/quoin/FR-162`, in the formats those requirements define.
Engineering Assurance copies no schema, reads only the members
FR-048 lists, and refuses an unreadable file with `release_input_unreadable`.

Out of scope: how Quoin presents, stores or reports the result, and any change
to a Quoin repository from this requirement.

## Outputs

- The `engineering-assurance.release-verify-result/v1` line, fixed by FR-047,
  and the `engineering-assurance.release-history/v1` line, fixed by FR-052, as
  the only interfaces Quoin reads from a release.

## Behavior

- Engineering Assurance SHALL offer the `release-verify` result and the
  `release-history` result as its only release interfaces to a consumer.
- Engineering Assurance SHALL NOT compute SOUP, evidence or anomaly status.
- The release-decision workflow SHALL read SOUP, evidence and anomaly status
  from supplied files only.
- The `release-verify` result SHALL NOT require its consumer to run git, read
  the repository or re-verify a signature to use it.
- Engineering Assurance SHALL NOT mutate a Quoin record.
- Engineering Assurance SHALL NOT embed a Quoin schema copy.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-049-AC-1 | Every member a consumer needs to retain a release (status, approver, meaning, signed time, baseline commit and items, accepted anomalies, the FR-041 approval) is present in one `released` result without a second command, and a `not_released` result carries its `reasons` and any `absent_items` paths. | Test |
| FR-049-AC-2 | The release module starts git itself and Quoin is not invoked by it; the result is the same when the Quoin executables are absent. | Test |
| FR-049-AC-3 | The evidence-status, SOUP-report and anomaly-list inputs are read from files and no code of the release module derives any of them from the repository. | Inspection |
| FR-049-AC-4 | The repository holds no copy of a Quoin schema or record for these inputs. | Inspection |

## Dependencies

- **Upstream**: [FR-047](./FR-047-release-record-and-signed-release-tag.md),
  [FR-052](./FR-052-release-history-since-the-previous-release.md),
  [FR-048](./FR-048-human-release-decision-workflow.md).
- **Downstream**: Quoin's release consumer, which is Quoin's own requirement
  and is tracked in the Quoin repository.
