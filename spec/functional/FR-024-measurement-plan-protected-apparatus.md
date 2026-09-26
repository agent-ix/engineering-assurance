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
files that produce the plan's number, and SHALL require it, together with
`negative_controls` declaring at least one gaming scenario the plan guards
against, when a non-retired plan's `stage` is `gate`. A change that edits the apparatus
has changed the measurement rather than the thing measured, so it earns no
credit toward the plan's objective.

## Inputs

- One MeasurementPlan frontmatter document with optional:
  - `protected_apparatus`: a list of repository-relative file paths and
    directory entries;
  - `negative_controls`: a list of `{ kind, description }` entries, where
    `kind` is exactly one of `suppressed-observation`, `gain-within-noise`,
    `stale-evidence`, `apparatus-edit`, or `selective-reporting`;
  - `ground_truth_kind`: one of `human-labelled`, `agent-labelled`, or
    `mechanical`;
  - `preregistration`: `{ bar_digest }`, a `sha256:` digest.
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
  present.
- The schema SHALL require `protected_apparatus` when `stage` is `gate`
  and `status` is not `retired`, and,
  at every stage, when any `negative_controls` entry has kind
  `apparatus-edit`: an apparatus-edit control over no declared apparatus
  guards nothing.
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
- A plan SHALL protect its population through the file that selects or
  enumerates it, listed as a `protected_apparatus` entry. The
  `statistical_design.population` field stays prose and is not part of the
  protected apparatus: it describes the population to a reader, while the
  selecting file decides which items are counted, and only a file can be
  digested.
- Quoin's measurement intake SHALL resolve every entry to files when it
  writes a measurement collection, and SHALL refuse to write a collection
  whose entry names no file. This repository states the rule and does not
  read the files.
- Quoin's resolution of an entry SHALL be case-sensitive and include
  dotfiles.
- Quoin's resolution SHALL refuse, without following it, a symlink named by
  an entry or found under a directory entry.
- Quoin's resolution SHALL refuse a directory entry that contains no file.
- Quoin's measurement intake SHALL record the resolved set of (path, digest)
  pairs with each collection, and SHALL treat any difference in that set
  between collections as an apparatus change: a protected file edited, and a
  file added under or removed from a directory entry.
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
- A change to the resolved (path, digest) set SHALL also require a new
  `definition_version`. Quoin detects it by comparing the recorded sets
  across collections (Linear PLAT-975); the definition-change check here sees
  only the list and cannot detect it.
- `negative_controls` SHALL be a non-empty list with no repeated entry when
  present. Each entry SHALL carry exactly `kind` and a non-empty
  `description` saying how the plan says it guards against that scenario.
- A negative control SHALL be a declaration. This repository checks only its
  shape: the closed kind, the description, and the list rules. Quoin's
  measurement checker (Linear PLAT-961) exercises the kinds it can detect:
  `selective-reporting` through its rerun-until-pass rule, and
  `apparatus-edit` through the (path, digest) comparison (Linear PLAT-975).
  A kind no checker exercises remains an unverified declaration.
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
  description, the control kinds, and the gate-stage and apparatus-edit
  requirements, and the MeasurementPlan skeleton and onboarding skill SHALL
  show both fields.
- `ApparatusPath`, `ProtectedApparatus`, `NegativeControlKind`,
  `NegativeControl`, and `NegativeControls` SHALL be reachable through the
  `measurement` Cargo feature.
- In the 0.5.0 Campaign candidate, an optional `execution_procedure` SHALL be
  one safe repository-relative JSON file path. When present, the plan SHALL
  declare `protected_apparatus`; Quoin SHALL verify that the exact procedure
  path is protected and bind its source bytes before execution.
- The schema SHALL admit `ground_truth_kind` only as `human-labelled`,
  `agent-labelled`, or `mechanical` at any stage, and SHALL require it for a
  gate-stage plan that is not retired.
- An optional `preregistration` SHALL carry exactly one member, `bar_digest`,
  a `sha256:` digest of 64 lowercase hexadecimal digits. Quoin's measurement
  intake computes and compares that digest; this repository checks only its
  shape.

## Error Conditions

An unsafe or malformed entry, an empty or repeated `protected_apparatus` list,
an unknown control kind, a control with a missing or empty description or an
extra key, an empty or repeated `negative_controls` list, a gate-stage plan
without `negative_controls`, `protected_apparatus`, or `ground_truth_kind`,
an unknown `ground_truth_kind`, a malformed or extended `preregistration`, and
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
| FR-024-AC-6 | The MeasurementPlan skeleton carries a valid `protected_apparatus` and `negative_controls` with sections explaining both, the onboarding skill describes both, and the onboarding checklist lists both lists' shape, the entry description rather than the raw pattern, the control kinds, and the gate-stage and apparatus-edit requirements; it omits lists with nothing to say, and no artifact type has a warning. | Test (TC-159) |
| FR-024-AC-7 | A minimal consumer with only the `measurement` feature reaches `ApparatusPath`, `ProtectedApparatus`, `NegativeControlKind`, `NegativeControl`, and `NegativeControls`, and resolves no `serde_json`. | Test (TC-142) |
| FR-024-AC-8 | The schema refuses a non-retired gate-stage plan without `protected_apparatus`, and a plan at any stage with an `apparatus-edit` negative control but no `protected_apparatus`; it accepts both with a protected list, and a below-gate plan whose controls are of other kinds without one. | Test (TC-155) |
| FR-024-AC-9 | A retired gate-stage plan under the v0.2.1 prose contract remains valid without fields added later for new gate plans; the same omission is refused for a proposed or active gate-stage plan, and a retired current-shape plan remains valid. | Test (TC-172) |
| FR-024-AC-10 | The 0.5.0 candidate schema accepts an absent `execution_procedure` and safe repository-relative JSON file paths with `protected_apparatus` declared; it refuses absolute, traversing, malformed, wildcard, non-JSON, and non-string paths, and refuses a procedure path without `protected_apparatus`. Exact protected-path membership and source-byte binding remain Quoin obligations. | Test (TC-185) |
| FR-024-AC-11 | The schema accepts each of `human-labelled`, `agent-labelled`, and `mechanical` at gate and below gate, requires `ground_truth_kind` at gate with no other finding, leaves it optional below gate, and refuses any other value, including a case variant and an empty string, at every stage. | Test (TC-203) |
| FR-024-AC-12 | The schema accepts a plan without `preregistration` and one whose `preregistration` is exactly `{ bar_digest }` with a `sha256:` digest of 64 lowercase hexadecimal digits; it refuses an empty object, a bare, uppercase, short, long, or other-algorithm digest, a non-string digest, an extra member, and a non-object value. | Test (TC-204) |

## Dependencies

- **Upstream**: [FR-020](./FR-020-measurement-plan-objective.md) for the
  `measurement` feature and the definition-change check;
  [FR-021](./FR-021-measurement-plan-decision-rule.md) for the rest of the
  measurement definition.
- **Downstream**: Quoin's measurement intake, which resolves and digests each
  protected entry when it writes a collection and reports an apparatus change
  between compared collections (Linear PLAT-975); Quoin's measurement
  checker, which exercises the negative-control kinds it can detect (Linear
  PLAT-961); change assurance, which
  gives no credit for a change that touches the protected apparatus (Linear
  PLAT-964).
