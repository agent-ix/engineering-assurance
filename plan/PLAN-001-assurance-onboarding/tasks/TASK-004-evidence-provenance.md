---
id: TASK-004
title: "Preserve evidence availability and provenance"
type: Task
status: todo
track: D
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-001"
    type: part_of
  - target: "ix://agent-ix/engineering-assurance/TASK-001"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/FR-004"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-020"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-046"
    type: verifies
---

# TASK-004: Preserve evidence availability and provenance

## Scope

Implement the onboarding evidence-state envelope, immutable governing-version
tuple, producer validation, operator observations, and explicit Quoin handoff.

## TDD Work

- Write TC-020..TC-025 and TC-046 first, including generated state combinations.
- Require exactly one of observed, unavailable, not-computed, and not-applicable.
- Refuse malformed output and missing/mutable module, plugin, skill, workflow,
  executable, schema, or producer identity.
- Preserve invocation failures and observations without converting them to evidence;
  delegate any persisted evidence record to Quoin.

## Exit Criteria

- Every considered producer has exactly one explicit availability state.
- Observed evidence carries the complete immutable tuple and raw observation detail.
- No unavailable/deferred/excluded producer is represented as successful evidence.

