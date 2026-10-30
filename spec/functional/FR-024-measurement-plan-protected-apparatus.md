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

The MeasurementPlan schema SHALL accept a `protected_apparatus` naming the
files that produce the plan's number. It SHALL require that list when the plan
declares its own `execution_procedure` or an `apparatus-edit` control. A
non-retired `stage: gate` plan SHALL declare `negative_controls`,
`ground_truth_kind`, and a `statistical_design` with a typed `decision_rule`
whether or not it owns a producer. A gate over independently
produced, admitted evidence can omit the file list; its typed decision rule and
evidence requirements still determine the gate. A change that edits declared
apparatus has changed the measurement rather than the thing measured, so it
earns no credit toward the plan's objective.

## Inputs

- One MeasurementPlan frontmatter document with optional:
  - `protected_apparatus`: a list of repository-relative file paths and
    directory entries;
  - `negative_controls`: a list of `{ kind, description }` entries, where
    `kind` is exactly one of `suppressed-observation`, `gain-within-noise`,
    `stale-evidence`, `apparatus-edit`, or `selective-reporting`;
  - `ground_truth_kind`: one of `human-labelled`, `agent-labelled`, or
    `mechanical`.
- For the definition-change check: the plan's frontmatter before and after an
  edit, as in [FR-020](./FR-020-measurement-plan-objective.md).

## Outputs

- A schema validation result for the frontmatter document.
- A typed Rust `ApparatusPath`, `ProtectedApparatus`, `NegativeControl`, or
  `NegativeControls` value, or a typed refusal naming the broken rule.
- For the definition-change check: no finding, or one typed finding whose
  changed members include `protected_apparatus`.

## Behavior

Declaring `execution_procedure` is how a plan elects to govern a producer.
The schema does not infer producer ownership from repository layout. The two
apparatus requirement triggers are that field and an `apparatus-edit` control;
an author can also elect to declare the list without either trigger. Every
declared list receives the same entry validation.

Schema admission validates the plan definition, not a passing execution result.
Quoin's [FR-108](ix://agent-ix/quoin/FR-108) evaluates gate verdicts over
admitted collections; missing or inconclusive evidence does not become a pass.

- `protected_apparatus` SHALL be a non-empty list with no repeated entry when
  present.
- The schema SHALL require `protected_apparatus` at every stage when
  `execution_procedure` is declared or any `negative_controls` entry has kind
  `apparatus-edit`: an apparatus-edit control over no declared apparatus guards
  nothing. `stage: gate` alone SHALL not require it.
- Each entry SHALL be either a repository-relative file path or a directory
  entry `<directory>/**`, which names every file under that directory,
  recursively. Segments are separated by `/`. `*` SHALL appear only in that
  final `/**` segment, and a directory entry SHALL name at least one
  directory, so `**` alone is refused and the whole repository cannot be
  protected. No other wildcard exists, which keeps the syntax a proved checker
  must handle small.
