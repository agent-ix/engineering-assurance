---
id: SR-16309
title: "spec-review of FR-021-AC-15 and TC-239 (engineering-assurance PR 197)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@fcdd162; spec/functional/FR-021-measurement-plan-decision-rule.md AC-15; spec/tests.md TC-239"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-021
    type: references
---

# Spec review of FR-021-AC-15 and TC-239 (engineering-assurance PR 197)

## Summary

Ticket: EA-32. Reviewed the new FR-021-AC-15 row, its coverage row and the new TC-239 row. `quire validate --scope .` passes for both changed files (exit 0). Its EARS and passive-voice warnings are on lines 86, 90 and 100, which this PR does not touch. AC-15 can fail: it names the bound for each comparator, covers both reference kinds, holds and not-holds, refusals equal to those of `holds_on_interval`, and that `holds_on_interval` returns exactly the reported verdict. It agrees with AC-14 and the FR-021 Behavior statement on `interval_level` (lower bound for `gt`/`ge`, upper for `lt`/`le`). The sub-analyses were scoped to one new AC row: integrity (ids unique, coverage row and TC row consistent) and EARS/criterion strength (judged inline; Jev was not used).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | FR-021's Outputs section still says rule evaluation yields "whether the rule holds, or a typed refusal". No Behavior statement says the evaluation reports the judged bound and its value. AC-15 therefore verifies an output that the FR body never states. Fix: extend the Outputs bullet (and, optionally, the `interval_level` Behavior bullet) to say interval evaluation also reports which bound was judged and its value. | spec/functional/FR-021-measurement-plan-decision-rule.md:49-50; spec/functional/FR-021-measurement-plan-decision-rule.md:218 | missing-requirement |

## Verdict

Mergeable. FND-001 is low: the AC is sound and testable, but the FR body should state the output it verifies.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit 9b11ff4 adds an Outputs bullet for interval evaluation (the verdict, which bound was judged and its value, or the same refusals) and a Behavior SHALL saying that evaluating a rule with `interval_level` reports the bound and its value. `quire validate` exits 0. Its only warnings are the pre-existing ones, shifted to lines 89, 93 and 105. |
