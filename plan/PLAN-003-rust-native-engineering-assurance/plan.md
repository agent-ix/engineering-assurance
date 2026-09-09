---
id: PLAN-003
title: "Rust-native Engineering Assurance"
type: Plan
status: active
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-016"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-018"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: references
  - target: "ix://agent-ix/engineering-assurance/NFR-005"
    type: references
---
# Implementation Plan: Rust-native Engineering Assurance

## Requirements Summary

### Stakeholder Requirements

- [ ] **StR-003**: Centralize first-party Engineering Assurance production and qualification behavior in the repository-owned Rust package without absorbing external owners.

### Functional Requirements

- [ ] **FR-014**: Expose a bounded, versioned Rust library and CLI boundary.
- [ ] **FR-015**: Preserve semantic, identity, compatibility, and refusal behavior.
- [ ] **FR-016**: Run onboarding and workflow invariants through Rust.
- [ ] **FR-017**: Run evaluations and repository qualification through Rust.
- [ ] **FR-018**: Migrate every recorded consumer before retiring a legacy executable path.
- [ ] **FR-019**: Replace duplicated assurance-chain orchestration without
  absorbing Quoin schemas, persistence, outcomes, or producer execution.

### Non-Functional Requirements

- [ ] **NFR-005**: Enforce Rust containment, exact Rust 1.98.1 qualification, canonical ix-trace-rs markers, performance parity, and current-head hosted gates.

## Dependency Graph

### Core dependency edges

- `ADR-002 -> FR-014`
  Reason: the accepted decision fixes the repository, package, library, CLI, ownership, and host boundaries.
- `FR-014 -> FR-015`
  Reason: pure shared types, stable errors, and versioned result contracts precede semantic ports.
- `FR-015 -> FR-016, FR-017`
  Reason: onboarding, invariants, evaluation, and qualification reuse the shared classifiers, identity behavior, projections, and artifact contracts.
- `FR-014 + FR-015 -> FR-019`
  Reason: Quoin orchestration consumes the bounded child lifecycle plus exact
  identity, state, and compatibility behavior.
- `FR-016 + FR-017 + FR-019 -> FR-018`
  Reason: consumer migration and deletion require workflow, qualification, and
  shared assurance-chain replacement paths.
- `FR-018 -> StR-003 completion`
  Reason: coexistence is a migration state; the stakeholder outcome is not achieved while unapproved legacy semantics remain.

### Shared dependencies

- Canonical identity encoding, stable typed errors, accepted-corpus access, and result-version validation are single library deliverables reused by every later CLI command.
- The assurance-contract registry and candidate snapshot are one population authority for Engineering Assurance and downstream contract/TL migrations; consumers must not recreate it locally.
- ix-flow and cli-agent-evals structured interfaces remain separately owned external gates, not local shims.

### Cross-cutting constraints

- `NFR-005` applies to every Rust source, test, generated executable path, package/Make dispatch, and hosted workflow touched by the migration.
- Every task uses exact Rust 1.98.1, bare `ix_trace_rs::trace` requirement markers, focused tests before full gates, and same-revision old/new evidence.
- Work is sequenced within this package to avoid concurrent semantic ports through shared files and to bound build pressure.

### The seams

Pure behavior lives in `src/` behind typed APIs. Filesystem, subprocess, environment, signal, and external-host behavior belongs to the `engineering-assurance` CLI. Retained Python/JavaScript/MJS paths remain reference implementations only until their corresponding same-revision removal gate passes.

## Test Plan

### Library and identity properties

- [x] **TC-096, TC-116**: The existing repository builds the library and CLI with exact Rust 1.98.1 and forbids unsafe code.
- [x] **TC-100, TC-103 compatibility subcase**: Rust compatibility classification matches the accepted retained corpus without writes.
- [ ] **TC-100, TC-102, TC-103 evidence subcase**: Evidence states, validation failures, accepted producer fixtures, generated JSON values, arbitrary-precision integers, and non-finite refusals match the reviewed boundary.
- [ ] **TC-100, TC-102..TC-104**: Portable semantic validation, projections, canonical fixture generation, and ownership/static constraints pass per capability.
- [ ] **TC-119, TC-120, TC-122**: Registry and candidate-snapshot identity, compatibility, mutation, duplicate, exclusion, and governing-profile cases pass.

### CLI and host integration

- [ ] **TC-098, TC-099, TC-101, TC-121**: Versioned CLI output, byte/deadline limits, buffered stdout, child termination, side-effect refusal, and library-I/O containment pass.
- [ ] **TC-105, TC-106**: Onboarding and ordered invariant results match retained behavior.
- [ ] **TC-107, TC-108, TC-123**: ix-flow lifecycle and host loading pass only against the exact accepted structured-provider artifact.
- [ ] **TC-109..TC-112, TC-124**: Evaluation, qualification, packaging, rights, hosted dispatch, and external-suite behavior pass against their retained cases and accepted host artifact.
- [ ] **TC-130..TC-135**: The fixed Quoin chain runs from declarative,
  pre-produced inputs; every preflight/child/response failure stops later
  actions; Quoin ownership and all eight consumer behaviors remain intact.

