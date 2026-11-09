---
id: FR-015
title: "Preserve semantic and identity behavior in Rust"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-009"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "requires"
---

# FR-015: Preserve semantic and identity behavior in Rust

## Description

Engineering Assurance SHALL port evidence-state
classification, verification-semantic validation, bounded projections, and
fixture generation to Rust without changing observable identities or ownership.

## Inputs

- Canonical semantic-reference fixtures.
- Finite RFC 8259 JSON producer outputs, including exact integers outside the
  `i64` and `u64` ranges.
- Accepted portable verification-contract versions when available.

## Outputs

- Versioned classifications, mappings, projections, and generated fixture bytes.
- Explicit errors for unknown, malformed, ambiguous, stale, or tampered inputs.

## Behavior

- Preserve every declared availability and non-success state without collapsing
  it to success.
- Exact integer values SHALL NOT pass through binary floating-point
  conversion.
- Reject non-finite or overflowing numeric identity inputs.
- Expose stable typed availability spellings, exact-one validation for untyped
  labels, result validity, one stable top-level semantic-error category, and an
  exhaustive typed refusal reason without formatted-output parsing.
- Reject empty, whitespace-bearing, and non-printable governing versions and
  accept any other version token.
- Assign a new explicit version to a different canonicalization algorithm.
- Keep generated foreign-language fixtures as inert data from one canonical
  semantic source.
- Consume portable verification contracts by version rather than copying them
  into a second contract family.

## Error Conditions

Unknown versions, missing provenance, ambiguous mappings,
malformed fixtures, invalid governing versions, unsupported canonicalization,
and non-finite or overflowing numeric inputs fail explicitly while preserving
the original input bytes. Each semantic failure exposes a typed refusal reason;
diagnostic prose is not the discriminator.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-015-CON-2 | Generated foreign-language fixtures SHALL NOT be executed by qualification. | Security | Test (TC-104) |
| FR-015-CON-3 | The implementation SHALL NOT define a second persisted verification or evidence record family. | Responsibility | Test (TC-104) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-015-AC-1 | Over focused fictional cases, Rust produces the declared canonical fixtures, including exact integers beyond `i64` and `u64`. | Test (TC-100) |
| FR-015-AC-2 | Success, unavailable, not-computed, not-applicable, failed, inconclusive, malformed, stale, tampered, and unreadable remain distinguishable; typed state labels retain stable wire spellings and untyped labels require exactly one known state. | Test (TC-102) |
| FR-015-AC-3 | Invalid versions, provenance, mappings, fixtures, and numeric identity inputs fail without a source-byte change; every exercised semantic refusal exposes its expected typed reason without inspecting diagnostic prose. | Test (TC-103) |
| FR-015-AC-4 | Static ownership and execution audits find no copied portable contract family, persisted evidence family, or executable foreign-language fixture. | Test (TC-104) |

FR-015-AC-5, FR-015-CON-1, and FR-015-CON-4 are withdrawn. Their rows are removed and the identifiers are not reused.

## Dependencies

- **Upstream**: FR-008, FR-009, FR-014, and any accepted portable
  verification-contract release consumed by the implementation.
- **Downstream**: FR-016 and FR-017 reuse these types and classifiers.
