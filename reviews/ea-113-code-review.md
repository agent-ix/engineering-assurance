---
id: SR-907
title: "Code review — approval gesture signing and reporting (EA-113 PR 307)"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance; changed Rust source and tests for approval-tag, approval-verify, approval-trail, signature-verify, and the process host; FR-085-AC-1..11; FR-086-AC-1..6"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-085
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-086
    type: references
---

## Summary

Ticket: EA-113; PR 307 local review at a0b60649 against origin/main a04da1e1. The Rust implementation and changed tests were reviewed, including the EA-147 AC-1 restatement and FR-086 result propagation. One medium process-host lifecycle defect remains.

## Verdict

**CONDITIONAL** — interactive terminal handoff can silently fail or lose a prompt that arrives before promotion.

## Examined scope

- FR-085-AC-1 through FR-085-AC-11, with FR-085-AC-9 inspection-only behavior treated as context.
- FR-086-AC-1 through FR-086-AC-6, especially AC-5 status omission and AC-6 trail propagation.
- New Rust process-host, approval-tag, approval-verify, approval-trail, and signature-verify code and their changed integration tests.
- Cargo feature/dependency changes and changed test target registration.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Foreground handoff can fail silently or race a terminal read | src/producer_execution.rs:1607,1617,1689,1713 |

## Review notes

The FR-085-AC-1 wording now matches the EA-147 decision: it requires a distinct, non-foreground process group, not a detached session or absence of a controlling terminal. TC-1160 asserts the relationship under a pseudo-terminal. The changed FR-086 status tests assert that meaning_missing and signed_bad results omit gesture, and TC-1162 verifies declaration propagation for an approved entry.

ForegroundGroup::acquire runs after command.spawn. A fast child can attempt a /dev/tty read before tcsetpgrp, receive SIGTTIN and stop; changing the foreground group later does not itself resume it. Also, acquire returns None for tcsetpgrp errors and the run continues in the background; Drop ignores restoration errors. A signer can therefore stall until the 300-second deadline or leave the caller with an unrestored terminal group. The current PTY test covers successful handoff and restoration, not these failure paths.

## Test and gate evidence

- make lint: passed (ruff).
- rustfmt --check on changed Rust files: passed.
- Exact direct command: cargo clippy --locked --features full --test approval_tag_cli --test approval_trail_cli --test approval_verify_cli --test approval_gesture_pty -- -D warnings. Passed, but was invoked without locked-build.sh.
- Exact direct command: cargo test --locked --features full --test approval_tag_cli tc_11 -- --nocapture. 10 passed, 0 failed; invoked without locked-build.sh.
- Subsequent focused tests invoked through locked-build.sh: PTY deadline restoration 1 passed; TC-1160, TC-1161 and TC-1162 each passed; approval verification tc_32 filter 8 passed; signature gesture tc_114 filter 8 passed.
- Exact command make test was interrupted at the content-rights-tree prerequisite, exit 130, as directed; the full suite was intentionally not run. make ci and make package-audit were not run.
- cargo deny and whole-workspace clippy/fmt gates were not run because whole-repository checks were disallowed.

