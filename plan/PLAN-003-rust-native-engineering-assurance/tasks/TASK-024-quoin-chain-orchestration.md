---
id: TASK-024
title: "Implement Quoin chain orchestration"
type: Task
status: blocked
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
  - target: "ix://agent-ix/engineering-assurance/TC-135"
    type: verifies
  - target: "ix://agent-ix/qa-corpus/FR-201"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-064"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-068"
    type: depends_on
---
# TASK-024: Implement Quoin chain orchestration

## Scope

Implement the reviewed Rust library types and native CLI capability that
replace the eight repository-local `assurance_chain.py` drivers while treating
Quoin as the only evidence-schema, persistence, audit, receipt, and outcome
authority.

## Subtasks

- [ ] Before implementation, accept the reviewed qa-corpus FR-201 baseline and
  reproduce all eight pinned revisions, driver/review-source digests,
  observations, witnesses, and the retained `partial` versus `incomplete`
  semantic-gap record. Baseline capture is not part of this task.
- [ ] Before implementation, accept a Quoin revision whose FR-064/FR-068
  qualification covers same-store concurrent publication, recovery exclusion,
  exact-input retry, collision preservation, and busy/stale-lock refusal.
- [ ] Define closed request/result types and a fixed Quoin operation state
  machine over `seal-record`, `seal-attestation`, `intake`, `receipt`, and
  `verify-receipt`; accept no arbitrary executable, argument, operation graph,
  or audit-report producer.
- [ ] Preflight repository/store roots, candidate revision, Quoin identity,
  declaration bytes, pre-produced results, and every digest before side effects.
- [ ] Implement bounded child lifecycle, structured response validation,
  operation-order accounting, byte-identical bounded retry, busy/exhaustion
  reporting, ambiguous-completion refusal, and stop-after-failure behavior.
- [ ] Differentially exercise the eight pinned consumer fixtures and all
  success/non-success states without changing historical bytes.
- [ ] Run the ownership audit, exact Rust 1.98.1 gates, bare ix-trace-rs
  reconciliation, Rust review, and independent review.

## Deliverables

- `engineering-assurance quoin-chain` with versioned JSON request/result
  contracts and no reusable library I/O.
- Accepted/adverse fixtures plus same-revision differential evidence for all
  eight consumers, consumed from the reviewed qa-corpus FR-201 baseline rather
  than recaptured from moving repository heads.
- A static ownership report proving no producer runner, arbitrary-output
  verdict parser, or copied Quoin schema/store/canonicalization exists.

## Notes

- A previously completed Quoin write is reported, not rolled back or hidden,
  when a later operation fails.
- The task stays blocked until both external prerequisites above are accepted;
  Engineering Assurance must not compensate by implementing a store lock,
  staging cleanup, collision policy, or canonical form.
- Consumer replacement/deletion remains owned by quire-research #60 and waits
  until this task and TASK-016/TASK-017 are reviewed and usable.
