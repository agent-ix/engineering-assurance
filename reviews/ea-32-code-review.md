---
id: SR-16307
title: "code-review of the reported interval bound (engineering-assurance PR 197)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@fcdd162; src/measurement.rs; tests/measurement.rs; spec/tests.md TC-239"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-021
    type: references
---

# Code review of the reported interval bound (engineering-assurance PR 197)

## Summary

Ticket: EA-32. Reviewed `git diff origin/main...origin/ea-32-interval-bound` only (head fcdd162), with the rust-review lane folded in against the Agent IX rust-style baseline. The change adds `IntervalBound`, `IntervalJudgement` and `DecisionRule::holds_on_interval_reporting`. It also makes `holds_on_interval` delegate to the new method and map the result to `.holds`. The change only adds API: no existing signature, error variant or wire type changes, and no compatibility layer is introduced. The refusal sequence (non-finite estimate, interval not requested, missing interval, level too low, estimate outside interval, eq arm, reference resolution) moved into the new method unchanged, so both methods refuse identically by construction. The bound is chosen before the reference is resolved, so the reported kind and value do not depend on the threshold, absolute-margin or relative-margin reference.

Mutation checks, each reverted after the run: (1) swapping only the `IntervalBound` labels fails TC-239; (2) reporting `value: estimate` instead of the bound fails TC-239; (3) swapping both arms (`gt`/`ge` to the upper bound, `lt`/`le` to the lower) fails TC-239 and TC-193. The existing TC-192/TC-193 tests pass unchanged. That confirms `holds_on_interval` behaves as before. `cargo clippy --features full --lib --test measurement -D warnings` and `cargo fmt --check` are clean. The 35 measurement `tc_` tests pass.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | TC-239 in spec/tests.md says it covers the absolute-margin baseline reference "when the rule holds and when it does not". The test has only one absolute-margin baseline case (`ratchet`, `ge`, holds = true). It has no failing absolute-baseline case and no `lt`/`le` absolute-baseline case. The AC is still met through the relative-margin baseline cases, and bound selection does not depend on the reference, so no behaviour goes unverified. The TC text claims more than the test does. Either add a failing `ratchet` case (for example interval 0.71..0.85 against reference 0.72, giving `(false, Lower, 0.71)`) or narrow the TC wording. | tests/measurement.rs:2134-2141; spec/tests.md:467 | correct-requirement-no-evidence |
| FND-002 | low | `holds_on_interval` and `holds_on_interval_reporting` carry nearly identical doc comments, including a verbatim 10-line `# Errors` section, although one only delegates to the other. The two copies will drift the next time a refusal is added. `holds_on_interval` could say it returns the `holds` of `holds_on_interval_reporting` and point to that method's `# Errors`. | src/measurement.rs:1128-1143; src/measurement.rs:1154-1170 | wrong-requirement |

## Verdict

Mergeable. Both findings are low and do not affect behaviour. The new API is additive. `holds_on_interval` returns the same verdict and the same refusals as before. Bound kind and value are correct for `gt`/`ge`/`lt`/`le` with threshold, absolute-margin and relative-margin baselines. Mutation shows the tests pin the bound kind and value. Naming and derives follow rust-style and the module's local conventions: closed enums with no `#[non_exhaustive]`, documented public items, and `Copy`/`PartialEq` without `Eq` on the `f64`-carrying struct.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit 9b11ff4 adds a failing absolute-margin `ge` case (`(false, Lower, 0.71)`) and an absolute-margin `le` rule (margin -5, reference 105) with a holding case `(true, Upper, 104.0)` and a failing case `(false, Upper, 106.0)`. The TC-239 row now says "absolute-margin baseline (higher and lower is better)". tc_239 passes. |
| FND-002 | fixed | Fix commit 9b11ff4 reduces the `holds_on_interval` doc to a pointer to `holds_on_interval_reporting` for its semantics and `# Errors`. The duplicated text is gone. clippy and fmt are clean. |
