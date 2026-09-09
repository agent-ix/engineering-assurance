---
id: TASK-013
title: "Port compatibility classification"
type: Task
status: done
track: Done
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/TASK-012"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-100"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-103"
    type: verifies
---
# TASK-013: Port compatibility classification

## Scope

Port the read-only compatibility classifier and its accepted corpus boundary into the Rust library without changing compatibility outcomes or source bytes.

## Subtasks

- [x] Implement typed compatibility outcomes and stable reasons.
- [x] Differentially verify accepted, incompatible, and unknown cases.
- [x] Prove corpus access remains read-only.

## Deliverables

- Reviewed Rust compatibility module and differential corpus tests.

## Notes

- This completes only the compatibility-classifier subcase of aggregate TC-100.
