---
id: FR-053
title: "Attestations may carry the snapshot but never replace the signed release decision"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-051"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-047"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-007"
    type: "references"
---

# FR-053: Attestations may carry the snapshot but never replace the signed release decision

## Description

Tracked by Linear EA-101, which states the relationship to the release attestations of Linear EA-97. A release
attestation may carry the release evidence snapshot ([FR-051](./FR-051-release-evidence-snapshot-placement.md)) as
the content of a statement, so that a standard verifier can read the evidence state beside a build or test
statement. The attestation is a copy of bytes that the signed release commit already holds. The signed tag remains
the only release decision: a release exists only when `release-verify`
([FR-047](./FR-047-release-record-and-signed-release-tag.md)) reports `released` from a named person's signed tag
([NFR-007](../non-functional/NFR-007-no-release-without-a-person.md)).

Out of scope: the attestation format, how it is emitted, and its verification, which belong to the attestation
requirements of EA-97.

## Behavior

- The `release-verify` result SHALL NOT depend on the presence, absence or validity of an attestation.
- `release-verify` SHALL NOT accept an attestation as the decision.
- `release-verify` SHALL NOT report `released` from an attestation.
- An attestation that carries the snapshot SHALL take the snapshot bytes from the release commit.
- An attestation that carries the snapshot SHALL NOT alter, extend or summarise the snapshot's members.
- The release-decision workflow SHALL NOT treat an attestation as the owner's choice.
- The repository SHALL NOT record a digest of the snapshot to compare with an attestation; the commit hash of the
  release commit is the only binding.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-053-AC-1 | A release commit with a valid snapshot and record but an unsigned tag yields `not_released` whatever attestation files exist, and a signed valid release yields the same `released` result with the attestation files removed. | Test |
| FR-053-AC-2 | A request or fixture that offers an attestation in place of a signed tag yields `not_released` with the FR-041 status the missing tag carries, never `released`. | Test |
| FR-053-AC-3 | Static inspection shows no attestation parsing in the release module and no code path in `release-verify` or the workflow that reads an attestation. | Inspection |
| FR-053-AC-4 | The snapshot bytes an attestation carries equal the bytes of `snapshot_path` read from the release commit, and the repository holds no snapshot digest. | Inspection |

## Dependencies

- **Upstream**: [FR-051](./FR-051-release-evidence-snapshot-placement.md),
  [FR-047](./FR-047-release-record-and-signed-release-tag.md),
  [NFR-007](../non-functional/NFR-007-no-release-without-a-person.md).
- **Downstream**: the attestation requirements of EA-97.
