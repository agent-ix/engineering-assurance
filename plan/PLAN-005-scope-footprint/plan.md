---
id: PLAN-005
title: "Declared scope footprint verification implementation"
type: Plan
status: complete
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-032"
    type: references
---

# PLAN-005: Declared scope footprint verification implementation

## Outcome

A shared check that compares a declared change scope with the candidate's
actual changed-path set and reports complete, false scope or incomplete (Linear
EA-78; consumer TL-63).

## Scope Boundaries

- The check decides coverage only; it does not judge a change acceptable.
- It reuses the crate's normalized-relative-path rule and the binary's bounded
  process host; it adds no dependency.
- Releasing or tagging is not part of this plan.

## Dependency Graph

```text
TASK-016 pure verification
  └── TASK-017 git footprint adapter and command
```

## Test Strategy

- Pure cases each carry a negative: a mutation of the containment rule, the
  truncation rule or the validator makes a named test fail.
- Command cases run the real binary against a temporary git repository.

## Task File Mapping

| Task | Scope | Owns |
| --- | --- | --- |
| TASK-016 | `scope_footprint` module and feature | FR-032-AC-1..5 |
| TASK-017 | `scope_footprint_host`, command | FR-032-AC-6, FR-032-AC-7 |

## Gate

fmt, clippy, doc, deny, `make test` and `tests/scope_footprint.rs` pass locally,
then review. No release.