- The schema and the Rust `ApparatusPath` type SHALL refuse an empty entry, a
  leading `/`, an empty segment (`//` or a trailing `/`), a `.` or `..`
  segment, `**` alone, `*` anywhere but a final `/**` segment, the
  characters `\`, `?`, `[`, `]`, `{`, `}`, `:`, and control characters. A
  control character is any C0 character, DEL, or C1 character (Unicode
  general category `Cc`, Rust `char::is_control`); the schema refuses the
  same class through its pattern and again through a `not` pattern, so a
  regex engine whose `$` matches before a trailing newline still refuses
  `a.json` followed by a newline. The refused characters keep Windows drive
  and separator forms out and leave no entry that can name a file outside the
  repository.
- A plan declaring `protected_apparatus` SHALL protect its population through
  the file that selects or enumerates it. The
  `statistical_design.population` field stays prose and is not part of the
  protected apparatus: it describes the population to a reader, while the
  selecting file decides which items are counted, and only a file can be
  named by an entry.
- Quoin's measurement intake SHALL resolve every entry to files when it
  writes a measurement collection, and SHALL refuse to write a collection
  whose entry names no file. This repository states the rule and does not
  read the files.
- Quoin's resolution of an entry SHALL be case-sensitive and include
  dotfiles.
- Quoin's resolution SHALL refuse, without following it, a symlink named by
  an entry or found under a directory entry.
- Quoin's resolution SHALL refuse a directory entry that contains no file.
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
- `negative_controls` SHALL be a non-empty list with no repeated entry when
  present. Each entry SHALL carry exactly `kind` and a non-empty
  `description` saying how the plan says it guards against that scenario.
- A negative control SHALL be a declaration. This repository checks only its
  shape: the closed kind, the description, and the list rules. Quoin's
  measurement checker (Linear PLAT-961) exercises the kinds it can detect:
  `selective-reporting` through its rerun-until-pass rule. A kind no checker
  exercises, including `apparatus-edit`, remains an unverified declaration.
- The kinds SHALL mean: `suppressed-observation`, an unfavourable item or run
  is left out of the collected population; `gain-within-noise`, an
  improvement smaller than the plan's stated uncertainty is claimed as a gain;
  `stale-evidence`, a result collected against an earlier subject version or
  apparatus is presented as current; `apparatus-edit`, a protected file is
  edited alongside the change it grades; and `selective-reporting`, only a
  favourable run or variant is reported out of several tried.
- The schema SHALL require `negative_controls` when `stage` is `gate`
  and `status` is not `retired`, as it
  requires `ground_truth_kind`. The Rust types carry no stage; the gate-stage
  requirement is enforced by the schema.
- `negative_controls` SHALL NOT be part of the measurement definition: the
  controls are declarations about the measurement, not how the number is
  computed.
- The schema SHALL list exactly the wire names of the Rust
  `NegativeControlKind` enum as the `negative_controls[].kind` values.
- The schema's entry pattern and the Rust `ApparatusPath` SHALL accept and
  refuse the same entries over one shared case table,
  `tests/fixtures/apparatus-paths.json`, in which each refused entry names
  its `ApparatusPathError` variant.
- The onboarding checklist SHALL list both lists' shape and the entry
  description, the control kinds, and the procedure and apparatus-edit
  requirements, and the MeasurementPlan skeleton and onboarding skill SHALL
  show both fields.
- `ApparatusPath`, `ProtectedApparatus`, `NegativeControlKind`,
  `NegativeControl`, and `NegativeControls` SHALL be reachable through the
  `measurement` Cargo feature.
- An optional `execution_procedure` SHALL be
  one safe repository-relative JSON file path. When present, the plan SHALL
  declare `protected_apparatus`; Quoin SHALL verify that the exact procedure
  path is protected before execution.
- The schema SHALL admit `ground_truth_kind` only as `human-labelled`,
  `agent-labelled`, or `mechanical` at any stage, and SHALL require it for a
  gate-stage plan that is not retired.

## Error Conditions

An unsafe or malformed entry, an empty or repeated `protected_apparatus` list,
an unknown control kind, a control with a missing or empty description or an
extra key, an empty or repeated `negative_controls` list, a gate-stage plan
without `negative_controls`, `ground_truth_kind`, or a typed statistical decision rule,
an unknown `ground_truth_kind`, and
a plan with an `apparatus-edit` control but no `protected_apparatus` fail
validation or construction. An edit to
the protected list without a `definition_version` change produces a finding
rather than being accepted silently.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-024-AC-1 | The MeasurementPlan frontmatter schema accepts no `protected_apparatus` below gate stage and a list of the shared case table's accepted file paths and directory entries; it rejects an empty list, a repeated entry, a non-list, a non-string entry, and every unsafe entry in the shared case table. | Test (TC-154) |
| FR-024-AC-2 | The schema accepts each control kind at every stage, leaves `negative_controls` optional below `gate`, and requires it at `gate`; it rejects an empty list, an unknown or missing kind, a missing or empty description, an extra key, a repeated control, and a bare kind string. | Test (TC-155) |
| FR-024-AC-3 | The Rust `ApparatusPath` accepts every accepted entry of the shared case table, reporting its directory for a directory entry, and refuses every refused entry with the typed error the table names, through construction, parsing, and deserialization, and the schema's entry pattern agrees on every case; `ProtectedApparatus` refuses an empty or repeated list, compares as a set, and round-trips in sorted order. | Test (TC-156) |
| FR-024-AC-4 | The schema's control kinds equal the Rust `NegativeControlKind` wire names in order, `ALL` covers every variant, and the non-retired gate-stage `then` requires `negative_controls`; `NegativeControl` and `NegativeControls` refuse an empty description, an empty list, and a repeated control with typed errors, refuse unknown kinds and extra keys on deserialization, and round-trip. | Test (TC-157) |
| FR-024-AC-5 | Given two plan definitions with an equal `definition_version`, a protected entry added, removed, or changed, or the list added or removed, yields one typed finding naming `protected_apparatus`; the same edit with a different `definition_version`, and a reordered list, yield no finding. | Test (TC-158, TC-141) |
| FR-024-AC-6 | The MeasurementPlan skeleton carries a valid `protected_apparatus` and `negative_controls` with sections explaining both, the onboarding skill describes both, and the onboarding checklist lists both lists' shape, the entry description rather than the raw pattern, the control kinds, and the procedure and apparatus-edit requirements; it omits lists with nothing to say, and no artifact type has a warning. | Test (TC-159) |
| FR-024-AC-7 | A minimal consumer with only the `measurement` feature reaches `ApparatusPath`, `ProtectedApparatus`, `NegativeControlKind`, `NegativeControl`, and `NegativeControls`, and resolves no `serde_json`. | Test (TC-142) |
| FR-024-AC-8 | The schema accepts a non-retired gate-stage plan without `protected_apparatus` when it has no `execution_procedure` or `apparatus-edit` control and declares ground truth, controls, and a typed statistical decision rule; it refuses a non-retired gate missing any of those three, and a plan at any stage with a procedure or `apparatus-edit` control but no `protected_apparatus`. | Test (TC-155) |
| FR-024-AC-9 | A retired gate-stage plan under the prose contract remains valid without fields added later for new gate plans; the same omission is refused for a proposed or active gate-stage plan, and a retired current-shape plan remains valid. | Test (TC-172) |
| FR-024-AC-10 | The schema accepts an absent `execution_procedure` and safe repository-relative JSON file paths with `protected_apparatus` declared; it refuses absolute, traversing, malformed, wildcard, non-JSON, and non-string paths, and refuses a procedure path without `protected_apparatus`. Exact protected-path membership remains a Quoin obligation. | Test (TC-185) |
| FR-024-AC-11 | The schema accepts each of `human-labelled`, `agent-labelled`, and `mechanical` at gate and below gate, requires `ground_truth_kind` at gate with no other finding, leaves it optional below gate, and refuses any other value, including a case variant and an empty string, at every stage. | Test (TC-203) |

FR-024-AC-12 is withdrawn. Its row is removed and the identifier is not reused.

## Dependencies

- **Upstream**: [FR-020](./FR-020-measurement-plan-objective.md) for the
  `measurement` feature and the definition-change check;
  [FR-021](./FR-021-measurement-plan-decision-rule.md) for the rest of the
  measurement definition.
- **Downstream**: Quoin's measurement intake, which resolves each
  protected entry to files when it writes a collection; Quoin's measurement
  checker, which exercises the negative-control kinds it can detect (Linear
  PLAT-961); change assurance, which
  gives no credit for a change that touches the protected apparatus (Linear
  PLAT-964).
