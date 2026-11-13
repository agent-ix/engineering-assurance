---
id: SR-16422
title: "spec-review of FR-043 approval trail of a change-assurance record (engineering-assurance PR 220)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@be9a4c9; spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md; spec/spec.md; spec/tests.md; README.md"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-043
    type: references
---

# Spec review of FR-043 approval trail (engineering-assurance PR 220)

## Summary

Ticket: PLAT-1594. This review covered the integrity and EARS portions of spec-review: the statement, the Inputs, Outputs and Behavior sections, CON-1 to CON-3 and AC-1 to AC-13. It also checked the spec index entry, the TC-351 to TC-362 rows and the README row.

Several parts are clear and consistent:
- The FR statement is one obligation, and Out of scope is clear.
- The `claimed_at` and `signed_at` distinction is explicit: `claimed_at` is the signer's claim on any status, attests nothing, and `signed_at` stays approved-only.
- The ordering is total and deterministic.
- The ids do not collide with FR-042 or TC-338 to TC-350.
- `quire validate` on the FR is clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `skipped` has three definitions that disagree. Outputs says "annotated tags excluded because their message could not be read". The Behavior section adds tags whose name is not UTF-8. The implementation also counts every mechanics failure (timeout, oversized output, resolve or cat-file failure). Meanwhile the Description promises "never silently dropped", yet a bare repository-wide count is all that remains, which in effect is a silent drop. Fix: define each excluded tag as reported by name, escaped when it is not UTF-8, with its reason, and state that a mechanics failure is not an unreadable message: it is listed or it sets `truncated`. | spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md:30-35; spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md:66-69 |
| FND-002 | medium | AC-13 bundles three claims (the 300 s bound, the shared-host bounds, and that nothing is written) and labels itself "(CON-2)", but CON-2 is about having no signature, trailer or decision code of its own, which is a different claim. The time-limit outcome (`truncation` `time_limit`) is a testable behaviour verified only by inspecting that the bound is present. Fix: split AC-13. Add a Test AC for the time-limit result (`truncated`, `truncation`, and what `considered` counts). Keep an Inspection AC for the bound's value and for writing nothing, and make CON-2's own inspection explicit. | spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md:164 |
| FND-003 | low | `considered` is defined as "refs examined", but the spec does not say what it counts after a `time_limit` stop, and the implementation reports every listed ref. Fix: state that `considered` counts only the refs reached before the stop. | spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md:66 |
| FND-004 | low | spec/tests.md now disagrees with itself. The FR-014-AC-2 row (line 111) says "16 machine-facing commands" and lists them, while the TC-098 row (line 346) still says "15-command table". The hand-maintained command count and list is an inventory that drifts with every new command. Fix: delete the count and the command list from both rows and say "every machine-facing command in the package_boundary table". | spec/tests.md:111; spec/tests.md:346 |

## Verdict

FR-043 is a sound, well-bounded requirement. It reuses FR-041 by reference and does not restate it, and it is honest that the order rests on the signer's claim. Fix FND-001 and FND-002 in the same round as the code findings; FND-003 and FND-004 are small wording fixes.
