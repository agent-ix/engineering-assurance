---
id: TASK-024
title: "Implement Quoin chain orchestration"
type: Task
status: not_started
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/TASK-017"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-130"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-131"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-132"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-133"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-134"
    type: verifies
---
# TASK-024: Implement Quoin chain orchestration

## Scope

Implement the reviewed Rust library types and native CLI capability that
replace the eight repository-local `assurance_chain.py` drivers while treating
Quoin as the only evidence-schema, persistence, audit, receipt, and outcome
authority.

## Subtasks

- [ ] Define closed request/result types and a fixed Quoin operation state
  machine over `seal-record`, `seal-attestation`, `intake`, `receipt`, and
  `verify-receipt`; accept no arbitrary executable, argument, operation graph,
  or audit-report producer.
- [ ] Preflight repository/store roots, candidate revision, Quoin identity,
  declaration bytes, pre-produced results, and every digest before side effects.
- [ ] Implement bounded child lifecycle, structured response validation,
  operation-order accounting, and stop-after-failure behavior.
- [ ] Differentially exercise the eight pinned consumer fixtures and all
  success/non-success states without changing historical bytes.
- [ ] Run the ownership audit, exact Rust 1.98.1 gates, bare ix-trace-rs
  reconciliation, Rust review, and independent review.

## Deliverables

- `engineering-assurance quoin-chain` with versioned JSON request/result
  contracts and no reusable library I/O.
- Accepted/adverse fixtures plus same-revision differential evidence for all
  eight consumers.
- A static ownership report proving no producer runner, arbitrary-output
  verdict parser, or copied Quoin schema/store/canonicalization exists.

## Notes

- A previously completed Quoin write is reported, not rolled back or hidden,
  when a later operation fails.
- Consumer replacement/deletion remains owned by quire-research #60 and waits
  until this task and TASK-016/TASK-017 are reviewed and usable.
