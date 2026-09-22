---
id: FR-021
title: "Declare a MeasurementPlan decision rule and estimator as closed data"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-005"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-020"
    type: "requires"
---

# FR-021: Declare a MeasurementPlan decision rule and estimator as closed data

## Description

A MeasurementPlan's `statistical_design.estimator` SHALL be one value from a
closed vocabulary, and its `statistical_design.decision_rule` SHALL be a
structured rule a checker evaluates mechanically against the plan's `metric`.

## Inputs

- One MeasurementPlan frontmatter document whose optional `statistical_design`
  carries:
  - `estimator`: exactly one of `proportion`, `count`, `mean`, `median`, or
    `ratio`;
  - `decision_rule`: an object with `comparator` (exactly one of `gt`, `ge`,
    `lt`, `le`, or `eq`) and exactly one reference: a numeric `threshold`, or a
    `baseline` (exactly one of `constant-predictor`, `prior-collection`, or
    `best-seen`) with an optional numeric `margin`.
- For rule evaluation: a validated rule, the estimate, and, for a baseline
  rule, the caller-computed baseline value.

## Outputs

- A schema validation result for the frontmatter document.
- A typed Rust `Estimator` or `DecisionRule` value, or a typed refusal naming
  why the rule is invalid.
- For rule evaluation: whether the rule holds, or a typed refusal naming the
  missing, unexpected, or non-finite input.

## Behavior

- `estimator: proportion` SHALL mean matching items divided by examined items;
  `count` the number of matching items; `mean` the arithmetic mean of per-item
  or per-repetition values, including a weighted mean the plan's Measure
  Definition states; `median` the middle per-item value; and `ratio` one total
  divided by another.
- A decision rule SHALL hold when `estimate <comparator> reference`, where
  `gt`, `ge`, `lt`, `le`, and `eq` are `>`, `>=`, `<`, `<=`, and exact `==`.
- The reference SHALL be the `threshold` when stated, and otherwise the
  baseline's value plus `margin`, with an absent `margin` meaning 0. `margin`
  is signed.
- `baseline: constant-predictor` SHALL mean the highest agreement rate any
  single constant answer per answer family scores on the corpus at evaluation
  time.
- `baseline: prior-collection` SHALL mean the plan's metric in the collection
  this result is compared against under the same `definition_version`.
- `baseline: best-seen` SHALL mean the maximum accepted value of the plan's
  metric under the same `definition_version` when `comparator` is `gt` or
  `ge`, and the minimum when `comparator` is `lt` or `le`.
- The schema and the Rust `DecisionRule` type SHALL refuse `comparator: eq`
  with `baseline: best-seen`.
- The calling checker SHALL supply the baseline value; this repository does
  not compute it.
- The rule SHALL apply to the plan's own `metric`.
- The schema SHALL require `metric` whenever `statistical_design` is present.
- The calling checker SHALL compute the estimate over all `repetitions`
  together and apply the rule once to that estimate.
- When `minimum_population` is set, the calling checker SHALL refuse a result
  over a smaller population before it evaluates the rule.
- The schema SHALL NOT accept `metric`, `repetitions`, or
  `minimum_population` inside the rule.
- The schema and the Rust `DecisionRule` type SHALL refuse an unknown
  comparator or baseline, a missing comparator, a rule with neither or both
  of `threshold` and `baseline`, a `margin` without `baseline`, a non-numeric
  `threshold` or `margin`, and any other key. The Rust type SHALL also refuse
  a non-finite `threshold` or `margin`.
- The schema and the Rust `Estimator` type SHALL refuse any estimator outside
  the closed set, including free prose.
- `population`, `sampling`, `error_model`, and `uncertainty` SHALL remain
  non-empty prose strings: rule evaluation reads none of them.
- The schema's `estimator`, `comparator`, and `baseline` values SHALL be
  exactly the wire names of the Rust `Estimator`, `Comparator`, and `Baseline`
  enums.
- The onboarding checklist SHALL list the estimator, comparator, and baseline
  sets, the one-of choice between `threshold` and `baseline`, `margin`'s
  dependency on `baseline`, and the refused `eq` against `best-seen`.
- `Estimator`, `Comparator`, `Baseline`, and `DecisionRule` SHALL be reachable
  through the `measurement` Cargo feature.

## Error Conditions

An unknown estimator, comparator, or baseline, a rule with no or two
references, a `margin` without `baseline`, `eq` against `best-seen`, a
non-numeric or non-finite `threshold` or `margin`, or an extra rule key fails validation or construction
and is never read as a valid rule. Evaluating a baseline rule without a
baseline value, a threshold rule with one, or a non-finite estimate or
baseline value is refused rather than answered.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-021-AC-1 | The MeasurementPlan frontmatter schema accepts a decision rule with every comparator against a `threshold`, and every baseline with and without `margin`; it rejects an unknown or missing comparator, neither or both of `threshold` and `baseline`, `margin` with `threshold`, an unknown baseline, `eq` against `best-seen`, a non-numeric `threshold`, an extra rule key, and a prose rule; and it requires `metric` when `statistical_design` is present. | Test (TC-144) |
| FR-021-AC-2 | The schema accepts each of the five estimators and rejects any other value; `population`, `sampling`, `error_model`, and `uncertainty` remain prose; the MeasurementPlan skeleton carries a valid structured estimator and decision rule. | Test (TC-145) |
| FR-021-AC-3 | The Rust `DecisionRule` accepts threshold and baseline rules and refuses neither or both references, `margin` without `baseline`, `eq` against `best-seen`, and a non-finite `threshold` or `margin` with distinct typed errors, through construction and deserialization; deserialization refuses unknown comparators, baselines, estimators, and extra keys; a valid rule round-trips. | Test (TC-146) |
| FR-021-AC-4 | The schema's `estimator`, `comparator`, and `baseline` enums equal the Rust `Estimator`, `Comparator`, and `Baseline` wire-name sets, in order, and each `ALL` constant covers every variant. | Test (TC-147) |
| FR-021-AC-5 | Each comparator holds exactly for the estimates below, at, or above the reference its symbol names; a baseline rule compares against the baseline value plus its signed margin; a missing or unexpected baseline value and a non-finite estimate or baseline value are typed refusals. | Test (TC-148) |
| FR-021-AC-6 | The onboarding checklist lists the estimator, comparator, and baseline sets, the one-of choice between `threshold` and `baseline`, `margin`'s dependency on `baseline`, `metric`'s dependency on `statistical_design`, and the refused `eq` against `best-seen`, with no warning. | Test (TC-149) |
| FR-021-AC-7 | A minimal consumer with only the `measurement` feature reaches `Estimator`, `Comparator`, `Baseline`, and `DecisionRule` and resolves no `serde_json`. | Test (TC-142) |

## Dependencies

- **Upstream**: [FR-020](./FR-020-measurement-plan-objective.md) for the
  `measurement` feature and the MeasurementPlan definition types.
- **Downstream**: Quoin's measurement checker, which evaluates the rule
  against collected results (Linear PLAT-961).
