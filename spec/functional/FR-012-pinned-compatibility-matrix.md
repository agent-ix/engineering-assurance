---
id: FR-012
title: "Pin the shared assurance contract versions"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-005"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-011"
    type: "requires"
---

# FR-012: Pin the shared assurance contract versions

## Description

Engineering Assurance SHALL publish one reviewed compatibility matrix naming
the exact released versions and artifact digests of the shared assurance
components, and SHALL classify an observed toolchain against it.

An enforcing repository migration SHALL NOT begin until a human records
acceptance of that matrix.

## Inputs

- The released Quire CLI and its engine, with their source revisions.
- The released Quoin, providing the evidence, measurement, attestation,
  intake, audit, and receipt surfaces.
- The released ix-flow providing human decision events.
- This repository's own released tag and the digests of its schemas.
- The accepted compatibility corpus, pinned as a submodule gitlink.

## Outputs

- `engineering_assurance/compatibility-matrix.json`, naming each component's
  released version, the versions it rules out and why, and the digest of every
  artifact the matrix identifies.
- A classification of an observed toolchain as compatible, incompatible, or
  unknown, per component, with the reason.
- Upgrade order and per-component rollback notes.

## Behavior

- Every pin SHALL name a released artifact. A branch name, a bare revision, or
  a floating tag SHALL NOT appear as a version.
- An observed version equal to the pin SHALL classify as compatible.
- A version the matrix names and rules out SHALL classify as incompatible, with
  the recorded reason.
- A version the matrix has never seen SHALL classify as unknown. Unknown SHALL
  NOT satisfy the gate, and SHALL NOT be reported as incompatible.
- A component that could not be observed SHALL classify as unknown.
- The gate SHALL require every pinned component to be compatible.
- The classifier SHALL execute nothing. Observing the environment SHALL be a
  separate program.
- Publication of these versions SHALL leave every campaign repository's
  workflows on manual dispatch only.

## Error Conditions

An unknown matrix version, a matrix missing its acceptance, gate, component or
rollback sections, a matrix pinning no component, and an unknown component name
are each refused with a `MatrixError`. An artifact whose bytes no longer match
its recorded digest is reported by path; an artifact the matrix names and this
tree does not contain is skipped rather than reported as drift.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-012-CON-1 | The classifier SHALL execute no subprocess and write no file. | Architecture | Test |
| FR-012-CON-2 | An agent SHALL NOT record human acceptance of the matrix. | Responsibility | Test |
| FR-012-CON-3 | No pin SHALL require a rebuild from source to roll back. | Compatibility | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-012-AC-1 | Every component pins a released version and names its release; no pin is a branch, `latest`, or `HEAD`. | Test (TC-079) |
| FR-012-AC-2 | Compatible, incompatible, and unknown are distinct, each carries its reason, and neither incompatible nor unknown satisfies the gate. | Test (TC-080) |
| FR-012-AC-3 | The gate requires every pinned component; one unobserved component withholds it. | Property (TC-081) |
| FR-012-AC-4 | Acceptance is pending, unattributed, and documented as a human act (CON-2). | Test (TC-082) |
| FR-012-AC-5 | Every artifact digest the matrix records matches this tree, over at least the ten schema assets. | Test (TC-083) |
| FR-012-AC-6 | Upgrade order and a rollback note exist per component, no rollback is irreversible, and publication changes no repository's CI posture. | Test (TC-084) |
| FR-012-AC-7 | An unknown matrix version and an unknown component name are refused. | Test (TC-085) |
| FR-012-AC-8 | The classifier reaches for no subprocess, socket, or write, and the observing program is a separate file (CON-1). | Inspection (TC-086) |

## Dependencies

- **Upstream**: [FR-011](./FR-011-accepted-compatibility-corpus.md); the
  released Quire CLI (`agent-ix/quire-cli#74`) and Quoin
  (`agent-ix/quoin#322`, `agent-ix/quoin#323`).
- **Downstream**: `agent-ix/engineering-assurance#10`, which may not begin an
  enforcing migration until this matrix is accepted.
