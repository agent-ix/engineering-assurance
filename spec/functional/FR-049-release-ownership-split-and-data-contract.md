---
id: FR-049
title: "Own the release record and its verification; hand Quoin the result as data"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-047"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-048"
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
| SOUP report, evidence status, unresolved anomalies | reads as supplied files | produces them and supplies them as files |
| The release decision | records the person's signed decision | none |

The data contract Quoin consumes is the `release-verify` result of FR-047: the
`status`, the `approval` member, and on `released` the `release`, `baseline`,
`approver`, `meaning`, `signed_at` and `accepted_anomalies` members. The files
Engineering Assurance reads from Quoin are the evidence-status, SOUP-report and
anomaly-list files named in
[FR-048](./FR-048-human-release-decision-workflow.md): the output of Quoin's
evidence, SOUP and anomaly reporting, in the format Quoin publishes. The
protocol identifiers of those outputs are Quoin's to name in its own
requirement; Engineering Assurance copies no schema, reads only the members
FR-048 lists, and refuses an unreadable file with `release_input_unreadable`.

Out of scope: how Quoin presents, stores or reports the result, and any change
to a Quoin repository from this requirement.

## Outputs

- The `engineering-assurance.release-verify-result/v1` line, fixed by FR-047,
  as the only interface Quoin reads from a release.

## Behavior

- Engineering Assurance SHALL offer the `release-verify` result as its only
  release interface to a consumer.
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
| FR-049-AC-1 | Every member a consumer needs to retain a release (status, approver, meaning, signed time, baseline commit and items, accepted anomalies, the FR-041 approval) is present in one `released` result without a second command. | Test |
| FR-049-AC-2 | The release module starts git itself and Quoin is not invoked by it; the result is the same when the Quoin executables are absent. | Test |
| FR-049-AC-3 | The evidence-status, SOUP-report and anomaly-list inputs are read from files and no code of the release module derives any of them from the repository. | Inspection |
| FR-049-AC-4 | The repository holds no copy of a Quoin schema or record for these inputs. | Inspection |

## Dependencies

- **Upstream**: [FR-047](./FR-047-release-record-and-signed-release-tag.md),
  [FR-048](./FR-048-human-release-decision-workflow.md).
- **Downstream**: Quoin's release consumer, which is Quoin's own requirement
  and is tracked in the Quoin repository.
