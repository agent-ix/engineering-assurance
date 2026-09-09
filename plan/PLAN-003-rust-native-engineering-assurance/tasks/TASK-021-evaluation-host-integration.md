---
id: TASK-021
title: "Integrate the accepted evaluation host boundary"
type: Task
status: blocked
track: B
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/TASK-020"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-109"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-124"
    type: verifies
---
# TASK-021: Integrate the accepted evaluation host boundary

## Scope

Qualify and consume a cli-agent-evals-owned structured suite interface and run the complete supported host/scenario matrix without inferring human decisions.

## Subtasks

- [ ] Record the accepted suite identity, version, revision, digest, and compatibility gate.
- [ ] Load Rust-owned scenarios/assertions and retain governing versions and transcript identity.
- [ ] Refuse incomplete cells and absent, stale, foreign, mismatched, or unaccepted host artifacts.

## Deliverables

- Qualified host integration and complete 28-cell evaluation evidence.

## Notes

- Blocked until TASK-020 completes and the cli-agent-evals owner accepts the exact interface artifact; no MJS replacement shim is authorized.
