---
id: FR-024
title: "Declare a MeasurementPlan's protected apparatus and negative controls"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-005"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-020"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-021"
    type: "requires"
---

# FR-024: Declare a MeasurementPlan's protected apparatus and negative controls

## Description

The MeasurementPlan schema SHALL accept an optional `protected_apparatus`
naming the files that produce the plan's number, and SHALL require
`negative_controls`, naming at least one gaming scenario the plan's
measurement catches, when the plan's `stage` is `gate`. A change that edits
the apparatus has changed the measurement rather than the thing measured, so
it earns no credit toward the plan's objective.

## Inputs

- One MeasurementPlan frontmatter document with optional:
  - `protected_apparatus`: a list of repository-relative paths or globs;
  - `negative_controls`: a list of `{ kind, description }` entries, where
    `kind` is exactly one of `suppressed-observation`, `gain-within-noise`,
    `stale-evidence`, `apparatus-edit`, or `selective-reporting`.
- For the definition-change check: the plan's frontmatter before and after an
  edit, as in [FR-020](./FR-020-measurement-plan-objective.md).

## Outputs

- A schema validation result for the frontmatter document.
- A typed Rust `ApparatusPath`, `ProtectedApparatus`, `NegativeControl`, or
  `NegativeControls` value, or a typed refusal naming the broken rule.
- For the definition-change check: no finding, or one typed finding whose
  changed members include `protected_apparatus`.

## Behavior

- `protected_apparatus` SHALL list the files that produce the plan's number:
  the measurement harness, the labels, corpus, or answer key, the file that
  selects the population, and the checker configuration.
- `protected_apparatus` SHALL be a non-empty list with no repeated entry when
  present. It is optional at every stage.
- Each entry SHALL be a repository-relative path or glob whose segments are
  separated by `/`. A segment is either exactly `**`, which matches zero or
  more whole directories, or a name in which each `*` matches zero or more
  characters other than `/`. Every other character is literal.
