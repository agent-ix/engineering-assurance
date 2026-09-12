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
- The Rust-owned CLI host adapter SHALL invoke only the matrix-declared `quire`,
  `quoin`, `ix-flow`, and local Git tag observations with bounded capture.
- The Rust-owned CLI host adapter SHALL pass typed observations to the classifier
  without reimplementing matrix policy.
- The observing program SHALL hash every artifact the matrix records against the
  selected tree, and SHALL report the count it hashed alongside its verdict.
- The observing program SHALL report a recorded artifact absent from the
  selected tree as an absence, separately from digest drift, and SHALL withhold
  the gate. An unverified artifact population is unknown for the same reason an
  unobserved component is, and unknown SHALL NOT satisfy the gate.
- Publication of these versions SHALL leave every campaign repository's
  workflows on manual dispatch only.

## Error Conditions

An unknown matrix version, a matrix missing its acceptance, gate, component or
rollback sections, a matrix pinning no component, and an unknown component name
are each refused with a `MatrixError`. An artifact whose bytes no longer match
its recorded digest is reported by path as drift. An artifact the matrix names
and the selected tree does not contain is reported by path as an absence, a
distinct condition from drift, because a tree that is missing the recorded
artifacts is a tree this matrix did not describe and cannot vouch for. Both
conditions withhold the gate; neither is skipped.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-012-CON-1 | The classifier SHALL execute no subprocess and write no file. | Architecture | Test |
| FR-012-CON-2 | An agent SHALL NOT decide acceptance of the matrix. | Responsibility | Test |
| FR-012-CON-3 | No pin SHALL require a rebuild from source to roll back. | Compatibility | Inspection |
| FR-012-CON-4 | Engineering Assurance MAY transcribe an acceptance only when a named human explicitly directs it to do so. | Responsibility | Test |
| FR-012-CON-5 | The recorded acceptance attribution SHALL name that human rather than the agent. | Responsibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-012-AC-1 | Every component pins a released version and names its release; no pin is a branch, `latest`, or `HEAD`. | Test (TC-079) |
| FR-012-AC-2 | Compatible, incompatible, and unknown are distinct, each carries its reason, and neither incompatible nor unknown satisfies the gate. | Test (TC-080) |
| FR-012-AC-3 | The gate requires every pinned component; one unobserved component withholds it. | Test (TC-081) |
| FR-012-AC-4 | Acceptance is either pending and wholly unattributed, or accepted with both a named human and a date; it is never half-recorded, and it is documented as a human act (CON-2, CON-4, CON-5). | Test (TC-082) |
| FR-012-AC-5 | Every artifact digest the matrix records matches this tree, over at least the ten schema assets, and the observing program refuses a tree in which any recorded artifact is absent rather than reporting a satisfied gate over an unverified population. | Test (TC-083) |
| FR-012-AC-6 | Upgrade order and a rollback note exist per component, no rollback is irreversible, and publication changes no repository's CI posture. | Test (TC-084) |
| FR-012-AC-7 | An unknown matrix version and an unknown component name are refused. | Test (TC-085) |
| FR-012-AC-8 | The classifier reaches for no subprocess, socket, or write, and the observing program is a separate file (CON-1). | Inspection (TC-086) |
| FR-012-AC-9 | Compatible versions and recorded human acceptance are independent gate conditions; a fully pinned toolchain does not satisfy the gate while acceptance is unrecorded, any state but `accepted` withholds, and an `accepted` state lacking a name or a date withholds as a half-record. | Test (TC-095) |
| FR-012-AC-10 | The Rust observer keeps an unavailable, failed, timed-out, oversized, or unparseable declared tool observation as unknown; it invokes no undeclared tool, preserves artifact-digest drift and recorded-artifact absence as separate gate conditions that each withhold the gate, publishes the number of artifacts it hashed, and delegates all verdicts to the pure classifier. | Test (TC-130) |

## Dependencies

- **Upstream**: [FR-011](./FR-011-accepted-compatibility-corpus.md); the
  released Quire CLI (`agent-ix/quire-cli#74`) and Quoin
  (`agent-ix/quoin#322`, `agent-ix/quoin#323`).
- **Downstream**: `agent-ix/engineering-assurance#10`, which may not begin an
  enforcing migration until this matrix is accepted.
