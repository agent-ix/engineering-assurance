---
id: FR-033
title: "Declare cross-profile comparison on a MeasurementPlan"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-020"
    type: "references"
---

# FR-033: Declare cross-profile comparison on a MeasurementPlan

## Description

A MeasurementPlan SHALL accept an optional boolean `cross_profile_comparison`
frontmatter key. A plan that sets it to `true` opts in to comparing
measurement collections built under different build profiles; a plan that
omits it keeps the default same-profile comparison rule. The key is a
declaration only: Engineering Assurance validates its shape and the consuming
checker acts on it.

## Inputs

- One MeasurementPlan frontmatter document, optionally carrying
  `cross_profile_comparison`.

## Outputs

- A schema validation result for the frontmatter document.

## Behavior

- The key SHALL be optional and independent of every other plan field.
- The schema SHALL accept `true` and `false`.

## Error Conditions

A non-boolean value (string, number, null, array, object) fails schema
validation.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-033-AC-1 | The MeasurementPlan frontmatter schema accepts `cross_profile_comparison: true`, `false`, and its absence, and rejects a string, number, null, array or object value. | Test (TC-244) |

## Dependencies

- **Upstream**: [FR-020](./FR-020-measurement-plan-objective.md) for the
  MeasurementPlan frontmatter schema.
