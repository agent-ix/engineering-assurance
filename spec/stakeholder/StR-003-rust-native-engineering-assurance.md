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
The port centralizes that behavior without taking over Quire validation, Quoin
persistence, ix-flow lifecycle, cli-agent-evals host mechanics, portable
verification contracts, or any project's own assurance artifacts.

## Validation Criteria

| ID | Criteria | Validation |
| --- | --- | --- |
| StR-003-VC-1 | This repository contains the shared Rust library and native CLI; no additional repository owns the port. | Test (TC-096) |
| StR-003-VC-3 | Differential qualification preserves accepted success, non-success, malformed, stale, tampered, canonicalization, report, and refusal behavior. | Test (TC-100) |
| StR-003-VC-5 | The Python package entry point contains no semantic implementation, and supported host configuration and package scripts remain declarative or dispatch to the native CLI rather than embedding semantic assertions. | Test (TC-115) |

StR-003-VC-2 and StR-003-VC-4 are withdrawn. Their rows are removed and the
identifiers are not reused.

## Boundaries

- Each project remains responsible for its own assurance artifacts and use of
  Engineering Assurance tools.
- Existing Quoin implementation and evidence ownership are retained.
- Native producers keep their domain execution and result formats.
- Foreign-language fixture samples remain inert data.
- Filament components are outside this port.
