---
id: US-005
title: "Correlate a definition, result, evidence, and decision"
type: US
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-002"
    type: "derives_from"
---

# US-005: Correlate a definition, result, evidence, and decision

## Story

**As an** assurance owner
**I want** one ownership and type-fit contract that links an authored
verification definition to native executions, structured results, retained
evidence, measurements, diagnostics, reports, and an attributed human decision
**So that** missing or incompatible information stays visible.

## Acceptance Examples (Illustrative)

| ID | Criteria | Verification |
| --- | --- | --- |
| US-005-AC-1 | A reviewer can identify the authoritative owner, schema/version premise, and link for every semantic concept without consulting a parallel generic envelope. | Inspection (TC-053) |

US-005-AC-2 and US-005-AC-3 are withdrawn. Their rows are removed and the identifiers are not reused.

## Dependencies (Contextual)

Quire, Quoin, native producer, and ix-flow contracts remain authoritative.

## Exception Scenarios

- Unknown schema versions are incompatible, not empty.
- Missing, unavailable, not-computed, malformed, stale, suspect, vacuous,
  failed, and tampered states remain distinguishable.

## Assurance Risk

The primary risk is a presentation layer silently turning an
absent execution, failed result, or unauthorised inference into successful
evidence.

## Priority and Risk (Informative)

- **Priority:** P0.
- **Primary risk:** semantic collapse can turn absence or failure into apparent
  successful evidence.
