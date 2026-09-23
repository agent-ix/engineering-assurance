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
    `baseline` (exactly one of `constant-predictor`, `prior-collection`,
    `best-seen`, or `external-reference`) with an optional numeric `margin`.
- For the consistency checks: a validated rule with the plan's `objective` or
  `estimator`.
- For rule evaluation: a validated rule, the estimate, and, for a baseline
  rule, the caller-computed baseline value.
- For the definition-change check: the plan's frontmatter before and after an
  edit, as in [FR-020](./FR-020-measurement-plan-objective.md).

## Outputs

- A schema validation result for the frontmatter document.
- A typed Rust `Estimator` or `DecisionRule` value, or a typed refusal naming
  why the rule is invalid.
- For the consistency checks: success, or a typed refusal naming the
  disagreeing direction and comparator, or the disallowed estimator.
- For rule evaluation: whether the rule holds, or a typed refusal naming the
  missing, unexpected, or non-finite input or reference.
- For the definition-change check: either no finding, or one typed finding
  naming which of `objective`, `statistical_design.estimator`, and
  `statistical_design.decision_rule` changed.

## Behavior

- The decision rule SHALL be the only part of a MeasurementPlan that is
  evaluated. `objective.bound` is the goal the metric should reach; it is
  informational and never evaluated.
- `estimator: proportion` SHALL mean matching items divided by examined items;
  `count` the number of matching items; `mean` the arithmetic mean of per-item
  or per-repetition values, including a weighted mean whose weights the plan's
  body states; `median` the middle per-item value; and `ratio` one total
  divided by another.
- A decision rule SHALL hold when `estimate <comparator> reference`, where
  `gt`, `ge`, `lt`, `le`, and `eq` are `>`, `>=`, `<`, `<=`, and exact `==`.
  For a `mean` or `ratio` estimate, `eq` is exact floating-point equality.
- The reference SHALL be the `threshold` when stated. For a baseline rule the
  reference SHALL be the baseline value plus `margin` for `gt` and `ge`, the
  baseline value minus `margin` for `lt` and `le`, and the baseline value
  itself for `eq`. An absent `margin` means 0.
- `margin` SHALL be in the metric's own units and signed in the direction of
  improvement: a positive margin requires the estimate to beat the baseline by
  at least that much, and a negative margin allows a regression of up to that
  much. For example, `{ comparator: gt, baseline: constant-predictor,
  margin: 0.05 }` needs an agreement rate more than 0.05 above the constant
  predictor, and, for a lower-is-better latency,
  `{ comparator: le, baseline: prior-collection, margin: 20 }` needs 180 ms or
  less against a 200 ms prior collection.
- `baseline: constant-predictor` SHALL mean, for each answer family, the
  agreement rate of the highest-scoring single constant answer on the corpus
  at evaluation time, combined across families as the size-weighted mean
  (total best-constant agreements over total items).
- `baseline: prior-collection` SHALL mean the plan's metric in the collection
  this result is compared against under the same `definition_version`.
- An accepted collection SHALL mean one the measurement intake admitted rather
  than refused.
- `baseline: best-seen` SHALL mean, over every accepted collection of the
  plan's metric under the same `definition_version`, the maximum value when
  `comparator` is `gt` or `ge`, and the minimum when `comparator` is `lt` or
  `le`.
- `baseline: external-reference` SHALL mean a per-dimension reference value
  the calling checker resolves from a source outside the plan at evaluation
  time -- neither a fixed number written into the plan, nor the plan's metric
  in a prior collection, nor a corpus-computed constant-predictor rate, nor a
  maximum or minimum over prior collections. This repository does not resolve
  it, exactly as it does not compute any other baseline value: the calling
  checker supplies it the same way it supplies every other baseline value.
- The schema and the Rust `DecisionRule` type SHALL accept
  `baseline: external-reference` with every `estimator`, carrying no
  estimator restriction.
- The schema and the Rust `DecisionRule` type SHALL accept `comparator: eq`
  with `baseline: external-reference`, unlike `baseline: best-seen`, because
  its value does not depend on `comparator`'s direction.
- The schema and the Rust `DecisionRule` type SHALL refuse `comparator: eq`
  with `baseline: best-seen`, and `comparator: eq` with a `margin`.
- The schema and the Rust `DecisionRule` type SHALL refuse
  `baseline: constant-predictor` unless `estimator` is `proportion`.
- While `objective` is present, the direction check SHALL refuse a
  comparator that disagrees with `objective.direction`, in the schema and in
  the Rust `DecisionRule`.
- Direction `higher` agrees with `gt` or `ge`, `lower` with `lt` or `le`,
  `zero` with `eq` or with `le` against `threshold: 0`, and `target` with any
  comparator.
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
  `threshold` or `margin`, and any other key.
- The Rust `DecisionRule` type SHALL refuse a non-finite `threshold` or
  `margin`. JSON Schema has no finiteness keyword, and a YAML `.inf` or `.nan`
  value satisfies `type: number`, so the frontmatter schema accepts a
  non-finite `threshold` or `margin`; finiteness is enforced only by the Rust
  type, at construction and at deserialization.
- Rule evaluation SHALL refuse a non-finite estimate or baseline value, and a
  reference that overflows to a non-finite value. `Comparator::holds` on its
  own does not guard NaN: every comparator is false when either side is NaN.
- The schema and the Rust `Estimator` type SHALL refuse any estimator outside
  the closed set, including free prose.
- `population`, `sampling`, `error_model`, and `uncertainty` SHALL remain
  non-empty prose strings: rule evaluation reads none of them.
