---
id: TASK-012
title: "Add the producer-execute command, adapter and schemas"
type: Task
status: done
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-003"
    type: part_of
  - target: "ix://agent-ix/engineering-assurance/FR-029"
    type: references
---

# TASK-012: Add the producer-execute command, adapter and schemas

## Objective

Wire the command, the built-in process-evidence adapter and the embedded schemas.

## Deliverables

- `src/producer_execute_host.rs` with the adapter, request parsing and typed errors.
- `schemas/producer-execution-request-v1.schema.json` and the result schema.
- The `producer-execute` subcommand in `src/main.rs`.

## Acceptance

FR-029-AC-1 through AC-4 hold under the TASK-013 tests.
