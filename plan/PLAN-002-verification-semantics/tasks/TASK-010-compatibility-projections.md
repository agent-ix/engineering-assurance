---
id: TASK-010
title: "Implement read-only PGM-01 compatibility projections"
type: Task
status: pending
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-002"
    type: part_of
  - target: "ix://agent-ix/engineering-assurance/FR-010"
    type: references
---

# TASK-010: Implement read-only PGM-01 compatibility projections

## Objective

Map PGM-01 v1/v2 fields to authoritative semantic references and render bounded
JSON/Markdown views without modifying or replacing the legacy record.

## Deliverables

- Pure mapping functions with explicit compatible/lossy/incompatible/unreadable
  outcomes and limitations.
- JSON and Markdown report projections.
- Byte-identity and ambiguity/tamper negative tests.

## Acceptance

TC-055 and TC-064 through TC-067 pass.
