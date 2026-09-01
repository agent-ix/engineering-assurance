---
id: FR-013
title: "Publish the reviewed campaign migration contract"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-005"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-012"
    type: "requires"
---

# FR-013: Publish the reviewed campaign migration contract

## Description

Engineering Assurance SHALL publish one migration contract that decides, for
every recurring script family in the eight campaign repositories, whether it is
kept, deleted, or replaced, and that defines the review checklist a migration
pull request is judged against.

A migration SHALL NOT begin until the compatibility matrix records human
acceptance.

## Inputs

- The `scripts/` and `schemas/` trees of the eight campaign repositories, read
  at `origin/main`.
- The accepted compatibility matrix (FR-012) and its acceptance state.
- The read-only compatibility view (FR-010) for legacy history.
- Quoin's adapter inventory for the formats those repositories emit.

## Outputs

- `docs/migration-contract.md`, containing the decision table, the two
  prohibitions, the domain/intake boundary, the procedure, rollback handling,
  and the pull-request review checklist.
- The preserved Agent A/B/C repository allocation.

## Behavior

- Every recurring script family SHALL carry exactly one decision: keep, delete,
  or replace, with a reason.
- The contract SHALL forbid a repository-local generic evidence schema, while
  permitting a schema that describes a repository's own domain output.
- The contract SHALL forbid recovering a verdict from stdout or stderr.
- The contract SHALL separate domain output validation from evidence intake,
  audit, and human decision, naming the owner of each.
- The contract SHALL preserve legacy evidence directories byte for byte and
  read them only through the compatibility view.
- The contract SHALL delete old generic machinery only after the shared path
  passes at the same candidate revision.
- The contract SHALL require every non-success state to be demonstrated.
- The contract SHALL leave every campaign workflow on manual dispatch.
- The contract SHALL make no certification, accreditation, authorization,
  identity, or non-repudiation claim.

## Error Conditions

A script family with no decision, a decision outside keep/delete/replace, a
repository missing from the allocation, and a checklist item with no
corresponding prohibition in the contract are each defects in the contract and
fail its gate.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-013-CON-1 | The contract SHALL NOT authorize a migration while matrix acceptance is unrecorded. | Responsibility | Test |
| FR-013-CON-2 | The contract SHALL change no repository's workflow trigger. | Architecture | Test |
| FR-013-CON-3 | The contract SHALL claim no certification, accreditation, authorization, identity, or non-repudiation. | Responsibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-013-AC-1 | Every family in the decision table carries exactly one of keep, delete, or replace, with a reason. | Test (TC-087) |
| FR-013-AC-2 | The table accounts for every recurring script family present in the eight repositories, and says so when the sources cannot be read. | Integration (TC-088) |
| FR-013-AC-3 | Repository-local generic evidence schemas and stdout-derived verdicts are both forbidden by name, and a domain-output schema is explicitly permitted. | Test (TC-089) |
| FR-013-AC-4 | Domain output validation, evidence intake, audit, and human decision each name a distinct owner. | Test (TC-090) |
| FR-013-AC-5 | Rollback is defined per failure mode, legacy history is never rewritten in any of them, and deletion is last. | Test (TC-091) |
| FR-013-AC-6 | The review checklist covers the inventory, both prohibitions, byte-identical legacy evidence, every non-success state, and the manual-dispatch posture. | Test (TC-092) |
| FR-013-AC-7 | All eight repositories appear exactly once in the Agent A/B/C allocation. | Test (TC-093) |
| FR-013-AC-8 | The contract states that migration waits on matrix acceptance and makes no qualification claim (CON-1, CON-3). | Test (TC-094) |

## Dependencies

- **Upstream**: [FR-010](./FR-010-read-only-compatibility-and-reporting.md),
  [FR-011](./FR-011-accepted-compatibility-corpus.md), and
  [FR-012](./FR-012-pinned-compatibility-matrix.md).
- **Downstream**: the eight repository migrations, each of which is reviewed
  against this contract's checklist.
