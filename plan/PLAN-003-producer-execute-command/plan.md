---
id: PLAN-003
title: "Producer-execute CLI command implementation"
type: Plan
status: done
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-029"
    type: references
---

# PLAN-003: Producer-execute CLI command implementation

## Outcome

Expose the FR-019 bounded executor as `engineering-assurance producer-execute`
with typed refusals and published, versioned request and result schemas, so a
consumer without a Cargo dependency reaches the executor over a process
boundary (EA-79).

## Scope Boundaries

- Add no dependency, Cargo feature or library module; the command lives in the
  binary crate and uses the existing `producer_execution` API.
- Keep the executor, request and result of FR-019 unchanged.

## Dependency Graph

```text
TASK-012 schemas, adapter and command
  └── TASK-013 CLI acceptance tests and package membership
```

## Test Strategy

Real-binary tests (TC-214..TC-218) run the Cargo-built CLI against the existing
Rust fixture producer; schema tests validate real documents and mutants with the
`jsonschema` crate already enabled by `full`.

## Task File Mapping

| Task | Scope | Owns |
| --- | --- | --- |
| TASK-012 | Schemas, built-in adapter, command wiring | FR-029-AC-1..AC-4 |
| TASK-013 | Acceptance tests, README, package audit | FR-029-AC-1..AC-5; TC-214..TC-218 |

## Completion Gates

- `make lint test package-audit validate-docs` and the Rust gates
  (`rust-format`, `rust-clippy`, `rust-tests`, `rust-docs`) pass locally.
