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

A current MeasurementPlan's `statistical_design.estimator` SHALL be one value
from a closed vocabulary, and its `statistical_design.decision_rule` SHALL be a
structured rule a checker evaluates mechanically against the plan's `metric`.
Retired plans authored under the prose contract remain readable as
history; they cannot govern new measurement collections.

## Inputs

- One MeasurementPlan frontmatter document whose optional `statistical_design`
  carries:
  - `estimator`: exactly one of `proportion`, `count`, `mean`, `median`, or
    `ratio`;
  - `decision_rule`: an object with `comparator` (exactly one of `gt`, `ge`,
    `lt`, `le`, or `eq`) and exactly one reference: a numeric `threshold`, or a
    `baseline` (exactly one of `constant-predictor`, `prior-collection`,
    `best-seen`, or `external-reference`) with an optional numeric `margin`;
    either form may also carry an optional `interval_level`, a number
    strictly between 0 and 1;
  - `answer_families`: a non-empty list of unique, non-empty strings naming
    the answer-space families of the population. It SHALL be present when
    `decision_rule.baseline` is `constant-predictor` and absent otherwise.
- For the consistency checks: a validated rule with the plan's `objective` or
  `estimator`.
- For rule evaluation: a validated rule, the estimate, and, for a baseline
  rule, the caller-computed baseline value; for a rule with `interval_level`,
  the observation's interval (`lower`, `upper`, `level`, `method`).
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
- For interval evaluation: whether the rule holds at the unfavourable bound,
  together with which bound (lower or upper) was judged and its value, or the
  same typed refusals.
- For the definition-change check: either no finding, or one typed finding
  naming which of `objective`, `statistical_design.estimator`,
  `statistical_design.decision_rule`, and `statistical_design.answer_families`
  changed.

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
- `margin` SHALL be a fraction of the baseline value's magnitude when
  `margin_mode` is `relative`, and in the metric's own units when `margin_mode`
  is `absolute` or absent. A relative margin moves the baseline value by
  `margin` times its absolute value, in the same direction of improvement, so
  `{ comparator: le, baseline: prior-collection, margin: -0.05, margin_mode:
  relative }` tolerates a 5% regression whatever the scale of the baseline.
  `margin_mode` SHALL be allowed only with `margin`, and never with `eq`.
- `interval_level`, when present, SHALL make the rule hold at the unfavourable
  end of the observation's stated interval instead of at its point estimate:
  the lower bound for `gt` and `ge`, the upper bound for `lt` and `le`. It
  SHALL be a finite number strictly between 0 and 1, SHALL NOT be stated with
  `eq`, and, being inside `decision_rule`, SHALL be part of the measurement
  definition (a reader that does not know it refuses the plan rather than
  judging the point estimate). An absent `interval_level` means the point
  estimate is used. `statistical_design.uncertainty` stays prose.
- Evaluating a rule with `interval_level` SHALL report, with the verdict, which
  bound (lower or upper) it was judged at and that bound's value.
- An interval (`lower`, `upper`, `level`, `method`) SHALL have finite bounds
  with `lower <= upper`, a `level` strictly between 0 and 1, and a non-empty
  `method` that no checker reads. A rule with `interval_level` SHALL refuse an
  observation with no interval, an interval at a lower level than the rule's,
  and an estimate outside its interval; an interval at an equal or higher level
  SHALL be accepted, and the point-estimate evaluation SHALL refuse such a
  rule rather than judge the point estimate. The observation's `interval`
  field is owned by Quoin, not this repository.
- `baseline: constant-predictor` SHALL mean, for each answer family, the
  agreement rate of the highest-scoring single constant answer on the corpus
  at evaluation time, combined across families as the size-weighted mean
  (total best-constant agreements over total items). The families SHALL be
  the plan's `statistical_design.answer_families`, never inferred from the
  population prose or from whatever grouping a producer wrote.
- The schema SHALL require `statistical_design.answer_families` when
  `decision_rule.baseline` is `constant-predictor`.
- The schema SHALL refuse `statistical_design.answer_families` when the
  decision rule has any other baseline or a threshold, because a family list
  that no baseline reads is a claim nothing checks.
- The schema SHALL refuse an `answer_families` list that is empty, repeats an
  entry, or holds an empty or non-string entry.
- The plan author SHALL name each answer-space family of the population once
  in `answer_families`, and SHALL NOT infer the list from the Population
  prose.
- The Rust `AnswerFamilies` type SHALL be a non-empty set of distinct,
  non-empty `AnswerFamily` strings, built through construction and
  deserialization, which SHALL refuse an empty list with
  `AnswerFamiliesError::Empty`, a repeated entry with
  `AnswerFamiliesError::Duplicate` naming it, and an empty-string entry with
  `AnswerFamiliesError::EmptyFamily`. It SHALL compare and serialize as a
  set: reordering the list is not a change, and entries serialize sorted.
