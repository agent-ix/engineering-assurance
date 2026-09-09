---
id: TASK-014
title: "Port evidence availability and identity"
type: Task
status: in_progress
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/TASK-013"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: references
  - target: "ix://agent-ix/engineering-assurance/NFR-005"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-100"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-102"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-103"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-117"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-118"
    type: verifies
---
# TASK-014: Port evidence availability and identity

## Scope

Port evidence availability, operator observation, immutable governing-version validation, and legacy canonical JSON identity without rounding integers or admitting non-finite identity inputs.

## Subtasks

- [x] Implement the four evidence availability branches and stable validation order.
- [x] Preserve arbitrary-precision integers and refuse non-finite/overflowing numeric inputs before digest creation.
- [x] Exercise accepted producer fixtures plus deterministic generated JSON values against the retained reference.
- [ ] Resolve independent review findings and obtain re-review.

## Deliverables

- Rust evidence module and public typed inspection surface.
- Bare ix-trace-rs differential, mutation, accepted-corpus, and numeric-boundary tests.
- Passing Rust review, Quire reconciliation, and external re-review.

## Notes

- Active implementation: engineering-assurance PR #27.
- No legacy executable path is removed by this additive slice.
