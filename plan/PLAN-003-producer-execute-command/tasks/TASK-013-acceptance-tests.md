---
id: TASK-013
title: "Accept the producer-execute command"
type: Task
status: done
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-003"
    type: part_of
  - target: "ix://agent-ix/engineering-assurance/TASK-012"
    type: depends_on
---

# TASK-013: Accept the producer-execute command

## Objective

Prove the command through the real binary and keep package membership and the
README accurate.

## Deliverables

- `tests/producer_execute_cli.rs` carrying TC-214 through TC-218.
- README command table row and passing local gates.

## Acceptance

TC-214 through TC-218 pass with no unresolved review finding.
