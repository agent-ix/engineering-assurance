---
id: NFR-005
title: "Enforce Rust containment and test traceability"
type: NFR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "constrains"
---

# NFR-005: Enforce Rust containment and test traceability

## Statement

Engineering Assurance first-party production and qualification semantics SHALL
be implemented, tested, and locally qualified in Rust.

## Scope

This constraint covers libraries, CLIs, generators, validators,
canonicalization, result adapters, workflow and evaluation assertions, fixture
and package audits, and rights checks. Declarative host configuration and inert
foreign-language fixture samples are not semantic implementations.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Unapproved first-party non-Rust semantic or assertion paths | 0 | 0 | executable-path and host-configuration audit |
| Unsafe Rust blocks | 0 | 0 | `#![forbid(unsafe_code)]` plus source audit |
| Supported and qualification Rust version | exactly 1.98.1 | every declared target and required tool succeeds | pinned local compatibility run |
| Requirement-verifying Rust tests with canonical ix-trace-rs markers | 100% | 100% | parsed Rust-syntax audit plus Quire coverage |

## Rationale

The port must cover actual executable paths, while test traces use the exact
macro form Quire recognizes. Rust 1.98.1 is the selected compiler; an older
version is not justified merely because existing files already name it.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-005-AC-1 | The Rust package builds and tests every declared target locally with exact Rust 1.98.1 and forbids unsafe code in every first-party crate target. | Compile (TC-116) |
| NFR-005-AC-2 | Every first-party Rust test under `src/` or `tests/`, excluding vendored crates, imports `ix_trace_rs::trace` without an alias and uses at least one bare `#[trace(...)]` carrying a `TC-XXX` and an `*-AC-N` literal; missing, aliased, path-qualified, or malformed markers are rejected by parsed-syntax inspection. | Static (TC-117) |
| NFR-005-AC-3 | Quire reconciles Rust criterion markers to the test matrix with no missing, orphaned, or duplicate binding. | Integration (TC-118) |
| NFR-005-AC-4 | The final executable-path and host-configuration audit reports zero unapproved non-Rust semantic or assertion paths. | Static (TC-115) |

## Verification

Using exact Rust 1.98.1, run the repository's formatting, Clippy, build, test,
documentation, dependency-policy, Quire traceability, package, and rights checks
locally. Inspect Rust syntax with the Rust-owned pure source-audit boundary, and
inspect first-party executable and host-dispatch paths against ADR-002. The
source audit refuses a non-UTF-8, syntactically invalid, or greater-than-
2,097,152-byte individual source before analysis. Its alias resolution is
lexical and per document; it does not replace compiler or whole-program gates.

## Dependencies

FR-014 through FR-018 define the port scope constrained by this requirement.
`ix-trace-rs` is a Rust test-only dependency whose license and source
disposition are part of the package audit.
