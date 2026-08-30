---
id: TASK-005
title: "Integrate resumable human decisions"
type: Task
status: done
track: E
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-001"
    type: part_of
  - target: "ix://agent-ix/engineering-assurance/TASK-001"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/TASK-002"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/FR-005"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-026"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-048"
    type: verifies
---

# TASK-005: Integrate resumable human decisions

## Scope

Bind canonical workflows to ix-flow run identity, persisted phases, resume, and
explicit human acceptance/rejection without duplicating ix-flow state mechanics.

## TDD Work

- Write TC-026..TC-030 and TC-047..TC-048 before modifying workflow definitions.
- Bind run id to repository, workflow, and definition version; refuse mismatches
  without mutating either run.
- Preserve the last completed phase across interruption and avoid repeated work.
- Keep every terminal transition human-gated and record exactly one attributed
  acceptance or rejection event.

## Exit Criteria

- Missing choice remains non-terminal and automatic gate override fails closed.
- Acceptance and rejection are distinct, attributed, idempotent terminal paths.
- Resume continues from persisted state under the original binding.
