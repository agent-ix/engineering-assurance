---
id: TASK-016
title: "Port semantic validation, projections, and fixtures"
type: Task
status: in_progress
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/TASK-014"
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

- [x] Implement semantic/reference validation and every non-success state.
- [x] Implement bounded report and PGM-01 projections without rewriting source bytes.
- [x] Generate inert cross-language fixtures from one Rust-owned semantic source.
- [ ] Resolve Rust review findings and obtain independent implementation review.

## Deliverables

- Pure Rust semantic, projection, and fixture-generation APIs.
- Per-capability accepted-corpus and adverse-case parity evidence.

## Notes

- Completion publishes the first reusable shared Rust library boundary needed by contract/TL consumer planning; it does not authorize local copies or claim a machine-facing CLI boundary before TASK-015 covers the corresponding command.
- Active additive implementation: `agent-c/issue-59-semantic-rust`, stacked on engineering-assurance PR #29 while that prerequisite awaits final re-review.
- Retained Python remains a differential oracle until the same-revision removal gate; no legacy executable path is removed by this slice.
