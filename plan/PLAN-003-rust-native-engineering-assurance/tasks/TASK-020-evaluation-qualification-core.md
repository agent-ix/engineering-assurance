---
id: TASK-020
title: "Port evaluation and repository qualification"
type: Task
status: not_started
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/TASK-018"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-110"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-111"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-112"
    type: verifies
---
# TASK-020: Port evaluation and repository qualification

## Scope

Port host-independent evaluation result validation/aggregation and repository package, rights, manifest, integration, publication-refusal, and hosted-dispatch assertions into Rust.

## Subtasks

- [ ] Implement typed result validation and fail-closed aggregation.
- [ ] Port package-member, rights, manifest, integration, and publication-refusal gates.
- [ ] Reduce package, Make, and CI files to declarative dispatch.

## Deliverables

- Rust evaluation and repository-qualification commands.
- Independent positive and negative parity evidence for every gate.

## Notes

- Real-agent evaluation remains manual and external host loading remains in TASK-021.
