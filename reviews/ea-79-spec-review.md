---
id: SR-16302
title: "spec-review of FR-029 producer-execute command, TC-214..218 and PLAN-003 (engineering-assurance PR 192)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@ea-79-producer-execute; spec/functional/FR-029-producer-execute-command.md; spec/tests.md TC-214..TC-218; plan/PLAN-003-producer-execute-command"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-029
    type: references
---

# Spec review of FR-029, TC-214..TC-218 and PLAN-003 (engineering-assurance PR 192)

## Summary

Ticket: EA-79. The coder wrote the spec by hand without the spec skills, so this pass applies integrity, EARS and the spec-matrix check. FR-029, PLAN-003 and its two tasks pass `quire validate`. `quoin matrix` shows FR-029-AC-1 to AC-5 each with a tagged test (TC-214..218). The requirement is atomic, testable and consistent with FR-019 (executor, request, result and closed states unchanged) and FR-028 (exit statuses and error codes are domain-owned, so exit 0 for every closed execution state and exit 2 for no-result errors is a stated FR-029 decision, not a conflict). IT-006 covers the library consumer and does not state CLI exit statuses. Three defects were fixed on the branch at head 6aa635d.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Every TC-214..218 row in spec/tests.md and the coverage table said the same "completed-state assertions run on Linux only" and a green check mark, including TC-215 and TC-217 where it is false, and it hid that nothing in this review environment ran the Linux branch (Docker down). Reword per test: which assertions are Linux only, and that they were not executed on the macOS authoring host. | spec/tests.md | wrong-requirement |
| FND-002 | medium | The published schemas carry neither `$schema` nor `$id`, yet FR-029 promises draft 2020-12 documents, and the repo's other published schema declares both. A consumer validator would guess the dialect. Add both, with $id under schemas.agent-ix.org/agent-ix/engineering-assurance/. | src/producer_execute_schemas/*.json | wrong-requirement |
| FND-003 | low | The result schema was only ever checked against a `containment_failure` line on non-Linux hosts, so the `completed` and `failed` shapes and the `process` object were untested there. Added a test that serializes completed and failed results with the real library encoder and validates them against the schema on every host. Added an oversized-stdin case for `producer_execute_request_unreadable`, which FR-029 states and TC-215 did not exercise. | tests/producer_execute_cli.rs | correct-requirement-no-evidence |
| FND-004 | low | The FR-029 index entry sat between FR-019 and FR-020, out of order. Moved after FR-027. | spec/spec.md | wrong-requirement |
| FND-005 | low | FR-029-AC-1 and AC-3 each bundle several outcomes in one criterion. Acceptable because each is one test, but a split would make failures attributable. Not changed. | spec/functional/FR-029-producer-execute-command.md | wrong-requirement |

## Verdict

Mergeable on the spec side after the fixes in 6aa635d. Unverified: the Linux-only assertions (state `completed` with exit code and stdout bytes in TC-214, `timed_out` in TC-216) have not run anywhere, because the Docker daemon on this host is down and no other machine was used.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Per-row wording in spec/tests.md. |
| FND-002 | fixed | `$schema` and `$id` added to both files. |
| FND-003 | fixed | Two tests added, both tagged. |
| FND-004 | fixed | Entry moved. |
| FND-005 | accepted | Left as is. |