- Forward note (not a requirement on this repository): Quoin's measurement
  checker (`quoin-measurement`) is to require the set of distinct `family`
  dimension values on the producer's item rows to equal `answer_families`
  exactly, and to return a typed inconclusive reason naming the missing and
  the undeclared families when it does not, so a producer that merges two
  declared families is caught. That comparison and its reason are specified
  and tracked in the Quoin follow-up to Linear PLAT-1020; this repository
  owns only the declared list.
- Changing `answer_families` changes the baseline, so the definition-change
  check SHALL treat it as a member of the measurement definition (the Rust
  `DefinitionMember::AnswerFamilies`, path
  `statistical_design.answer_families`) and compare it as a set.
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
  `statistical_design.estimator`, `statistical_design.decision_rule`, and
  `statistical_design.answer_families`;
  [FR-024](./FR-024-measurement-plan-protected-apparatus.md) adds
  `protected_apparatus`.
- When any of them is added, removed, or changed while `definition_version`
  stays the same, the definition-change check SHALL return one typed finding
  naming each changed member. The same edit with a `definition_version`
  change SHALL produce no finding.
- A decision rule edited after results were seen therefore needs a new
  `definition_version`, and Quoin's measurement intake refuses to compare
  results across definition versions, so the edited rule never re-judges the
  earlier results.
- The onboarding checklist SHALL list the estimator, comparator, and baseline
  sets, the one-of choice between `threshold` and `baseline`, `margin`'s
  dependency on `baseline`, the direction and estimator consistency rules,
  the refused `eq` combinations, and that `answer_families` is required with
  `baseline: constant-predictor` and refused otherwise.
- `Estimator`, `Comparator`, `Baseline`, `DecisionRule`, `AnswerFamily`,
  `AnswerFamilies`, `AnswerFamiliesError`, and the definition-change check
  SHALL be reachable through the `measurement` Cargo feature.

## Error Conditions

