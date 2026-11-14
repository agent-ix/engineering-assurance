---
id: SR-20261113192938
title: "Code and Rust review of EA-115 ESRCH containment handling"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance; PR 240 (EA-115); head 67566d3f6a16b2e52e8dff71158a8ae6db43d4b8; base 4b4bc9f26841d8df4ef2ec5093c856f71a82526e; src/producer_execution.rs; tests/record_sink.rs (TC-651 refactor)"
review_set: subset
relationships:
  - { target: "ix://agent-ix/engineering-assurance/spec", type: references }
---

# Code and Rust review of EA-115 ESRCH containment handling

## Summary

Initial review covered PR 240 at head `b55844a5e5fa81df7deff3a7e3ee429e528d8549`. Refreshed the review at frozen head `aec85f5c5feeadfe6def188912281412be41cb6d` against base `4b4bc9f26841d8df4ef2ec5093c856f71a82526e`; current head `67566d3f6a16b2e52e8dff71158a8ae6db43d4b8` adds only a TC-651 helper refactor in `tests/record_sink.rs` after that refresh. Rust review was folded into code-review. The proposed ESRCH classification is localized to FR-019 descendant observation and the regression test preserves a real non-ESRCH failure assertion. The initial reviewed head had a nonexistent rustix errno constant; the refreshed head has the correction and the focused test passes.

At initial head `b55844a5e5fa81df7deff3a7e3ee429e528d8549`, the focused test and all-target Clippy failed at `src/producer_execution.rs:2004` because `Errno::ACCES` does not exist; rustc suggested `Errno::ACCESS`. At refreshed head `aec85f5c5feeadfe6def188912281412be41cb6d`, the focused test compiled and passed (1 passed, 0 failed). Library-only Clippy had passed at the initial review because it did not compile tests. `cargo fmt --all -- --check` reports formatting differences in three unrelated generated campaign files. `cargo deny check` could not acquire the Cargo advisory database lock (`db.lock`) because its configured path is read-only. No build, test, lint, or CI workflow files changed.

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


## Rebase refresh

At frozen head `aec85f5c5feeadfe6def188912281412be41cb6d`, `src/producer_execution.rs:2004` contains `Errno::ACCESS`; the focused TC-125 regression test compiled and passed (1 passed, 0 failed). Inspection of the old commit object `b55844a5e5fa81df7deff3a7e3ee429e528d8549` shows it still contained `Errno::ACCES`, contrary to the round-1 evidence row that cited `ACCESS` at that head. This refresh supersedes that stale evidence: the correction is verified at the rebased head. No new code-review findings were found.


## Incremental review at 67566d3

Compared `241d35caa7b39ef6f9254eafcbf1850e572f3da5..67566d3f6a16b2e52e8dff71158a8ae6db43d4b8`; the only changed path is `tests/record_sink.rs`. The refactor extracts the same `Boundary` visitor, recursive source scan, dependency metadata check, and assertions into helpers. It preserves the exact expected signer methods, empty concrete-implementation list, injected trait-object check, three signer-call/receiver pairs, and declared dependency assertions. The `#[trace("TC-651", "NFR-010-AC-2", "NFR-010-M-1")]` binding is unchanged, and no lint suppression attribute was added. The focused TC-651 test passed (1 passed, 0 failed); focused Clippy for `--test record_sink` passed with `-D warnings`. No new findings; FND-001 remains fixed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified at frozen head `558f76889c2b5df8f2a434ad01a49f84cf8e786d`, `src/producer_execution.rs:2004`: `let eacces = rustix::io::Errno::ACCESS.raw_os_error();`. Focused `cargo test --locked --features full --lib producer_execution::descendant_inspection_tests::tc_125_vanished_process_esrch_is_tolerated_but_other_errors_fail` passed at this head (1 passed, 0 failed); this compiled the corrected test target. |
| FND-001 | fixed | Reconfirmed at rebased head `aec85f5c5feeadfe6def188912281412be41cb6d`, `src/producer_execution.rs:2004`; the focused TC-125 test compiled and passed (1 passed, 0 failed). |
| FND-001 | fixed | At final head `67566d3f6a16b2e52e8dff71158a8ae6db43d4b8`, FND-001 remains fixed; the incremental TC-651 refactor preserves assertions, adds no lint suppression, and passes the focused test and Clippy checks. |
