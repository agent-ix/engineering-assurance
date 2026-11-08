---
id: SR-16306
title: "code-review of the scope-footprint command (EA-78)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@ea-78-scope-footprint; src/scope_footprint.rs; src/scope_footprint_host.rs; src/main.rs; tests/scope_footprint.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-032
    type: references
---

# Code review of the scope-footprint command (EA-78)

## Summary

Ticket: EA-78. Reviewed the diff only. The pure module performs no I/O and reuses the crate's normalized-relative-path rule; the git adapter runs without user or system git configuration and never writes. Each new test was shown to fail under a temporary mutation of the rule it asserts (containment, truncation precedence, unavailable status, validator, duplicate check, rename arity, option-like revision, output-too-large mapping, path-count truncation). `cargo fmt --check`, `cargo clippy --all-targets --all-features -D warnings`, `cargo doc` and `cargo deny` are clean.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | `unused_scope` listed every entry when the footprint was truncated or unavailable, which misreports over-declaration. It is now empty unless the footprint was fully observed. | src/scope_footprint.rs | wrong-requirement |
| FND-002 | low | Repository-selecting and config environment variables were not all removed. `GIT_COMMON_DIR`, `GIT_CONFIG_PARAMETERS`, `GIT_CONFIG_COUNT` and `GIT_REPLACE_REF_BASE` were added. | src/scope_footprint_host.rs | wrong-requirement |
| FND-003 | low | The feature table in lib.rs listed transitive dependencies. Now direct ones only. | src/lib.rs | wrong-requirement |
| FND-004 | low | The footprint is a direct two-tree comparison, not a merge-base diff. FR-032 now says the caller passes the merge-base as `base`. | spec/functional/FR-032-declared-scope-footprint-verification.md | wrong-requirement |
| FND-005 | low | A changed path that breaks the normalized-path rule (backslash, control character, non-UTF-8, over 4096 bytes) makes the command fail closed with exit 2 or `incomplete` rather than produce a verdict. A hostile filename therefore cannot pass the check; it is not classified `false_scope`. Kept as specified. | src/scope_footprint.rs | wrong-requirement |
| FND-006 | low | An unresolvable revision or non-repository reports `incomplete`, as the ticket requires; a gate must treat anything other than `complete` as not passing. Kept. | src/scope_footprint_host.rs | wrong-requirement |

## Verdict

Mergeable. The pure-function tests require `full` because they share a test target with the command tests.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Test added in TC-236. |
| FND-002 | fixed | Variables added. |
| FND-003 | fixed | Row corrected. |
| FND-004 | fixed | Spec note added. |
| FND-005 | deferred | Fail-closed is the safe direction; a classification for unrepresentable paths is a follow-up. |
| FND-006 | deferred | Matches the ticket's unavailable-to-incomplete rule. |
