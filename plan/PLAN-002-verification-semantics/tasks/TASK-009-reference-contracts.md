---
id: TASK-009
title: "Add reference contracts"
type: Task
status: done
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-002"
    type: part_of
  - target: "ix://agent-ix/engineering-assurance/FR-009"
    type: references
---

# TASK-009: Add reference contracts

## Objective

Add JSON schemas for source-attributed semantic references without adding a persisted evidence record family.

## Deliverables

- Draft-07 schemas with explicit purpose and non-authoritative projection
  discriminators.
- Complete producer tuple, definition version, state, record identity, and
  source-field path validation.

## Acceptance

TC-057 through TC-063 and TC-068 pass.
