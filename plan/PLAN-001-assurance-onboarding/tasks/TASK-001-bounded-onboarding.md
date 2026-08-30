---
id: TASK-001
title: "Implement bounded repository onboarding"
type: Task
status: todo
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-001"
    type: part_of
  - target: "ix://agent-ix/engineering-assurance/FR-001"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-004"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-045"
    type: verifies
---

# TASK-001: Implement bounded repository onboarding

## Scope

Create the canonical onboarding procedure and support code that inventories the
selected repository boundary before proposing the smallest justified assurance
work or explicitly proposing none.

## TDD Work

- Write TC-004..TC-008 and TC-044..TC-045 before implementing inventory/proposal
  behavior.
- Keep declared decisions, existing assurance artifacts, measurement definitions,
  retained evidence, and producer availability as separate collections.
- Refuse malformed/conflicting artifacts without modification or selection.
- Stage any justified artifact under the selected root, render from the installed
  skeleton, validate with Quire, and publish atomically only after success.

## Exit Criteria

- Inventory always precedes proposal and an absent justification writes nothing.
- Missing boundary input pauses without a write.
- No target or manifest reference can escape the selected repository root.

