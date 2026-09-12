---
id: SR-118
title: "Review of the minimal producer-execution consumer feature boundary"
type: SpecReview
analysis: code-review
scope: "FR-019-AC-7, TC-128; the library feature boundary in src/lib.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: reviews
---

## Summary

`make rust-foundation-gate` did not pass at `main`. TC-128
`minimal_downstream_compiles_only_producer_execution_feature` failed: a
downstream crate depending on Engineering Assurance with `default-features =
false, features = ["producer-execution"]` could not compile the library at all.

`src/lib.rs` gated every full-only module behind `#[cfg(feature = "full")]`
except one. `pub mod evaluation;` was unconditional, and `src/evaluation.rs`
imports `time`, which is a `full`-only optional dependency:

```
error[E0433]: cannot find module or crate `time`
  --> src/evaluation.rs:13:5
```

`evaluation` has no consumer inside the `producer-execution` feature: the only
module that references it is `src/integration_evidence_host.rs`, a binary-only
module that already requires `full`. Gating it is therefore an exact fix, not a
weakening of the boundary — the feature split was correct, one declaration was
simply omitted from it.

`make rust-foundation-gate` now passes end to end (exit 0, no failing test).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-219 | high | Closed. `pub mod evaluation;` was the one library module not confined to the `full` feature, so the FR-019 minimal-consumer boundary did not exist in practice and `make rust-foundation-gate` — and therefore `make integration-gate` and `make release-gate` — could not pass. | `src/lib.rs:20` | implementation-bug-despite-evidence |
| FND-220 | high | Closed by this review rather than by code. The TC-128 row in `spec/tests.md` read `✅ isolated offline downstream compile and activated direct-dependency census passing` at `d9d3023`, where the test was red. A P0 compile gate was reported green while failing, which is precisely the matrix-truth failure the v0.3.1 patch set out to end. The row is accurate as of this change; the escape is that it was published before the gate was run. | `spec/tests.md:307` | correct-requirement-no-evidence |

## Scope limits

This review covers the library feature boundary only. It does not re-review the
FR-019 producer-execution kernel, and it makes no claim about live agent
evaluation, the retained aggregate, or release evidence.

## Gate results

| Gate | Result |
| --- | --- |
| `cargo +1.98.1 test --test producer_execution` | pass (12 tests, TC-128 included) |
| `make rust-foundation-gate` | pass (exit 0; rust-format, rust-clippy, rust-toolchain, rust-tests, rust-docs, rust-deps, rust-audit) |
| TC-128 without the gate | fails to compile the minimal consumer (regression guard verified red) |
