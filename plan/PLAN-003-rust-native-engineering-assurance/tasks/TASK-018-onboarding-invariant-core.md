---
id: TASK-018
title: "Port onboarding and invariant evaluation"
type: Task
status: not_started
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/TASK-017"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/FR-016"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-105"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-106"
    type: verifies
---
# TASK-018: Port onboarding and invariant evaluation

## Scope

Port host-independent repository discovery, bounded onboarding, and deterministic workflow-invariant evaluation into the Rust library and CLI.

## Subtasks

- [ ] Preserve inventory-before-proposal and boundary refusal behavior.
- [ ] Return every applicable invariant failure in stable order.
- [ ] Differentially test canonical and adverse onboarding/invariant fixtures.

## Deliverables

- Rust onboarding and invariant-provider core.
- Old/new corpus evidence for boundaries, ordering, and failures.

## Notes

- ix-flow lifecycle and loading remain in TASK-019 under the external host gate.
