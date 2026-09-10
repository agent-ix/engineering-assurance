---
id: FR-015
title: "Preserve semantic, identity, and compatibility behavior in Rust"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-009"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-010"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-011"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "requires"
---

# FR-015: Preserve semantic, identity, and compatibility behavior in Rust

## Description

Engineering Assurance SHALL port compatibility classification, accepted-corpus
access, evidence-state classification, verification-semantic validation,
bounded projections, and fixture generation to Rust without changing observable
identities or ownership.

## Inputs

- The accepted compatibility matrix and pinned corpus.
- Canonical semantic-reference and report fixtures.
- Finite RFC 8259 JSON producer outputs, including exact integers outside the
  `i64` and `u64` ranges.
- Historical PGM-01 v1/v2 records.
- Accepted portable verification-contract versions when available.

## Outputs

- Versioned classifications, mappings, projections, and generated fixture bytes.
- Explicit errors for unknown, malformed, ambiguous, stale, or tampered inputs.

## Behavior

- Preserve every declared availability and non-success state without collapsing
  it to success.
- Preserve source-field references and declared lossy or unmapped limitations
  in historical views.
- Preserve legacy canonical JSON bytes and identity digests. Exact integer
  values SHALL NOT pass through binary floating-point conversion.
- Reject non-finite or overflowing numeric identity inputs without producing an
  identity digest.
- Treat canonical output bytes and their digest, rather than parsed structural
  equality, as evidence identity.
- Expose stable typed availability spellings, exact-one validation for untyped
  labels, result validity, one stable top-level semantic-error category, and an
  exhaustive typed refusal reason without formatted-output parsing.
- When a historical PGM-01 identity is a JSON array or object, the Rust mapper SHALL return a schema-valid explicit classification or refusal rather than reproduce a language-specific object representation or interpreter exception.
- When the retained Python implementation represents or rejects a structured identity differently, the differential qualification SHALL record the deliberate divergence.
- Reject empty, whitespace-bearing, non-printable, mutable-alias, range, and
  wildcard governing versions while accepting `x` or `X` inside immutable
  version metadata.
- Assign a new explicit version to a different canonicalization algorithm.
- Keep generated foreign-language fixtures as inert data from one canonical
  semantic source.
- Consume portable verification contracts by version rather than copying them
  into a second contract family.

## Error Conditions

Unknown versions, missing provenance, ambiguous mappings, digest mismatch,
malformed fixtures, invalid governing versions, unsupported canonicalization,
and non-finite or overflowing numeric inputs fail explicitly while preserving
the original input bytes. Each semantic failure exposes a typed refusal reason;
diagnostic prose is not the discriminator.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-015-CON-1 | Compatibility access SHALL be read-only. | Data Integrity | Test |
| FR-015-CON-2 | Generated foreign-language fixtures SHALL NOT be executed by qualification. | Security | Test |
| FR-015-CON-3 | The implementation SHALL NOT define a second persisted verification or evidence record family. | Responsibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-015-AC-1 | Over the shared supported input domain, Rust and retained reference implementations produce byte-identical canonical fixtures, identity digests, compatibility classifications, and bounded reports over the accepted corpus and focused fictional cases, including exact integers beyond `i64` and `u64`; focused structured-identity cases record the declared Rust/Python divergence. | Test (TC-100) |
| FR-015-AC-2 | Success, unavailable, not-computed, not-applicable, failed, inconclusive, malformed, stale, tampered, lossy, and unreadable remain distinguishable; typed state labels retain stable wire spellings and untyped labels require exactly one known state. | Test (TC-102) |
| FR-015-AC-3 | Invalid versions, provenance, mappings, fixtures, digests, and numeric identity inputs fail without an identity digest or source-byte change; immutable `x`/`X` metadata remains accepted; every exercised semantic and PGM-01 refusal exposes its expected typed reason without inspecting diagnostic prose. | Test (TC-103) |
| FR-015-AC-4 | Static ownership and execution audits find no copied portable contract family, persisted evidence family, or executable foreign-language fixture. | Test (TC-104) |

## Dependencies

- **Upstream**: FR-008 through FR-011, FR-014, and any accepted portable
  verification-contract release consumed by the implementation.
- **Downstream**: FR-016 and FR-017 reuse these types and classifiers.