- The schema's `estimator`, `comparator`, and `baseline` values SHALL be
  exactly the wire names of the Rust `Estimator`, `Comparator`, and `Baseline`
  enums.
- The plan's measurement definition SHALL include `objective`,
  `statistical_design.estimator`, and `statistical_design.decision_rule`;
  [FR-024](./FR-024-measurement-plan-protected-apparatus.md) adds
  `protected_apparatus`.
- When any of them is added, removed, or changed while `definition_version`
  stays the same, the definition-change check SHALL return one typed finding
  naming each changed member. The same edit with a `definition_version`
  change SHALL produce no finding.
- A decision rule edited after results were seen therefore needs a new
  `definition_version`, and Quoin's measurement intake refuses to compare
  results across definition versions, so the edited rule never re-judges the
  earlier results. `preregistration.bar_digest` is unchanged by this
  requirement.
- The onboarding checklist SHALL list the estimator, comparator, and baseline
  sets, the one-of choice between `threshold` and `baseline`, `margin`'s
  dependency on `baseline`, the direction and estimator consistency rules,
  and the refused `eq` combinations.
- `Estimator`, `Comparator`, `Baseline`, `DecisionRule`, and the
  definition-change check SHALL be reachable through the `measurement` Cargo
  feature.

## Error Conditions

An unknown estimator, comparator, or baseline, a rule with no or two
references, a `margin` without `baseline` or with `eq`, `eq` against
`best-seen`, `constant-predictor` without `estimator: proportion`, a
comparator that disagrees with the objective's direction, a non-numeric or
non-finite `threshold` or `margin`, or an extra rule key fails validation or
construction and is never read as a valid rule. Evaluating a baseline rule
without a baseline value, a threshold rule with one, a non-finite estimate or
baseline value, or a reference that overflows is refused rather than
answered. An edit to the estimator or rule without a `definition_version`
change produces a finding rather than being accepted silently.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-021-AC-1 | The MeasurementPlan frontmatter schema accepts a decision rule with every comparator against a `threshold`, and every baseline with and without `margin`; it rejects an unknown or missing comparator, neither or both of `threshold` and `baseline`, `margin` with `threshold` or with `eq`, a non-numeric `margin`, an unknown baseline, `eq` against `best-seen`, a non-numeric `threshold`, an extra rule key, and a prose rule; it accepts a non-finite `threshold`; and it requires `metric` when `statistical_design` is present. | Test (TC-144) |
| FR-021-AC-2 | The schema accepts each of the five estimators and rejects any other value; `population`, `sampling`, `error_model`, and `uncertainty` remain prose; the MeasurementPlan skeleton carries a valid structured estimator and decision rule, and an objective `bound` distinct from the rule's `threshold`. | Test (TC-145) |
| FR-021-AC-3 | The Rust `DecisionRule` accepts threshold and baseline rules and refuses neither or both references, `margin` without `baseline` or with `eq`, `eq` against `best-seen`, and a non-finite `threshold` or `margin` with distinct typed errors, through construction and deserialization; deserialization refuses unknown comparators, baselines, estimators, a non-numeric `margin`, and extra keys; a valid rule round-trips. | Test (TC-146) |
| FR-021-AC-4 | The schema's `estimator`, `comparator`, and `baseline` enums equal the Rust `Estimator`, `Comparator`, and `Baseline` wire-name sets, in order, and each `ALL` constant covers every variant. | Test (TC-147) |
| FR-021-AC-5 | Each comparator holds exactly for the estimates below, at, or above the reference its symbol names, and none holds against NaN; a baseline rule compares against the baseline value moved by its margin in the direction of improvement, for higher- and lower-is-better rules and for positive and negative margins; a missing or unexpected baseline value, a non-finite estimate or baseline value, and an overflowing reference are typed refusals. | Test (TC-148) |
| FR-021-AC-6 | The onboarding checklist lists the estimator, comparator, and baseline sets, the one-of choice between `threshold` and `baseline`, `margin`'s dependency on `baseline`, `metric`'s dependency on `statistical_design`, the direction and estimator consistency rules, and the refused `eq` combinations, with no warning. | Test (TC-149) |
| FR-021-AC-7 | A minimal consumer with only the `measurement` feature reaches `Estimator`, `Comparator`, `Baseline`, `DecisionRule`, and the definition-change check, and resolves no `serde_json`. | Test (TC-142) |
| FR-021-AC-8 | Given two plan definitions with an equal `definition_version`, an added, removed, or changed estimator or decision rule yields one typed finding naming each changed member; the same edit with a different `definition_version`, and an unchanged definition, yield no finding. | Test (TC-141) |
| FR-021-AC-9 | The schema and the Rust `DecisionRule` accept a comparator that agrees with the objective's direction and refuse one that disagrees, with a typed error naming both; they accept `constant-predictor` only with `estimator: proportion`, with a typed error naming the estimator. | Test (TC-144, TC-146) |
| FR-021-AC-10 | The schema and the Rust `DecisionRule` accept `baseline: external-reference` with every estimator and, unlike `best-seen`, with `comparator: eq`; the Rust `Baseline` type round-trips `external-reference` through construction, serialization, and deserialization. | Test (TC-171) |

## Dependencies

- **Upstream**: [FR-020](./FR-020-measurement-plan-objective.md) for the
  `measurement` feature and the MeasurementPlan definition types.
- **Downstream**: Quoin's measurement checker, which evaluates the rule
  against collected results (Linear PLAT-961).
