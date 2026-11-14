---
id: SR-20261113192938
title: "Code and Rust review of EA-115 ESRCH containment handling"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance; PR 240 (EA-115); src/producer_execution.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/engineering-assurance/spec", type: references }
---

# Code and Rust review of EA-115 ESRCH containment handling

## Summary

Reviewed PR 240 at frozen head `b55844a5e5fa81df7deff3a7e3ee429e528d8549` against `origin/main...HEAD`. Rust review was folded into code-review. The proposed ESRCH classification is localized to FR-019 descendant observation and the regression test preserves a real non-ESRCH failure assertion, but the test does not compile because it names a nonexistent rustix errno constant.

Focused `cargo test --locked --features full --lib producer_execution::descendant_inspection_tests::tc_125_vanished_process_esrch_is_tolerated_but_other_errors_fail` through the repository locked-build wrapper fails at `src/producer_execution.rs:2004`: `Errno::ACCES` does not exist; rustc suggests `Errno::ACCESS`. `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`, also through the locked wrapper, fails for the same error. `cargo clippy --locked --features full --lib -- -D warnings` passes because it does not compile tests. `cargo fmt --all -- --check` reports formatting differences in three unrelated generated campaign files. `cargo deny check` could not acquire the Cargo advisory database lock (`db.lock`) because its configured path is read-only. No build, test, lint, or CI workflow files changed.

## Scope

- Examined FR-019-AC-4: “Exact wall-clock, stream, input-byte, output-count/byte and concurrency boundaries are admitted while the next value is structurally invalid or terminated; an admitted non-zero exit reaches the adapter; ordinary descendants are reaped and an escaping-descendant mutant produces `containment_failure`.”
- Examined FR-019-CON-4: the process-group profile does not claim adversarial full-descendant containment.
- Examined TC-125 regression behavior in `src/producer_execution.rs` and the existing escape-mutation test in `tests/producer_execution.rs`.
- Examined the changed implementation in `src/producer_execution.rs` and the repository Rust conventions in `AGENTS.md`, `Cargo.toml`, Rust review checks, and applicable Rust-style references.

## Assurance Context

No applicable AssuranceProfile was found under `spec/` for producer descendant observation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The regression test refers to nonexistent `rustix::io::Errno::ACCES`, so the test target and all-target Clippy do not compile. Use the available `Errno::ACCESS` constant (or another valid non-ESRCH error) so the asserted preservation of other read errors can run. | src/producer_execution.rs:2004 |

## Verdict

**FAIL** — the added regression test cannot compile, so the change cannot pass the Rust test and all-target lint gates.
