---
id: TASK-015
title: "Implement the versioned CLI boundary"
type: Task
status: in_progress
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/TASK-014"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-098"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-099"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-101"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-121"
    type: verifies
---
# TASK-015: Implement the versioned CLI boundary

## Scope

Implement the machine-facing CLI protocol, filesystem/process boundary, size/deadline ceilings, buffered stdout, termination behavior, and stable refusal results.

## Subtasks

- [x] Define versioned request/result types and bounded input decoding for the compatibility command.
- [ ] Apply the reviewed protocol to each remaining machine-facing command.
- [ ] Implement root confinement, child lifecycle, deadline, cancellation, and signal behavior.
- [ ] Audit the library for reusable I/O or arbitrary stdout verdict inference.

## Deliverables

- Versioned CLI commands with one-result stdout and diagnostic-only stderr.
- Boundary and mutation tests for every no-side-effect refusal.

## Notes

- This task introduces no evidence persistence and no universal producer runner.
- This is a cross-cutting exposure gate, not a hard predecessor of pure library ports. Each later task may add its I/O-free library behavior after its own predecessor, but no machine-facing command is usable until this task applies and verifies the reviewed protocol for that command.
