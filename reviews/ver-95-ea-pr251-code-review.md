---
id: SR-20261113192949
title: code-review of observed producer timeout cleanup
type: SpecReview
analysis: code-review
scope: agent-ix/engineering-assurance; src/producer_execution.rs; tests/producer_execution.rs;
  tests/fixtures/producer_execution_fixture.rs; FR-019/TC-125
review_set: subset
relationships:
- target: ix://agent-ix/engineering-assurance/FR-019
  type: references
---

## Summary

Code review with Rust review lane. No actionable defect found in the assigned change.

Ticket: VER-95. PR: agent-ix/engineering-assurance#251. Frozen head: `2a99ea66ae6b6dba27afb9259464b269926d2562`. Diff: `git diff origin/main...HEAD`. Merged spec PR #250 governs; EA-115 is a separate completed ESRCH-race change, not this review's finding target.

The assigned scope is the producer timeout change in `src/producer_execution.rs`, `tests/producer_execution.rs`, and `tests/fixtures/producer_execution_fixture.rs`. No applicable active AssuranceProfile was found. No branch changes, aggregate gate, merge, or unrelated repository review was performed.

## Verdict

PASS. Code review with Rust review lane. No actionable defect found in the assigned change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

The fixture's producer waits on a nested Rust child; that child uses `setpgid` (not `setsid`) to create a distinct group in the same session, inherits both invocation pipes, records its PID, and sleeps beyond the 350ms request deadline. The regression waits for both PID markers and confirms both processes initially live. It rejects completion before the deadline, requires worker completion by deadline + 1,000ms, joins the worker, asserts `ContainmentFailure`, requires retained terminal status, and verifies neither observed PID is live. A zombie is not counted as a live descendant; the direct child's terminal status comes from `Child::wait`/`try_wait`, which reap that direct child.

The supervisor accumulates observed descendants and latches the escape flag until terminal/deadline cleanup. Cleanup signals the invocation group and every tracked descendant, kills and waits for the direct child, then the capture function joins stdout and stderr reader handles before returning this fixture's successful capture. Thus the public worker's completion demonstrates both reader joins on the exercised path; there is no detached-reader shortcut. The test has independent timing, process-liveness and terminal-state oracles. The baseline's immediate escape return fails the lower-bound assertion. Stream/input/output/concurrency limits and cancellation behavior are not widened by the change; existing TC-125 boundary and cancellation/escape checks pass.

This is evidence for the specified observed, cooperative fixture. It is not a guarantee for unobserved descendants, adversarial process trees, arbitrary kernel stalls, or all host failure modes. No such stronger claim is introduced. Source coverage is not execution evidence for every failure branch.

## Checks and limitations

- Independent focused check: `cargo test --locked --no-default-features --features producer-execution --test producer_execution tc_125 -- --test-threads=1`, run through the required locked-build wrapper with the assigned worktree target and `RUSTUP_TOOLCHAIN=1.98.1 CARGO_PROFILE_DEV_DEBUG=0`. Result: 3 passed, 0 failed, 21 filtered out; test execution 2.38s. The command's tool-session completion status was lost during output truncation; the retained log contains the successful complete test-result summary and no matching build process remains.
- Read-only checks: frozen HEAD, clean worktree, complete three-file diff, governing FR-019/TC-125, process supervision/capture/termination call paths, Linux platform gates, SPDX headers, stub/ignore census. No new ignored test, placeholder implementation, unsafe block, public API, dependency, or domain oracle.
- `quoin matrix --repo . --json` and `quire trace --scope . --id FR-019 --prefix --format json` succeeded. Tools: `quoin` 0.30.1; `quire` 0.45.0 (engine 0.60.0); spec-artifacts-process v0.31.0. No older binary fallback.
- Coder-reported broader checks (not independently rerun): formatting, 24 producer-execution tests, focused clippy, rights acceptance, pre-push lint/78 tests/package audit. Dispatch also reports an existing `gh pr create` hook running tests/clippy using a shared Cargo target directory outside the required build wrapper. That workflow issue is outside this diff; this review neither reproduces it nor treats that output as its own verification. Every reviewer build used the assigned wrapper/target.

## Examined scope

```yaml
scope:
- id: FR-019-AC-3
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: context_only
  excerpt: Unavailable, refused, failed, timed-out, malformed-response, containment-failure, cancelled, and completed remain distinguishable and canonically serializable; launched failures retain bounded raw evidence where observable; and only completed carries `T`.
- id: FR-019-AC-4
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: examined
  excerpt: Exact wall-clock, stream, input-byte, output-count/byte and concurrency boundaries are admitted while the next value is structurally invalid or terminated; an admitted non-zero exit reaches the adapter; ordinary descendants are reaped; an escaping-descendant mutant produces `containment_failure`;
- id: FR-019-AC-4
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: examined
  excerpt: and when an observed nested descendant leaves the invocation process group, keeps an invocation stream open, and remains alive past the request deadline, the executor terminates it, completes both stream-reader workers, and returns within 1,000 milliseconds after the deadline with no surviving fixture descendant or reader worker.
- id: FR-019-CON-4
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: examined
  excerpt: The `process-group-v1` profile SHALL NOT be represented as adversarial full-descendant containment.
- id: FR-019
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: examined
  excerpt: "When the executor observes a\n  descendant outside the invocation-owned process group, the executor SHALL\n  keep that process tracked for cleanup. When cleanup runs, the executor SHALL\n  terminate each observed descendant as well as the invocation-owned group."
- id: FR-019
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: examined
  excerpt: "When the request deadline is reached, the executor SHALL complete process\n  cleanup, reap the direct child, and finish both invocation stream-reader\n  workers within 1,000 milliseconds of that deadline. While cleanup remains\n  pending, the executor SHALL NOT return with an observed fixture descendant\n  alive or an invocation stream-reader worker still running."
- id: TC-125
  path: spec/tests.md
  role: examined
  excerpt: Exact wall-clock, stream, input-byte, output-count/byte, and concurrency boundaries are admitted and the next value is invalid or terminates; admitted non-zero exits reach the adapter; ordinary descendants are reaped; an escaping-descendant mutant becomes containment-failure under the explicitly cooperative profile;
- id: TC-125
  path: spec/tests.md
  role: examined
  excerpt: and a Rust fixture whose observed nested child moves to a distinct process group, holds an invocation stream open, and survives the request deadline is terminated with both reader workers complete within 1,000 ms after the deadline, leaving no fixture descendant alive
```

Artifact validation exited 0 at both explicit scratch paths. It emitted `semantic.unused-import` for `agent-ix/spec-artifacts-iso` in `spec-artifacts-app`, and `DuplicateModuleName` for `engineering-assurance` declared at two paths (first-wins). These installed-module diagnostics were not suppressed; the validation command and diagnostic output were retained separately. No validation error or fallback was reported.
