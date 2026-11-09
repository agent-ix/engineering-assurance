---
id: SR-16308
title: "gap-analysis of the reported interval bound (engineering-assurance PR 197)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@fcdd162; spec/functional/FR-021-measurement-plan-decision-rule.md AC-15; spec/tests.md TC-239; src/measurement.rs; tests/measurement.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-021
    type: references
---

# Gap analysis of the reported interval bound (engineering-assurance PR 197)

## Summary

Ticket: EA-32. This analysis is incremental and planless, so plan completion was not assessed. `quoin matrix --repo .` (quoin 0.29.1, quire 0.36.2 / engine 0.50.2) at head fcdd162 shows FR-021-AC-15 as `tagged`, with one binder, `tc_239_the_judgement_reports_the_bound_it_was_judged_at` (tests/measurement.rs:2101), through `#[trace("TC-239", "FR-021-AC-15")]`. FR-021-AC-14 keeps its two TC-193 binders. TC-239 and FR-021-AC-15 are unique in the repository, and no other open PR claims them. All new production code (`IntervalBound`, `IntervalJudgement`, `holds_on_interval_reporting`) is owned by FR-021-AC-15. There are no stubs and no coverage inflation: the test asserts exact `(holds, bound, value)` tuples and exact typed refusals, and a mutation of the bound kind or value fails it (see SR-16307). The TC-239 wording overstatement is recorded in SR-16307 FND-001, and the FR-body gap in SR-16309 FND-001.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

Clean. The new criterion is tagged by a test that asserts the returned bound and can fail.
