---
id: TASK-011
title: "Seal compatibility fixtures and the issue #5 gate"
type: Task
status: done
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-002"
    type: part_of
  - target: "ix://agent-ix/engineering-assurance/IT-005"
    type: references
---

# TASK-011: Seal compatibility fixtures and the issue #5 gate

## Objective

Provide canonical, negative, legacy, and generated-language fixtures and retain
the final implementation review evidence.

## Deliverables

- Canonical JSON fixture and deterministic Rust, TypeScript, and Python
  projections.
- Non-success, missing-provenance, unknown-version, malformed, stale, suspect,
  vacuous, tampered, ambiguous, and unreadable fixtures.
- Passing full local gates, code review, gap analysis, and package audit.

## Acceptance

TC-052 through TC-068 pass with no unresolved required review fix.
