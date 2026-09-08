---
id: NFR-005
title: "Enforce Rust containment and canonical test traceability"
type: NFR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "constrains"
---

# NFR-005: Enforce Rust containment and canonical test traceability

## Statement

Engineering Assurance first-party production and qualification semantics SHALL
be implemented, tested, and audited in Rust.

## Scope

This constraint covers libraries, CLIs, generators, validators,
canonicalization, result adapters, workflow assertions, evaluation assertions,
fixture and package audits, rights checks, and CI/release assertions. Declarative
host configuration and inert foreign-language fixture samples are not semantic
implementations.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Unapproved first-party non-Rust semantic or assertion paths | 0 | 0 | complete executable-path and inline-CI audit |
| Unsafe Rust blocks | 0 | 0 | `#![forbid(unsafe_code)]` plus source audit |
| Supported Rust version | 1.75 | build succeeds | pinned toolchain build |
| Requirement-verifying Rust tests with canonical ix-trace-rs markers | 100% | 100% | Quire coverage plus static macro-form audit |

## Rationale

A language label is not containment. The gate must inspect actual executable and
generated paths, while test traces must use the exact macro form Quire recognizes
so passing tests bind to reviewed acceptance criteria.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-005-AC-1 | The Rust package builds with Rust 1.75 and forbids unsafe code in every first-party crate target. | Compile (TC-116) |
| NFR-005-AC-2 | Every Rust test that verifies a specification criterion imports `ix_trace_rs::trace` and uses bare `#[trace("TC-...", "...-AC-...")]`; no path-qualified trace macro is accepted. | Static (TC-117) |
| NFR-005-AC-3 | Quire reconciles every Rust criterion marker to the test matrix with no missing, orphaned, or duplicate TC binding. | Integration (TC-118) |
| NFR-005-AC-4 | The final executable-path and inline-CI audit reports zero unapproved non-Rust semantic or assertion paths. | Static (TC-115) |

## Verification

Build and test with the reviewed minimum toolchain, run formatting and Clippy
with warnings denied, run Quire coverage from the repository root, and inspect
all tracked executable, generated, package-script, Make, and hosted-workflow
paths against ADR-002's matrix.

## Dependencies

FR-014 through FR-018 define the implementation and migration scope constrained
by this requirement. `ix-trace-rs` is a Rust test-only dependency and its license
and source disposition are part of the package audit.
