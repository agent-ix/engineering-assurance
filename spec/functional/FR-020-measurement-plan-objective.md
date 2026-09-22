---
id: FR-020
title: "Declare a MeasurementPlan objective as part of its definition"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-005"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "requires"
---

# FR-020: Declare a MeasurementPlan objective as part of its definition

## Description

The MeasurementPlan frontmatter SHALL accept an optional `objective` block
stating which way the plan's metric is supposed to move, and a change to that
objective SHALL count as a change to the plan's measurement definition.

## Inputs

- One MeasurementPlan frontmatter document, optionally carrying an `objective`
  object with:
  - `direction`: exactly one of `higher`, `lower`, `zero`, or `target`;
  - `bound`: an optional finite number.
- For the definition-change check: the plan's frontmatter before and after an
  edit, each projected to its `definition_version` and its measurement
  definition (`objective`, and the FR-021 `statistical_design.estimator` and
  `statistical_design.decision_rule`).

## Outputs

- A schema validation result for the frontmatter document.
- A typed Rust `Objective` value, or a typed refusal naming why the objective
  is invalid.
- For the definition-change check: either no finding, or one typed finding
  carrying the unchanged `definition_version`, the changed definition members,
  and both definitions.

## Behavior

- `direction: higher` SHALL mean a larger metric value is better.
- `direction: lower` SHALL mean a smaller metric value is better.
- `direction: zero` SHALL mean the metric is expected to be exactly zero, so
  any non-zero value is a deviation.
- `direction: target` SHALL mean the metric is expected to reach `bound`, and
  a `target` objective SHALL carry `bound`.
- For `higher`, `lower`, and `zero`, `bound` SHALL be optional.
- `bound` SHALL state the goal the metric is meant to reach. It is
  informational and never evaluated; only the FR-021 decision rule is
  evaluated.
- The MeasurementPlan frontmatter schema SHALL accept a plan with no
  `objective`.
- The schema and the Rust `Objective` type SHALL refuse an unknown direction,
  a `target` objective without `bound`, a non-numeric `bound`, and any key in
  `objective` other than `direction` and `bound`.
- The Rust `Objective` type SHALL refuse a non-finite `bound`. JSON Schema has
  no finiteness keyword, and a YAML `.inf` or `.nan` value satisfies
  `type: number`, so the frontmatter schema accepts a non-finite `bound`;
  finiteness is enforced only by the Rust `Objective` type, at construction
  and at deserialization.
- The schema's `direction` values SHALL be exactly the wire names of the Rust
  `Direction` enum.
- The `objective` SHALL be part of the plan's measurement definition: adding,
  removing, or changing it while `definition_version` stays the same SHALL be
  reported as a typed definition-change finding naming `objective` among the
  changed members. Adding, removing, or changing
  it together with a `definition_version` change SHALL produce no finding.
- The Rust `Objective`, `Direction`, and definition-change check SHALL be
  reachable through a `measurement` Cargo feature that activates no
  `serde_json` dependency.

## Error Conditions

An unknown direction, a `target` objective without `bound`, a non-numeric or
non-finite `bound`, or an extra `objective` key fails validation or
construction and is never read as a valid objective. An objective edit without
a `definition_version` change produces a finding rather than being accepted
silently.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-020-AC-1 | The MeasurementPlan frontmatter schema accepts a plan without `objective`, every direction without `bound`, every direction with a numeric `bound`, and `target` with `bound`; it rejects `target` without `bound`, an unknown direction, a missing direction, a non-numeric `bound`, and an extra key in `objective`. The MeasurementPlan skeleton carries a valid `objective`. | Test (TC-139) |
| FR-020-AC-2 | The Rust `Objective` accepts every direction and refuses `target` without `bound` and a non-finite `bound` with distinct typed errors, both when constructed and when deserialized; deserialization refuses unknown directions and extra keys; and the schema's `direction` enum equals the Rust `Direction` wire-name set. | Test (TC-140) |
| FR-020-AC-3 | Given two plan definitions, an added, removed, or changed objective with an equal `definition_version` yields one typed finding carrying that version, `objective` as a changed member, and both definitions; the same edit with a different `definition_version`, and an unchanged definition, yield no finding. | Test (TC-141) |
| FR-020-AC-4 | A minimal consumer compiles the crate with default features disabled and only `measurement` enabled, reaches `Objective` and the definition-change check, and resolves no `serde_json` package anywhere in its dependency graph. | Test (TC-142) |

## Dependencies

- **Upstream**: [FR-014](./FR-014-versioned-rust-boundary.md) for the Rust
  library boundary and its feature-gating pattern.
- **Downstream**: Quoin's measurement intake, which refuses cross-definition
  comparisons and can read the objective through the `measurement` feature
  (Linear PLAT-957).