An unknown estimator, comparator, or baseline, a rule with no or two
references, a `margin` without `baseline` or with `eq`, `eq` against
`best-seen`, `constant-predictor` without `estimator: proportion`,
`constant-predictor` without `answer_families`, `answer_families` with any
other reference, an empty or duplicate or empty-string family, a
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
| FR-021-AC-1 | For non-retired plans, the MeasurementPlan frontmatter schema accepts a decision rule with every comparator against a `threshold`, and every baseline with and without `margin`; it rejects an unknown or missing comparator, neither or both of `threshold` and `baseline`, `margin` with `threshold` or with `eq`, a non-numeric `margin`, an unknown baseline, `eq` against `best-seen`, a non-numeric `threshold`, an extra rule key, and a prose rule; it accepts a non-finite `threshold`; and it requires `metric` when `statistical_design` is present. | Test (TC-144) |
| FR-021-AC-2 | For non-retired plans, the schema accepts each of the five estimators and rejects any other value; `population`, `sampling`, `error_model`, and `uncertainty` remain prose; the MeasurementPlan skeleton carries a valid structured estimator and decision rule, and an objective `bound` distinct from the rule's `threshold`. | Test (TC-145) |
| FR-021-AC-3 | The Rust `DecisionRule` accepts threshold and baseline rules and refuses neither or both references, `margin` without `baseline` or with `eq`, `eq` against `best-seen`, and a non-finite `threshold` or `margin` with distinct typed errors, through construction and deserialization; deserialization refuses unknown comparators, baselines, estimators, a non-numeric `margin`, and extra keys; a valid rule round-trips. | Test (TC-146) |
| FR-021-AC-4 | The schema's `estimator`, `comparator`, and `baseline` enums equal the Rust `Estimator`, `Comparator`, and `Baseline` wire-name sets, in order, and each `ALL` constant covers every variant. | Test (TC-147) |
| FR-021-AC-5 | Each comparator holds exactly for the estimates below, at, or above the reference its symbol names, and none holds against NaN; a baseline rule compares against the baseline value moved by its margin in the direction of improvement, for higher- and lower-is-better rules and for positive and negative margins; a missing or unexpected baseline value, a non-finite estimate or baseline value, and an overflowing reference are typed refusals. | Test (TC-148) |
| FR-021-AC-6 | The onboarding checklist lists the estimator, comparator, and baseline sets, the one-of choice between `threshold` and `baseline`, `margin`'s dependency on `baseline`, `metric`'s dependency on `statistical_design`, the direction and estimator consistency rules, the refused `eq` combinations, and the `interval_level` refusal with `eq`, and notes that an observation MAY carry a Quoin-validated `interval` object, with no warning. | Test (TC-149) |
| FR-021-AC-7 | A minimal consumer with only the `measurement` feature reaches `Estimator`, `Comparator`, `Baseline`, `DecisionRule`, and the definition-change check, and resolves no `serde_json`. | Test (TC-142) |
| FR-021-AC-8 | Given two plan definitions with an equal `definition_version`, an added, removed, or changed estimator or decision rule yields one typed finding naming each changed member; the same edit with a different `definition_version`, and an unchanged definition, yield no finding. | Test (TC-141) |
| FR-021-AC-9 | The schema and the Rust `DecisionRule` accept a comparator that agrees with the objective's direction and refuse one that disagrees, with a typed error naming both; they accept `constant-predictor` only with `estimator: proportion`, with a typed error naming the estimator. | Test (TC-144, TC-146) |
| FR-021-AC-10 | The schema and the Rust `DecisionRule` accept `baseline: external-reference` with every estimator and, unlike `best-seen`, with `comparator: eq`, and refuse it with `eq` plus a `margin` and with a comparator that disagrees with the objective's direction, with the same typed errors as every other baseline; evaluating it without a supplied value is a typed refusal naming `external-reference`, and a supplied value is moved by the margin in the direction of improvement; the Rust `Baseline` type round-trips `external-reference` through construction, serialization, and deserialization. | Test (TC-171) |
| FR-021-AC-11 | A retired plan with the exact prose `statistical_design` shape validates without a fabricated numeric rule; the same prose is refused for proposed or active plans. Empty, missing, or extra legacy design fields are refused, a retired current-shape plan remains valid, and a legacy prose plan cannot mix in fields introduced later. | Test (TC-172) |
| FR-021-AC-12 | The schema and the Rust `DecisionRule` accept `margin_mode` of `absolute` or `relative` only with a `margin`, and refuse it with a threshold, without a margin, with `eq`, or with any other value; an absent mode is absolute and an absolute rule does not serialize the key; a relative margin moves the baseline value by the margin times its absolute value in the direction of improvement, so one rule tolerates the same fraction of baselines that differ in scale, including negative baselines; the schema's `margin_mode` enum equals the Rust `MarginMode` wire names. | Test (TC-188) |
| FR-021-AC-13 | The schema and the Rust `DecisionRule` accept an `interval_level` strictly between 0 and 1 with a threshold or baseline rule and every comparator but `eq`, and refuse it with `eq`, at 0 or 1, outside that range, non-numeric, or (Rust) non-finite, with distinct typed errors, through construction and deserialization; an absent level is not serialized and a stated level round-trips; the Rust `Interval` and `ConfidenceLevel` types accept a finite ordered interval at a level strictly between 0 and 1 with a non-empty method and refuse each violation with a typed `IntervalError`, and round-trip; the `interval_level` key is inside `decision_rule`, so an unknown-key reader refuses it and editing it without a `definition_version` bump yields a finding. | Test (TC-192) |
| FR-021-AC-14 | `holds_on_interval` judges a rule at the interval's lower bound for `gt` and `ge` and its upper bound for `lt` and `le`, for higher- and lower-is-better rules and for threshold and baseline (including relative-margin) references, so a point estimate that passes but whose unfavourable bound fails is refused; an interval at a level at or above the rule's is accepted; a missing interval, a lower level, an estimate outside the interval, a rule without `interval_level`, and non-finite inputs are typed refusals; and the point-form `holds` returns a typed `IntervalRequired` error for a rule with `interval_level`. | Test (TC-193) |
| FR-021-AC-15 | `holds_on_interval_reporting` returns, with the verdict of `holds_on_interval`, the bound it judged at and that bound's value: the lower bound for `gt` and `ge`, the upper bound for `lt` and `le`, for threshold and baseline (including relative-margin) references, whether the rule holds or not; it refuses with the same typed errors as `holds_on_interval`, and `holds_on_interval` returns exactly its verdict. | Test (TC-239) |
| FR-021-AC-16 | The MeasurementPlan frontmatter schema requires `statistical_design.answer_families` when `decision_rule.baseline` is `constant-predictor` and refuses a plan without it; it accepts a non-empty list of unique non-empty strings and refuses an empty list, a duplicate entry, an empty-string entry, and a non-string entry; it refuses `answer_families` with any other baseline and with a threshold rule. | Test (TC-240) |
| FR-021-AC-17 | Adding, removing, or changing a family in `statistical_design.answer_families`, or adding or removing the list, while `definition_version` stays the same yields one definition-change finding naming `statistical_design.answer_families`; reordering the same families yields no finding (the list is compared as a set); the same edit with a changed `definition_version` yields none. | Test (TC-241) |
| FR-021-AC-18 | The MeasurementPlan skeleton's constant-predictor guidance and worked example show the `answer_families` declaration and say the families come from it rather than from the Population prose, and the onboarding checklist states the required-with-constant-predictor, refused-otherwise rule and that a producer's item-row families must equal the list. | Test (TC-242) |
| FR-021-AC-19 | The Rust `AnswerFamilies` accepts a non-empty set of distinct non-empty `AnswerFamily` strings and refuses an empty list, a repeated entry, and an empty-string entry with distinct typed `AnswerFamiliesError`s, through construction and deserialization; two lists of the same families in a different order are equal and serialize identically in sorted order; `DefinitionMember::AnswerFamilies` has path `statistical_design.answer_families`; and a minimal consumer with only the `measurement` feature reaches these types. | Test (TC-243) |

## Dependencies

- **Upstream**: [FR-020](./FR-020-measurement-plan-objective.md) for the
  `measurement` feature and the MeasurementPlan definition types.
- **Downstream**: Quoin's measurement checker, which evaluates the rule
  against collected results (Linear PLAT-961).
