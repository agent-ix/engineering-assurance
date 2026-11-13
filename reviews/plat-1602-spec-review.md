---
id: SR-16425
title: "spec-review of FR-044 document converter producer requests (engineering-assurance PR 222)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@406604ec; spec/functional/FR-044-document-converter-producer-requests.md; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-045
    type: references
---

# Spec review of FR-044 document converter producer requests (engineering-assurance PR 222)

## Summary

Ticket: PLAT-1602. This review covered the integrity and EARS portions of spec-review, checked against quire-cli FR-031, FR-032, FR-033 and NFR-010 at quire-cli `origin/main` dc1876e. It examined:
- the FR statement, Inputs, Outputs and the four Behavior bullets
- CON-1 and AC-1 to AC-6
- the `spec/spec.md` index entry
- the TC-368 to TC-376 rows

`quire validate` on FR-044 is clean; it shows only pre-existing module warnings.

These parts are sound:
- The FR keeps the request field table in quire-cli FR-032 and does not duplicate it.
- It states plainly that Engineering Assurance adds no builder, field, state, dependency or feature.
- The Behavior bullets are EARS-conformant.
- The ids do not collide with FR-043 or TC-367.
- CON-1 mirrors the accepted FR-038-CON-1.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The ticket AC "a missing converter binary gives a clear error naming it" has no home in FR-044. AC-3 and TC-370 assert only state `unavailable` and no artifact. The FR does not say that the naming comes from the `producer` descriptor the result echoes, or that the diagnostic is quire-cli FR-033's job. Add one clause that names the owner. If the result's `producer.name` and `producer.executable` are the evidence, extend AC-3 to assert them. | spec/functional/FR-044-document-converter-producer-requests.md:46-47,66 |
| FND-002 | low | The Behavior bullet "If the converter executable cannot be opened or launched, then the result state SHALL be `unavailable`" is broader than the executor's behavior. A path that opens but is not a regular file, such as a directory, yields `refused` (`ExecutionRefusal::Executable`, src/producer_execution.rs:1316-1318), not `unavailable`. Narrow the trigger to "does not exist, or cannot be opened or launched as a regular file", and note that a non-file is `refused`. FR-033 still classes `refused` as a converter failure. | spec/functional/FR-044-document-converter-producer-requests.md:46-47 |
| FND-003 | low | The new TC rows put a hand-written run status ("✅ ... passing (Linux)", "🚧 NOT yet executed") into the spec's test table. That status drifts from the code and duplicates what `quoin matrix` computes from evidence. This follows the repository convention for every row, so the fix is repo-wide rather than confined to this PR. Raised because hand-kept status tracking is an antipattern under the owner's rules. | spec/tests.md:599-607 |

## Verdict

FR-044 is well bounded and consistent with the merged quire-cli export spec. It has three low findings and is mergeable.

## Dispositions

Round 1, at head 4be67958. "FR-044" in this file means the converter FR, now FR-045 after renumber 4be6795. The relationship target was repointed to FR-045.

The renumber is complete:
- the file is renamed and its `id`, H1, CON-1 and AC-1 to AC-6 are updated
- the `spec/spec.md` entry and the nine `spec/tests.md` rows are updated
- the module doc and the `#[trace]` tags in the test are updated
- the PR title now reads FR-045

No FR-044 reference remains outside `reviews/`, and FR-045 collides with nothing. `quire validate` on the FR-045 file alone is clean.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The Behavior text now states that the result's `producer.name` and `producer.executable` echo the request and that the user-facing diagnostic belongs to quire-cli FR-033. AC-3 and TC-370 now assert both fields. |
| FND-002 | fixed | The trigger is narrowed to "does not exist or cannot be opened without following a symbolic link". A non-regular file is now `refused`, and TC-370 asserts that for a directory. |
| FND-003 | rejected | This was the reviewer's own error. The Status column is required by the module's TestMatrix archetype, which requires each cell to match `^(✅\|❌\|🚧\|⛔)`. The repository cannot drop it per row, and blanking the cells breaks validation (see FND-004). |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | The fix for FND-003 blanked the Status cell of TC-368 to TC-376. `make validate-docs` now fails: `[TestMatrix] 'test_cases': TC-368: column 'Status' cell '' does not match pattern /^(✅\|❌\|🚧\|⛔)(\s+.*)?$/` for all nine rows, and "1 document(s) failed structural validation" (exit 3). Restore a status mark in each cell, for example `✅` for TC-368 to TC-373 and TC-376 and `🚧` for TC-374 and TC-375. | spec/tests.md:599-607 |
