---
id: IT-005
title: "Verify cross-component semantic compatibility"
type: IT
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-008"
    type: "verifies"
  - target: "ix://agent-ix/engineering-assurance/FR-009"
    type: "verifies"
---

# IT-005: Verify cross-component semantic compatibility

## Objective

Validate canonical fixtures against exact Quire, Quoin, ix-flow,
and native-producer premises without executing any producer.

## Target Integration

Engineering Assurance validation and projection code consumes versioned Quire,
Quoin, ix-flow, and native-result fixtures as read-only inputs.

## Preconditions

- Exact schema/module versions are supplied.
- Fixture inputs are immutable for the duration of the check.

## Inputs

Canonical reference fixtures and invalid or incomplete
variants with exact version premises.

## Test Procedure

1. Validate the ownership registry and canonical reference fixture.
2. Validate JSON and generated Rust, TypeScript, and Python projections for
   semantic equality.
3. Exercise every non-success, unknown-version, and missing-provenance case.

## Expected Results

The exact valid fixtures pass; incompatible or ambiguous fixtures fail
explicitly; and no producer, shell, evidence store,
or workflow transition is invoked.

## Metadata

- Priority: P0
- Target Integration: versioned shared assurance fixtures
- Automation: local-only automated contract tests

## Dependencies

FR-008 and FR-009 and their reviewed ownership/type-fit ADR.

## Traceability

Verifies [FR-008](../functional/FR-008-distinguish-verification-semantics.md) and
[FR-009](../functional/FR-009-preserve-provenance-and-states.md).
