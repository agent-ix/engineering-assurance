---
id: FR-018
title: "Migrate consumers before retiring legacy executable paths"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-016"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "requires"
---

# FR-018: Migrate consumers before retiring legacy executable paths

## Description

Engineering Assurance SHALL migrate each recorded consumer through an additive
Rust parity phase before removing the corresponding Python, JavaScript, or MJS
executable path.

## Inputs

- The executable-path and consumer matrix in ADR-002.
- The reviewed Rust library, CLI, and external-host interfaces.
- Same-revision differential, package, rights, and integration evidence.
- The assurance-artifact schema consumer inventory and its versioned migration
  dispositions.
- The committed consumer registry and candidate-revision compatibility snapshot
  required by FR-015.

## Outputs

- A disposition and migration state for every matrix row and consumer.
- Per-capability rollback instructions.
- A final inventory of retained configuration and inert foreign-language data.

## Behavior

- Migration SHALL follow the order recorded in ADR-002.
- Each consumer SHALL identify its old path, new path, interface version,
  candidate revision, parity evidence, and rollback action.
- Engineering Assurance SHALL resolve a recorded assurance-contract consumer
  from the committed registry and bind it to an exact repository commit,
  artifact path, and blob digest; an ad hoc workstation scan is not a migration
  record.
- A legacy path SHALL remain available until its replacement passes at the same
  candidate revision.
- Engineering Assurance SHALL remove a legacy path only after every recorded
  consumer has moved.
- Historical corpus and evidence bytes SHALL remain unchanged.
- A changed interface or compatibility promise SHALL return to specification
  and review before implementation continues.
- An assurance-artifact schema replacement SHALL wait until every recorded
  active consumer validates or completes its owner-approved migration.

## Error Conditions

Missing parity evidence, a mismatched or dirty candidate revision, an unrecorded
consumer, an unresolved registry entry, an unavailable replacement host, a
changed historical byte, and an unresolved owner disposition each block removal.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-018-CON-1 | Deletion SHALL be the final step for each migrated capability. | Lifecycle | Test |
| FR-018-CON-2 | The migration SHALL NOT rewrite historical evidence or corpus bytes. | Data Integrity | Test |
| FR-018-CON-3 | Coexistence during parity SHALL NOT be reported as completed remediation. | Reporting | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-018-AC-1 | Every executable-path matrix row and consumer records exactly one current migration state and one owner-approved final disposition. | Test (TC-097) |
| FR-018-AC-2 | A removal attempt is refused unless old and new paths passed at the same candidate revision and all recorded consumers use the new versioned interface. | Property (TC-113) |
| FR-018-AC-3 | Each failure mode has a rollback that restores the old invocation without rewriting historical evidence or corpus bytes. | Test (TC-114) |
| FR-018-AC-4 | Final inventory and static scans find no unapproved first-party non-Rust semantic or assertion logic and do not count inert fixture samples as executable remediation debt. | Test (TC-115) |
| FR-018-AC-5 | Removal is refused when the consumer registry digest differs from the snapshot binding or when any accepted external host interface identity differs from the one qualified at the candidate revision. | Property (TC-125) |

## Dependencies

- **Upstream**: accepted ADR-002 and completed FR-014 through FR-017 plus
  FR-019.
- **Downstream**: consumer repositories may migrate only against a released or
  revision-pinned interface that satisfies this gate.
