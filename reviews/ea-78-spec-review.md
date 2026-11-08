---
id: SR-16305
title: "spec-review of FR-032, TC-232..238 and PLAN-005 (EA-78 scope footprint)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@ea-78-scope-footprint; spec/functional/FR-032-declared-scope-footprint-verification.md; spec/tests.md TC-232..TC-238; plan/PLAN-005-scope-footprint"
review_set: base
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-032
    type: references
---

# Spec review of the EA-78 scope-footprint bundle

## Summary

Ticket: EA-78. FR-032 takes the next free number after FR-031 (EA-80); TC-232..238 follow TC-231; PLAN-005 and TASK-016/017 follow PLAN-004. No active AssuranceProfile applies, so the base set was chosen. `quire validate` is clean on FR-032 and PLAN-005. The criteria were checked for the ability to fail: each acceptance criterion maps to a test that was shown to fail under a temporary mutation of the rule it asserts. No hash, digest or pin is introduced. The Linear ticket carried no separate scoping comment, so the design follows the dispatch brief.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The first draft left open whether an observed uncovered path outranks a truncated footprint. FR-032 now states that an observed uncovered path is `false_scope` even when truncated, and that only a truncated or unavailable footprint without one is `incomplete`. | spec/functional/FR-032-declared-scope-footprint-verification.md | wrong-requirement |
| FND-002 | low | FR-032-AC-3 had no named adverse constraint for truncation. FR-032-CON-1 now forbids `complete` for an unobserved footprint and is traced to TC-234. | spec/functional/FR-032-declared-scope-footprint-verification.md | correct-requirement-no-evidence |
| FND-003 | low | No published JSON Schema accompanies the request or result, unlike FR-029. The request is a closed typed document parsed with unknown fields refused and the result is a flat verdict; a schema is a follow-up if a non-Rust consumer asks for one. | spec/functional/FR-032-declared-scope-footprint-verification.md | wrong-requirement |

## Verdict

Mergeable on the spec side. FND-003 is recorded, not blocking.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Behavior bullets state the precedence. |
| FND-002 | fixed | Constraint added and traced. |
| FND-003 | deferred | No consumer needs a schema yet; the verdict fields are documented in FR-032. |
