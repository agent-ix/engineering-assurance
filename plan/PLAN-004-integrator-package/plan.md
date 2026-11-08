---
id: PLAN-004
title: "Integrator package contract implementation"
type: Plan
status: complete
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-030"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-031"
    type: references
  - target: "ix://agent-ix/engineering-assurance/IT-007"
    type: references
---

# PLAN-004: Integrator package contract implementation

## Outcome

A Rust-consumable, versioned integrator-package writer, strict reader and
candidate-bound store that preserve source facts and open or negative states and
leave adopter validation open (Linear EA-80; consumer TL-19, Task-015 and IT-002
in tl-syntax).

## Scope Boundaries

- The contract is generic: it names no consuming repository and carries no
  release decision, acceptance, qualification or certification.
- No digest, pin or compatibility corpus is introduced; integrity is the
  canonical re-encoding check.
- Publication reuses the FR-027 no-replace publisher (EA#98); it adds no second
  atomic-write primitive.
- Releasing or tagging is not part of this plan.

## Dependency Graph

```text
TASK-014 model, writer, strict reader
  └── TASK-015 candidate-bound atomic idempotent store
```

## Test Strategy

- Mutation tests delete, alter or improve each required premise and assert a
  typed refusal.
- Real filesystem tests cover retry, attempt reuse with changed input and an
  eight-writer race for identical and non-identical content.
- An offline downstream crate compiles with only the `integrator-package`
  feature and round-trips a package.

## Task File Mapping

| Task | Scope | Owns |
| --- | --- | --- |
| TASK-014 | Model, writer, strict reader | FR-030 |
| TASK-015 | Store | FR-031, IT-007 |

## Gate

`make integration-gate` passes locally, then review. No release.
