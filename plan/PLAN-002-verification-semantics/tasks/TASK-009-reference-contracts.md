---
id: TASK-009
title: "Add reference and bounded-report contracts"
type: Task
status: done
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-002"
    type: part_of
  - target: "ix://agent-ix/engineering-assurance/FR-009"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-010"
    type: references
---

# TASK-009: Add reference and bounded-report contracts

## Objective

Add JSON schemas for source-attributed semantic references and bounded report
views without adding a persisted evidence record family.

## Deliverables

- Draft-07 schemas with explicit purpose and non-authoritative projection
  discriminators.
- Complete producer tuple, definition version, state, record identity, and
  source-field path validation.
- Claims/evidence/counterevidence/gaps/owner/action report contract with no trust
  score or embedded human verdict.

## Acceptance

TC-054, TC-057 through TC-063, TC-067, and TC-068 pass.
