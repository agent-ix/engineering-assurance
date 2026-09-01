---
id: FR-009
title: "Preserve producer provenance and non-success states"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-005"
    type: "implements"
---

# FR-009: Preserve producer provenance and non-success states

## Description

Every mapped execution, result, evidence reference, and measurement SHALL
preserve the applicable producer identity and version, configuration digest,
source revision, environment, definition version, and availability/result
state supplied by its authoritative record.

## Inputs

Versioned Quire, Quoin, ix-flow, and native-producer records.

## Outputs

Validated interoperability fixtures and generated Rust, TypeScript, and Python
fixture projections that retain the same semantic values.

## Behavior

- Unknown schema majors or module versions SHALL be incompatible.
- Missing, unavailable, not-computed, malformed, failed, stale, suspect,
  vacuous, tampered, and unreadable SHALL remain distinguishable.
- A projection SHALL carry source field paths and authoritative record
  identities so every value can be traced without duplicating the record.
- The fixture generator SHALL derive every language fixture from the same
  canonical fixture with semantic equality.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-009-AC-1 | Valid fixtures preserve the complete producer tuple and definition version. | Test (TC-060) |
| FR-009-AC-2 | Every declared non-success state survives JSON and generated-language projections unchanged. | Test (TC-061) |
| FR-009-AC-3 | Unknown/incompatible versions and missing required provenance fail explicitly. | Test (TC-062) |
| FR-009-AC-4 | Projection values retain source record and field-path references rather than copied authority. | Test (TC-063) |

## Dependencies

FR-008 establishes owners and link directions. Authoritative producer schemas
remain owned in their source repositories.
