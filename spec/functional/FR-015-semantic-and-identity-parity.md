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
- Historical structured-identity inputs SHALL produce the declared Rust classification or refusal without depending on another implementation at qualification time.
- Reject empty, whitespace-bearing, non-printable, mutable-alias, range, and
  wildcard governing versions while accepting `x` or `X` inside immutable
  version metadata.
- Assign a new explicit version to a different canonicalization algorithm.
- Keep generated foreign-language fixtures as inert data from one canonical
  semantic source.
- Read the accepted compatibility corpus only through one explicit
  caller-selected corpus root, and SHALL NOT discover a root from the ambient
  environment.
- Re-hash every retained corpus artifact against the identity its index records
  before that artifact's bytes are used, and refuse the artifact on any
  mismatch.
- Refuse a `referenced` retention rather than reading bytes for it, because a
  referenced artifact is pinned by digest and deliberately not retained.
- Refuse an absolute retained path, a parent-traversing retained path, a
  symlinked or non-regular retained entry, a missing retained entry, a duplicate
  case or chain-role identity, an unknown corpus version, a malformed corpus
  index, and tampered retained bytes, each as a distinct typed refusal.
- If an accepted-corpus input exceeds a declared ceiling on index bytes,
  retained artifact bytes, retained path bytes, entry population, or directory
  nesting depth, then the accepted-corpus reader SHALL refuse that input as a
  typed error before reading or allocating beyond the ceiling.
- Consume portable verification contracts by version rather than copying them
  into a second contract family.

## Error Conditions

Unknown versions, missing provenance, ambiguous mappings, digest mismatch,
malformed fixtures, invalid governing versions, unsupported canonicalization,
and non-finite or overflowing numeric inputs fail explicitly while preserving
the original input bytes. Each semantic failure exposes a typed refusal reason;
diagnostic prose is not the discriminator. An incomplete corpus retention is six
distinguishable failures, not one: a retained producer case naming no path, a
retained producer case recording no digest, a referenced producer case carrying
a path it must not carry, a referenced producer case carrying a digest it must
not carry, a retained chain artifact naming no path, and a retained chain
artifact recording no digest. Each SHALL carry its own typed reason, so a caller
can separate a transcription slip from a corpus-integrity violation without
reading prose.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-015-CON-1 | Compatibility access SHALL be read-only. | Data Integrity | Test |
| FR-015-CON-2 | Generated foreign-language fixtures SHALL NOT be executed by qualification. | Security | Test |
| FR-015-CON-3 | The implementation SHALL NOT define a second persisted verification or evidence record family. | Responsibility | Test |
| FR-015-CON-4 | The accepted-corpus reader SHALL refuse, as a typed error and before allocating for it, any input beyond its declared ceiling on index bytes, retained artifact bytes, retained path bytes, entry population, or directory nesting depth. | Resource | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-015-AC-1 | Over the accepted corpus and focused fictional cases, Rust produces the declared byte-identical canonical fixtures, identity digests, compatibility classifications, and bounded reports, including exact integers beyond `i64` and `u64`; structured-identity inputs produce their declared classification or refusal. | Test (TC-100) |
| FR-015-AC-2 | Success, unavailable, not-computed, not-applicable, failed, inconclusive, malformed, stale, tampered, lossy, and unreadable remain distinguishable; typed state labels retain stable wire spellings and untyped labels require exactly one known state. | Test (TC-102) |
| FR-015-AC-3 | Invalid versions, provenance, mappings, fixtures, digests, and numeric identity inputs fail without an identity digest or source-byte change; immutable `x`/`X` metadata remains accepted; every exercised semantic and PGM-01 refusal exposes its expected typed reason without inspecting diagnostic prose, and each of the six distinguishable incomplete-retention failures carries a typed reason distinct from the other five. | Test (TC-103) |
| FR-015-AC-4 | Static ownership and execution audits find no copied portable contract family, persisted evidence family, or executable foreign-language fixture. | Test (TC-104) |
| FR-015-AC-5 | Native accepted-corpus access reads every retained artifact from one explicit corpus root, reproduces each recorded identity, covers every required corpus state, and refuses referenced retentions, absolute or parent-traversing paths, symlinked or non-regular entries, missing entries, duplicate identities, unknown corpus versions, malformed indexes, and tampered bytes without writing a byte. | Test (TC-069, TC-070, TC-071, TC-072, TC-073, TC-074, TC-075, TC-076, TC-078) |

## Dependencies

- **Upstream**: FR-008 through FR-011, FR-014, and any accepted portable
  verification-contract release consumed by the implementation.
- **Downstream**: FR-016 and FR-017 reuse these types and classifiers.