### Migration and qualification

- [ ] **TC-097, TC-113..TC-115, TC-125**: Every consumer has one state and disposition; deletion refuses incomplete parity, changed bindings, or unmigrated consumers and preserves historical bytes.
- [ ] **TC-117, TC-118**: Every Rust requirement test uses the canonical bare trace form and reconciles through Quire without missing, orphaned, or duplicate bindings.
- [ ] **TC-126..TC-129**: Same-runner performance, current-head hosted status,
  new-stable-toolchain adoption, and digest-bound trace-population gates pass.

## Remaining Work

### Track A: Shared implementation (serial)

- **A1 = TASK-014** Evidence availability and identity — Medium; exit: accepted and generated producer outputs preserve exact identity while invalid numeric inputs mint no digest.
- **A2 = TASK-015** Versioned CLI boundary — Hard; exit: protocol, resource, subprocess, and interruption failures are bounded and side-effect-free.
- **A3 = TASK-016** Semantic validation, projections, and fixtures — Hard; exit: every remaining pure ADR-002 capability matches its accepted corpus independently.
- **A4 = TASK-017** Assurance-contract registry — Hard; exit: one revision-bound population accounts for every active artifact and rejects ambiguity or mutation.
- **A5 = TASK-024** Quoin chain orchestration — Hard; exit: one Rust command replaces the eight local orchestrators without running producers or absorbing Quoin authority.
- **A6 = TASK-018** Onboarding and invariant core — Hard; exit: host-independent discovery, onboarding, and ordered invariants match retained behavior.
- **A7 = TASK-020** Evaluation and qualification core — Hard; exit: host-independent evaluation, package, rights, integration, and publication-refusal gates match retained behavior.

### Track B: External host gates (blocked)

- **B1 = TASK-019** ix-flow integration — Hard; exit: exact accepted provider identity loads the Rust invariant path and all lifecycle cases pass.
- **B2 = TASK-021** cli-agent-evals integration — Hard; exit: exact accepted suite identity drives all required host/scenario cells without inferred decisions.

### Track C: Post-gate migration

- **C1 = TASK-022** Consumer migration and removal — Hard; exit: every recorded consumer uses the qualified Rust interface before its legacy path is removed.
- **Gate = TASK-023** Final qualification — measures complete language containment, trace reconciliation, performance, and hosted enforcement; pass: every remaining migration TC is green at one candidate revision with no unapproved executable residue.

## Parallel Execution Summary

```text
Completed: TASK-012 -> TASK-013
Critical:             TASK-014 -> 015 -> 016 -> 017 -> 024 -> 018 -> 020
External gates:                                      019 ----\
                                                         021 -+-> 022 -> 023
```

The two host gates may advance in their owning repositories when their prerequisites exist, but Engineering Assurance modifies its shared package serially. Ecosystem follow-on work remains ordered as quire-research #59, then #61, then #63; this bundle does not authorize simultaneous downstream ports.

## Task File Mapping

| Task | Track | Owns (references) | Verified by (verifies) | Status |
| --- | --- | --- | --- | --- |
| TASK-012 | Done | FR-014, NFR-005 | TC-096, TC-116 | done |
| TASK-013 | Done | FR-015 | TC-100, TC-103 | done |
| TASK-014 | A | FR-015, NFR-005 | TC-100, TC-102, TC-103, TC-117, TC-118 | in_progress |
| TASK-015 | A | FR-014 | TC-098, TC-099, TC-101, TC-121 | in_progress |
| TASK-016 | A | FR-015 | TC-100, TC-102, TC-103, TC-104 | not_started |
| TASK-017 | A | FR-015 | TC-119, TC-120, TC-122 | not_started |
| TASK-024 | A | FR-019 | TC-130..TC-134 | not_started |
| TASK-018 | A | FR-016 | TC-105, TC-106 | not_started |
| TASK-019 | B | FR-016 | TC-107, TC-108, TC-123 | blocked |
| TASK-020 | A | FR-017 | TC-110, TC-111, TC-112 | not_started |
| TASK-021 | B | FR-017 | TC-109, TC-124 | blocked |
| TASK-022 | C | FR-018 | TC-097, TC-113, TC-114, TC-115, TC-125 | blocked |
| TASK-023 | Gate | StR-003, NFR-005 | TC-115..TC-118, TC-126..TC-129 | blocked |

## Coordination Rules

- One writer owns `src/`, `tests/`, the Rust lockfile, and qualification configuration for each Track A slice; the next slice starts only after review disposition is recorded.
- TASK-019 and TASK-021 do not implement foreign-language shims. They remain blocked until their external owners accept exact artifact identity, version, revision, digest, and compatibility gates.
- Contract/TL consumers may inventory and plan while #59 is active, but
  migration and helper removal wait for the usable shared boundary from
  TASK-016/TASK-017/TASK-024 and follow quire-research #60 ownership.
- Every semantic change returns through `specify` and base `spec-review`; every Rust slice receives the repository `/rust-review` gate and external pull-request review before merge.
- Qualification runs one resource-intensive command at a time, begins with focused tests, and caps Cargo parallelism when other agents are active.
