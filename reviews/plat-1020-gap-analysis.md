---
id: SR-16402
title: "Gap analysis of answer_families implementation (engineering-assurance PR 200)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/engineering-assurance@9057c54; FR-021-AC-16..AC-19; tests/measurement.rs; tests/python_port/answer_families.rs; src/measurement.rs; engineering_assurance/schemas/measurement-plan-frontmatter.schema.json"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-021
    type: references
---

# Gap analysis of answer_families implementation (engineering-assurance PR 200)

## Summary

Ticket: PLAT-1020. Plan completion: not assessed. `quoin matrix --repo . --json` (quoin 0.29.1) reports FR-021-AC-16..AC-19 as `tagged`, with binders in `tests/python_port/answer_families.rs` (AC-16 and AC-18) and `tests/measurement.rs` (AC-17 and AC-19, including `tc_142` for the measurement-only consumer). No production code in the diff lacks an owning requirement, and there are no stubs.

Mutation checks were run in a scratch worktree and each was reverted:
- Removing the schema `else` makes `answer_families_are_refused_with_any_other_baseline_or_a_threshold_rule` fail.
- Removing `then.required` makes `answer_families_are_required_with_a_constant_predictor_and_a_valid_list` fail.
- Removing the empty-family refusal makes `tc_243` fail.
- Storing the families as an order-preserving `Vec` makes `tc_243` and `tc_241` fail.
- Making the `changed_members` arm return `false` makes `tc_241` and `tc_141` fail.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The TC-242 onboarding-checklist half cannot fail. In `skeleton_and_onboarding_state_the_answer_families_rule`, the `conditionalRequired` assertion is `contains({when: constant-predictor, required: [statistical_design.answer_families]}) \|\| text.contains("answer_families")`. The checklist always contains that path through its `arrays` entry, so the right-hand side is always true, and the line before already asserts it. Nothing asserts the refused-otherwise constraint (`otherwise (... constant-predictor does not hold)` / `statistical_design.answer_families must be absent`). Shown by mutation: with both the schema `then.required` and the `else` removed, the generated checklist no longer states the rule, yet this test still passes. Drop the `\|\| text.contains(...)` and assert the `conditionalConstraints` entry for the `else` branch. | tests/python_port/answer_families.rs:104-116 |

## Verdict

FR-021-AC-16, AC-17 and AC-19 are backed by tests that fail under mutation. FR-021-AC-18 is backed only partly: the skeleton and onboarding-skill text are asserted, but the generated checklist's required and refused rule is not, although the checklist does state it today (verified from `onboard-report --json` output).

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit 5be346c removes the `\|\| text.contains("answer_families")` fallback, asserts the `conditionalRequired` entry `{when: statistical_design.decision_rule.baseline = constant-predictor, required: [statistical_design.answer_families]}` on its own, and asserts a `conditionalConstraints` entry containing `statistical_design.answer_families must be absent`. Re-run mutation at c65c784: with the schema `else` removed the test fails; restored, it passes. |
