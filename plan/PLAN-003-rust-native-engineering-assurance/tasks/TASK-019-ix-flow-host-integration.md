---
id: TASK-019
title: "Integrate the accepted ix-flow host boundary"
type: Task
status: blocked
track: B
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/TASK-018"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/FR-016"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-107"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-108"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-123"
    type: verifies
---
# TASK-019: Integrate the accepted ix-flow host boundary

## Scope

Qualify and consume an ix-flow-owned structured provider interface, then migrate canonical and pilot invariant loading without reimplementing run state or human gates.

## Subtasks

- [ ] Record the accepted interface identity, version, revision, digest, and compatibility gate.
- [ ] Integrate the Rust provider and exercise interruption, resume, decision, transition, and binding cases.
- [ ] Refuse absent, stale, foreign, mismatched, or unaccepted host artifacts.

## Deliverables

- Qualified ix-flow integration and same-revision canonical/pilot parity evidence.

## Notes

- Blocked until TASK-018 completes and the ix-flow owner accepts the exact interface artifact; no foreign-language shim is authorized.
