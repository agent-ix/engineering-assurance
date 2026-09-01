---
id: IT-005
title: "Verify cross-component semantic compatibility"
type: IT
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-008"
    type: "verifies"
  - target: "ix://agent-ix/engineering-assurance/FR-009"
    type: "verifies"
  - target: "ix://agent-ix/engineering-assurance/FR-010"
    type: "verifies"
---

# IT-005: Verify cross-component semantic compatibility

## Objective

Validate canonical and legacy fixtures against exact Quire, Quoin, ix-flow,
and native-producer premises without executing any producer.

## Target Integration

Engineering Assurance mapping and projection code consumes versioned Quire,
Quoin, ix-flow, and native-result fixtures as read-only inputs.

## Preconditions

- Exact schema/module versions and artifact digests are supplied.
- Fixture inputs are immutable for the duration of the check.

## Inputs

Canonical reference fixtures, PGM-01 v1/v2 fixtures, and invalid or incomplete
variants with exact version premises.

## Test Procedure

1. Validate the ownership registry and canonical reference fixture.
2. Validate JSON and generated Rust, TypeScript, and Python projections for
   semantic equality.
3. Map PGM-01 v1/v2 fixtures read-only and compare source digests before/after.
4. Exercise every non-success, unknown-version, and missing-provenance case.
5. Render JSON and Markdown reports and inspect the required bounded sections.

## Expected Results

The exact valid fixtures pass; incompatible or ambiguous fixtures fail
explicitly; source bytes are unchanged; and no producer, shell, evidence store,
or workflow transition is invoked.

## Metadata

- Priority: P0
- Target Integration: versioned shared assurance fixtures
- Automation: local-only automated contract tests

## Dependencies

FR-008 through FR-010 and their reviewed ownership/type-fit ADR.

## Traceability

Verifies [FR-008](../functional/FR-008-distinguish-verification-semantics.md),
[FR-009](../functional/FR-009-preserve-provenance-and-states.md), and
[FR-010](../functional/FR-010-read-only-compatibility-and-reporting.md).
