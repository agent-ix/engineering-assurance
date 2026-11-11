---
id: FR-034
title: "Declare a paired run relation on a MeasurementPlan"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-020"
    type: "references"
---

# FR-034: Declare a paired run relation on a MeasurementPlan

## Description

A MeasurementPlan SHALL accept an optional `paired_run` frontmatter object
stating how two runs of the same measurement relate. The object is a
declaration only: Engineering Assurance validates its shape and the consuming
checker acts on it.

## Inputs

- One MeasurementPlan frontmatter document, optionally carrying `paired_run`.

## Outputs

- A schema validation result for the frontmatter document.

## Behavior

- `paired_run` SHALL be optional and independent of every other plan field.
- `paired_run.relation` SHALL be required and one of `equal`, `within_delta`
  or `not_worse`.
- `paired_run.minimum_pairs` SHALL be optional and an integer of at least 1;
  its default when absent is left to the consumer.
- `paired_run.delta` SHALL be a number greater than 0, required when
  `relation` is `within_delta` and refused for any other relation.
- `paired_run` SHALL carry no members beyond `relation`, `minimum_pairs` and
  `delta`.

## Error Conditions

A non-object `paired_run`, a missing or unknown `relation`, a `minimum_pairs`
that is not an integer of at least 1, a missing, non-positive or non-numeric
`delta` where required, a `delta` with a relation other than `within_delta`,
or an extra member fails schema validation.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-034-AC-1 | The MeasurementPlan frontmatter schema accepts an absent `paired_run`, each of the three relations, and `minimum_pairs` of 1 and 5, and rejects an unknown relation, `minimum_pairs` of 0, -1 or "2", a `within_delta` relation without `delta`, a `delta` with relation `equal`, an extra member, and a non-object `paired_run`. | Test (TC-246) |

## Dependencies

- **Upstream**: [FR-020](./FR-020-measurement-plan-objective.md) for the
  MeasurementPlan frontmatter schema.