- The schema and the Rust `ApparatusPath` type SHALL refuse an empty entry, a
  leading `/`, an empty segment (`//` or a trailing `/`), a `.` or `..`
  segment, `**` inside a longer segment, and the characters `\`, `?`, `[`,
  `]`, `{`, `}`, `:`, and control characters. The refused characters keep one
  glob syntax, keep Windows drive and separator forms out, and leave no entry
  that can name a file outside the repository.
- A plan SHALL protect its population through the file that selects or
  enumerates it, listed as a `protected_apparatus` entry. The
  `statistical_design.population` field stays prose and is not part of the
  protected apparatus: it describes the population to a reader, while the
  selecting file decides which items are counted, and only a file can be
  digested.
- Quoin's measurement intake SHALL resolve every entry to at least one real
  file and SHALL digest each file an entry names when it writes a measurement
  collection, refusing to write a collection whose entry names no file. This
  repository states the rule and does not read the files.
- The plan's measurement definition SHALL include `protected_apparatus`
  alongside `objective`, `statistical_design.estimator`, and
  `statistical_design.decision_rule`.
- When an entry is added or removed, an entry is changed, or the list is added
  or removed while `definition_version` stays the same, the definition-change
  check SHALL return one typed finding naming `protected_apparatus`. The same
  edit with a `definition_version` change SHALL produce no finding.
- The definition-change check SHALL compare the list as a set: reordering it
  is not a change, and the Rust `ProtectedApparatus` serializes its entries in
  sorted order.
- Editing the content of a protected file SHALL also require a new
  `definition_version`. Quoin detects that edit by comparing file digests
  across collections; the definition-change check here sees only the list and
  cannot detect it.
- `negative_controls` SHALL be a non-empty list with no repeated entry when
  present. Each entry SHALL carry exactly `kind` and a non-empty
  `description` saying how the plan's measurement catches that scenario.
- The kinds SHALL mean: `suppressed-observation`, an unfavourable item or run
  is left out of the collected population; `gain-within-noise`, an
  improvement smaller than the plan's stated uncertainty is claimed as a gain;
  `stale-evidence`, a result collected against an earlier subject version or
  apparatus is presented as current; `apparatus-edit`, a protected file is
  edited alongside the change it grades; and `selective-reporting`, only a
  favourable run or variant is reported out of several tried.
- The schema SHALL require `negative_controls` when `stage` is `gate`, as it
  requires `ground_truth_kind`. The Rust types carry no stage; the gate-stage
  requirement is enforced by the schema.
- `negative_controls` SHALL NOT be part of the measurement definition: the
  controls describe what the measurement catches, not how the number is
  computed.
- The schema SHALL list exactly the wire names of the Rust
  `NegativeControlKind` enum as the `negative_controls[].kind` values.
- The schema's entry pattern and the Rust `ApparatusPath` SHALL accept and
  refuse the same entries over one shared case table.
- The onboarding checklist SHALL list both lists' shape and item pattern, the
  control kinds, and the gate-stage requirement, and the MeasurementPlan
  skeleton and onboarding skill SHALL show both fields.
- `ApparatusPath`, `ProtectedApparatus`, `NegativeControlKind`,
  `NegativeControl`, and `NegativeControls` SHALL be reachable through the
  `measurement` Cargo feature.

## Error Conditions

An unsafe or malformed entry, an empty or repeated `protected_apparatus` list,
an unknown control kind, a control with a missing or empty description or an
extra key, an empty or repeated `negative_controls` list, and a gate-stage
plan without `negative_controls` fail validation or construction. An edit to
the protected list without a `definition_version` change produces a finding
rather than being accepted silently.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-024-AC-1 | The MeasurementPlan frontmatter schema accepts no `protected_apparatus` and a list of safe relative paths and globs; it rejects an empty list, a repeated entry, a non-list, a non-string entry, and every unsafe entry in the shared case table. | Test (TC-154) |
| FR-024-AC-2 | The schema accepts each control kind at every stage, leaves `negative_controls` optional below `gate`, and requires it at `gate`; it rejects an empty list, an unknown or missing kind, a missing or empty description, an extra key, a repeated control, and a bare kind string. | Test (TC-155) |
| FR-024-AC-3 | The Rust `ApparatusPath` accepts every accepted entry and refuses every refused entry of the shared case table with a distinct typed error, through construction, parsing, and deserialization, and the schema's entry pattern agrees on every case; `ProtectedApparatus` refuses an empty or repeated list, compares as a set, and round-trips in sorted order. | Test (TC-156) |
| FR-024-AC-4 | The schema's control kinds equal the Rust `NegativeControlKind` wire names in order, `ALL` covers every variant, and the gate-stage `then` requires `negative_controls`; `NegativeControl` and `NegativeControls` refuse an empty description, an empty list, and a repeated control with typed errors, refuse unknown kinds and extra keys on deserialization, and round-trip. | Test (TC-157) |
| FR-024-AC-5 | Given two plan definitions with an equal `definition_version`, a protected entry added, removed, or changed, or the list added or removed, yields one typed finding naming `protected_apparatus`; the same edit with a different `definition_version`, and a reordered list, yield no finding. | Test (TC-158, TC-141) |
| FR-024-AC-6 | The MeasurementPlan skeleton carries a valid `protected_apparatus` and `negative_controls` with sections explaining both, the onboarding skill describes both, and the onboarding checklist lists both lists' shape, the entry pattern, the control kinds, and the gate-stage requirement, with no warning. | Test (TC-159) |
| FR-024-AC-7 | A minimal consumer with only the `measurement` feature reaches `ApparatusPath`, `ProtectedApparatus`, `NegativeControlKind`, `NegativeControl`, and `NegativeControls`, and resolves no `serde_json`. | Test (TC-142) |

## Dependencies

- **Upstream**: [FR-020](./FR-020-measurement-plan-objective.md) for the
  `measurement` feature and the definition-change check;
  [FR-021](./FR-021-measurement-plan-decision-rule.md) for the rest of the
  measurement definition.
- **Downstream**: Quoin's measurement intake, which resolves and digests each
  protected entry when it writes a collection and reports an apparatus change
  between compared collections (Linear PLAT-975); change assurance, which
  gives no credit for a change that touches the protected apparatus (Linear
  PLAT-964).
