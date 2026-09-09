---
id: TASK-022
title: "Migrate consumers and retire legacy paths"
type: Task
status: blocked
track: C
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/TASK-019"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/TASK-021"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/FR-018"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-097"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-113"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-114"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-115"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-125"
    type: verifies
---
# TASK-022: Migrate consumers and retire legacy paths

## Scope

Move every recorded consumer through an additive same-revision parity state and remove each legacy executable path only after all consumers use the qualified Rust interface.

## Subtasks

- [ ] Record old/new path, interface, revision, evidence, state, disposition, and rollback per consumer.
- [ ] Refuse migration or deletion on changed registry/host identity, dirty or mismatched revision, missing evidence, or unmigrated consumer.
- [ ] Remove legacy semantics last while preserving historical corpus and evidence bytes.

## Deliverables

- Complete migration ledger, rollback evidence, removals, and final executable-path inventory.

## Notes

- Blocked on both accepted host integrations. Contract/TL coordination is owned by quire-research #60 and consumes this boundary rather than duplicating it.
