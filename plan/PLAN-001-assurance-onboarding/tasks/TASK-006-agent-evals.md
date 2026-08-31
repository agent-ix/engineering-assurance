---
id: TASK-006
title: "Build the four-host onboarding evaluation suite"
type: Task
status: done
track: F
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-001"
    type: part_of
  - target: "ix://agent-ix/engineering-assurance/TASK-002"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/TASK-004"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/TASK-005"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/FR-006"
    type: references
  - target: "ix://agent-ix/engineering-assurance/NFR-002"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-031"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-049"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-050"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-051"
    type: verifies
---

# TASK-006: Build the four-host onboarding evaluation suite

## Scope

Implement the 28-cell evaluation matrix over seven scenario variants and four real
agent hosts, plus version-bound envelopes and a complete-only aggregate gate.

## TDD Work

- Write TC-031..TC-034, TC-039, and TC-049..TC-051 around harness selection,
  envelope validation, runtime identity, revision binding, and aggregation before
  live execution.
- Record immutable module/plugin/skill/workflow/executable/schema/producer versions,
  source revision, configuration, transcript digest, commands, elapsed time, human
  interaction counts, outcome, and explicit terminal choice.
- Fail the aggregate on a missing executable, missing cell, malformed envelope,
  unsupported outcome, or collapsed acceptance/rejection state.

## Exit Criteria

- All 28 cells execute through their real installed host binary.
- Equivalent runs retain explicit acceptance and rejection on every host.
- Snapshot and post-run verification bind the complete ix-flow runtime package.
- Release evidence is rejected when its revision differs from repository `HEAD`.
- The aggregate passes only with 28 complete passing, non-inventing envelopes.
