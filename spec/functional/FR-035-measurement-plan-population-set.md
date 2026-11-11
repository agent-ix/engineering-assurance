---
id: FR-035
title: "Declare an enumerated population or an exemption on a MeasurementPlan"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-020"
    type: "references"
---

# FR-035: Declare an enumerated population or an exemption on a MeasurementPlan

## Description

A MeasurementPlan's `statistical_design` SHALL accept an optional
`population_set` object enumerating the population a percentage metric is taken
over, and an optional `population_exempt` reason string. Both are declarations
only: Engineering Assurance validates their shape and the consuming checker acts
on them.

## Inputs

- One MeasurementPlan frontmatter document whose `statistical_design` optionally
  carries `population_set` and `population_exempt`.

## Outputs

- A schema validation result for the frontmatter document.

## Behavior

- `population_set` and `population_exempt` SHALL each be optional and
  independent of every other `statistical_design` member and of each other.
- `population_set` SHALL be an object whose only member is `ids`, which is
  required.
- `population_set.ids` SHALL be a non-empty array of distinct, non-empty strings.
- `population_exempt` SHALL be a string containing at least one
  non-whitespace character.

## Error Conditions

A non-object `population_set`, a missing `ids`, an `ids` that is not an array,
is empty, repeats an id, or holds an empty or non-string id, an extra member of
`population_set`, and a `population_exempt` that is not a string or is empty or
blank fails schema validation.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-035-AC-1 | The MeasurementPlan frontmatter schema accepts a `statistical_design` with neither member, with `population_set` of distinct ids, with `population_exempt` text, and with both, and rejects an empty `ids`, a repeated id, an empty-string id, a non-string id, an extra `population_set` member, a missing `ids`, a non-object `population_set`, and an empty, blank or non-string `population_exempt`. | Test (TC-247) |

## Dependencies

- **Upstream**: [FR-020](./FR-020-measurement-plan-objective.md) for the
  MeasurementPlan frontmatter schema.
