---
id: StR-003
title: "Centralize Engineering Assurance behavior in Rust"
type: StR
relationships:
  - target: "ix://agent-ix/engineering-assurance/ADR-002"
    type: "traces_to"
---

# StR-003: Centralize Engineering Assurance behavior in Rust

## Stakeholder Need

As the repository owner, I need first-party Engineering Assurance production
and qualification behavior to live in one reviewable Rust boundary so that its
language, ownership, compatibility, and test evidence are explicit.

## Rationale

The current behavior is divided among Python library and audit modules,
JavaScript workflow invariants, MJS evaluation contracts, and package scripts.
That division obscures which component owns a rule and permits qualification
logic to diverge between hosts. The migration must centralize the behavior
without taking over Quire validation, Quoin persistence, ix-flow lifecycle, or
portable verification contracts.

## Validation Criteria

| ID | Criteria | Validation |
| --- | --- | --- |
| StR-003-VC-1 | The existing engineering-assurance repository contains the shared Rust library and native CLI; no additional repository owns the migration. | Test (TC-096) |
| StR-003-VC-2 | Every current first-party executable path has a recorded classification, Rust remediation, external-host dependency, or explicit owner disposition. | Test (TC-097) |
| StR-003-VC-3 | Differential qualification preserves accepted success, non-success, malformed, stale, tampered, canonicalization, report, and refusal behavior. | Test (TC-100) |
| StR-003-VC-4 | Completion evidence identifies no unapproved Python, JavaScript, MJS, shell, generated, or inline-CI semantic or assertion logic. | Test (TC-115) |

## Boundaries

- Existing Quoin implementation and evidence ownership are retained.
- Native producers keep their domain execution and result formats.
- Foreign-language fixture samples remain data only and are never executed as
  qualification logic.
- Filament components are outside this migration.
