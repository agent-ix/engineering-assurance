---
id: TASK-016
title: "Port semantic validation, projections, and fixtures"
type: Task
status: not_started
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/TASK-015"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-100"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-102"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-103"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-104"
    type: verifies
---
# TASK-016: Port semantic validation, projections, and fixtures

## Scope

Port the remaining shared verification vocabulary validation, historical mappings, bounded projections, and deterministic fixture generation while consuming externally owned portable contracts by version.

## Subtasks

- [ ] Implement semantic/reference validation and every non-success state.
- [ ] Implement bounded report and PGM-01 projections without rewriting source bytes.
- [ ] Generate inert cross-language fixtures from one Rust-owned semantic source.

## Deliverables

- Pure Rust semantic, projection, and fixture-generation APIs.
- Per-capability accepted-corpus and adverse-case parity evidence.

## Notes

- Completion publishes the first reusable shared boundary needed by contract/TL consumer planning; it does not authorize local copies.
