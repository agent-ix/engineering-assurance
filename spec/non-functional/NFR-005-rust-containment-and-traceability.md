---
id: NFR-005
title: "Enforce Rust containment, current tool qualification, and test traceability"
type: NFR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "constrains"
---

# NFR-005: Enforce Rust containment, current tool qualification, and test traceability

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
| Supported and qualification Rust version | exactly 1.98.1 | every declared target and required tool succeeds | pinned compatibility run |
| Requirement-verifying Rust tests with canonical ix-trace-rs markers | 100% | 100% | Quire coverage plus static macro-form audit |
| Same-runner Rust parity-path p95 latency and peak RSS regression | none | no more than 10% above the retained path over 30 runs | versioned benchmark record |
| Required pull-request gates | all | `ea-rust-format`, `ea-rust-clippy`, `ea-rust-toolchain`, `ea-rust-tests`, `ea-quire-trace`, `ea-package-rights-static` at the current head | hosted CI status checks |

## Rationale

A language label is not containment. The gate must inspect actual executable and
generated paths, while test traces must use the exact macro form Quire recognizes
so passing tests bind to reviewed acceptance criteria. An inherited minimum
compiler is not evidence that it remains appropriate. The qualification version
tracks current stable Rust unless a required tool has a demonstrated
incompatibility; formatting output or newly reported lint does not justify
freezing an older compiler.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-005-AC-1 | The Rust package builds and tests every declared target with exact Rust 1.98.1 and forbids unsafe code in every first-party crate target. | Compile (TC-116) |
| NFR-005-AC-2 | Every Rust test that verifies a specification criterion imports `ix_trace_rs::trace` and uses bare `#[trace("TC-...", "...-AC-...")]`; no path-qualified trace macro is accepted. | Static (TC-117) |
| NFR-005-AC-3 | Quire reconciles every Rust criterion marker to the test matrix with no missing, orphaned, or duplicate TC binding. | Integration (TC-118) |
| NFR-005-AC-4 | The final executable-path and inline-CI audit reports zero unapproved non-Rust semantic or assertion paths. | Static (TC-115) |
| NFR-005-AC-5 | Same-runner benchmarks over compatibility classification, onboarding, invariant evaluation, result aggregation, and repository qualification record at least 30 old/new observations per capability; no Rust-path p95 latency or peak RSS exceeds the retained path by more than 10%. | Benchmark (TC-126) |
| NFR-005-AC-6 | A pull request that claims a Rust-migration criterion has successful `ea-rust-format`, `ea-rust-clippy`, `ea-rust-toolchain`, `ea-rust-tests`, `ea-quire-trace`, and `ea-package-rights-static` statuses bound to its current head revision; a missing, failed, stale-revision, or manually substituted status withholds the gate, and none of these jobs invokes a real-agent evaluation or release operation. | Integration (TC-127) |
| NFR-005-AC-7 | Within seven calendar days after a stable Rust release, the repository runs its real build, test, Clippy, rustfmt, documentation, dependency-policy, Quire, and package gates on that exact release and records adoption or a time-bounded hold. A hold names a reproducible incompatibility in a required tool and expires within 30 days or when that tool releases a compatible version, whichever occurs first; formatting changes and repairable lint findings are not incompatibilities. | Integration (TC-128) |

## Verification

Build and test with the exact supported toolchain, run formatting and Clippy
with warnings denied, run Quire coverage from the repository root, and inspect
all tracked executable, generated, package-script, Make, and hosted-workflow
paths against ADR-002's matrix. Exercise the same commands on each new stable
candidate and classify actual tool failures separately from formatting or lint
changes. Repository settings require the six named statuses on Rust-migration
pull requests; workflow configuration alone is not evidence that the branch
gate enforces them.

## Dependencies

FR-014 through FR-018 define the implementation and migration scope constrained
by this requirement. `ix-trace-rs` is a Rust test-only dependency and its license
and source disposition are part of the package audit.
