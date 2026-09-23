---
id: FR-026
title: "Declare steering fields on a MeasurementPlan objective"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-005"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-020"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-021"
    type: "requires"
---

# FR-026: Declare steering fields on a MeasurementPlan objective

## Description

The MeasurementPlan `objective` block SHALL accept three optional steering
fields -- `weight`, `value_half_life`, and `budget` -- that state how much an
objective matters, how quickly its value decays, and how it should be
normalized against other objectives' cost. These fields SHALL be advisory
only. They SHALL NOT gate anything. They SHALL NOT feed
`statistical_design.decision_rule`, which remains the only part of a plan
that is evaluated (FR-021).

## Inputs

- One MeasurementPlan frontmatter document's `objective` block, optionally
  carrying, alongside `direction` and `bound` (FR-020):
  - `weight`: an optional finite, non-negative number;
  - `value_half_life`: an optional finite, positive number;
  - `budget`: an optional finite, non-negative number.
- For the definition-change check: the plan's frontmatter before and after an
  edit, as in [FR-020](./FR-020-measurement-plan-objective.md).

## Outputs

- A schema validation result for the frontmatter document.
- A typed Rust `Objective` value carrying the steering fields, or a typed
  refusal naming why one is invalid.
- For the definition-change check: a result computed only from `direction`
  and `bound`, unaffected by the steering fields (FR-020's existing output).

## Behavior

- `weight` SHALL state this objective's relative value against the project's
  other objectives, in units the plan's body states. It has no fixed scale;
  it is meaningful only relative to the weights of other objectives in the
  same project.
- `value_half_life` SHALL state how quickly the value of improving this
  objective decays, in units (e.g. days) the plan's body states, so that a
  sooner improvement ranks higher than an equally-sized later one.
- `budget` SHALL state the time, tokens, or compute allowed per attempt at
  this objective, in units the plan's body states, used only to normalize
  comparisons across objectives with different costs per attempt.
- All three fields SHALL be optional and independent of one another and of
  `direction`/`bound`.
- The schema and the Rust `Objective` type SHALL refuse a non-numeric
  `weight`, `value_half_life`, or `budget`.
- The schema and the Rust `Objective` type SHALL refuse a negative `weight`
  and a negative `budget`. The schema and the Rust `Objective` type SHALL
  accept zero for both (an objective may carry no relative value or no
  budget).
- The Rust `Objective` type SHALL refuse a non-finite `weight` or `budget`
  (NaN or infinite). JSON Schema has no finiteness keyword, so a YAML `.inf`
  or `.nan` value satisfies `type: number`; finiteness is enforced only by
  the Rust type, at construction and at deserialization, as FR-020 already
  does for `bound`.
- The schema and the Rust `Objective` type SHALL refuse a `value_half_life`
  that is zero, negative, or non-finite: a half-life SHALL be strictly
  positive, since a decay rate of zero or less is not a decay.
- **These fields SHALL NEVER gate anything. They SHALL NEVER feed
  `statistical_design.decision_rule`.** No function in this module that
  evaluates a decision rule SHALL read `weight`, `value_half_life`, or
  `budget`: [`DecisionRule::holds`] takes an estimate and an optional
  baseline value, never an `Objective`, so a decision rule's verdict is
  structurally unable to depend on the steering fields regardless of what
  they contain -- including extreme or adversarial values such as
  `weight: 0` or `budget: 0`.
- The steering fields SHALL NOT be part of the plan's measurement
  definition: unlike `direction` and `bound`, adding, removing, or changing
  `weight`, `value_half_life`, or `budget` alone SHALL NOT be reported as a
  definition change and SHALL NOT require a new `definition_version`. They
  are declarations about priority, not about how the number is computed --
  the same rationale FR-024 uses to exclude `negative_controls`.
- The MeasurementPlan skeleton SHALL show all three steering fields with a
  section explaining that they are advisory only and never gate, and the
  onboarding skill SHALL describe them as advisory.
- The steering fields SHALL be reachable through the `measurement` Cargo
  feature that activates no `serde_json` dependency, as `Objective` already
  is under FR-020.

## Error Conditions

A non-numeric `weight`, `value_half_life`, or `budget`; a negative `weight`
or `budget`; a non-positive or non-finite `value_half_life`; and a non-finite
`weight` or `budget` fail validation or construction and are never read as a
valid objective. A steering-field edit without a `definition_version` change
is accepted silently, by design: it is not part of the measurement
definition.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-026-AC-1 | The MeasurementPlan frontmatter schema accepts an `objective` with any combination of `weight`, `value_half_life`, and `budget` present or absent, `weight: 0` and `budget: 0`, and large finite values; it rejects a negative `weight`, a negative `budget`, a zero or negative `value_half_life`, and a non-numeric value for any of the three (a string, a boolean, an array). | Test (TC-166) |
| FR-026-AC-2 | The Rust `Objective::with_steering` accepts every valid combination of steering fields and refuses a non-finite or negative `weight`, a non-finite, zero, or negative `value_half_life`, and a non-finite or negative `budget`, each with a distinct typed `ObjectiveError` variant, both when constructed and when deserialized from YAML. | Test (TC-167) |
| FR-026-AC-3 | Given two otherwise-identical plan definitions whose `objective` differs only in `weight`, `value_half_life`, or `budget` (added, removed, or changed) under an unchanged `definition_version`, the FR-020 definition-change check reports no finding; the same is true when all three change at once. A `direction` or `bound` change alongside an unchanged steering field is still reported, naming `objective`. | Test (TC-168) |
| FR-026-AC-4 | Otherwise-identical MeasurementPlan frontmatter documents -- one carrying the smallest accepted steering-field values (`weight: 0`, `value_half_life` at its smallest accepted magnitude, `budget: 0`), one carrying the largest finite values, and one carrying none -- parse to the same `statistical_design.decision_rule`, and evaluating that rule against the same estimate and baseline value via `DecisionRule::holds` produces an identical verdict in every case, for a threshold rule and a baseline rule, including each rule's exact boundary. | Test (TC-169) |
| FR-026-AC-5 | The MeasurementPlan skeleton carries a valid `objective` with all three steering fields and a section stating they are advisory only and never gate; the onboarding skill describes the three fields as advisory; the schema and skeleton changes carry no warning from the onboarding checklist generator. | Test (TC-170) |
| FR-026-AC-6 | A minimal consumer that compiles the crate with default features disabled and only `measurement` enabled reaches `Objective::with_steering`, `Objective::weight`, `Objective::value_half_life`, and `Objective::budget`, and resolves no `serde_json` package anywhere in its dependency graph. | Test (TC-142) |

## Dependencies

- **Upstream**: [FR-020](./FR-020-measurement-plan-objective.md) for the
  `objective` block, the `measurement` feature, and the definition-change
  check; [FR-021](./FR-021-measurement-plan-decision-rule.md) for the
  decision rule these fields must never feed.
- **Downstream**: none yet declared. A future portfolio-ranking consumer
  that reads these fields to order objectives across a project is out of
  scope here and would be a separate ticket under PLAT-956.
