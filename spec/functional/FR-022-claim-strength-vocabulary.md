---
id: FR-022
title: "Own an unordered claim-strength vocabulary"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-005"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-008"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "requires"
---

# FR-022: Own an unordered claim-strength vocabulary

## Description

Engineering Assurance SHALL own and publish a closed claim-strength vocabulary
that names the kind of support behind one advanced verification result, as part
of the verification-semantics vocabulary it owns under ADR-001.

## Inputs

- A claim-strength label supplied by a producer, a transcribed record, or a
  consumer of the shared vocabulary.

## Outputs

- The Rust type `engineering_assurance::claim_strength::ClaimStrength`, with
  one variant per strength, a listing of every strength, the exact wire
  spelling of each strength, text rendering, and parsing.
- A `claim_strength` entry in the verification-semantics ownership registry
  naming the owner, the authoritative type, the cardinality, the absence of an
  order, and the exact wire spellings.
- Inert Python, TypeScript, and Rust listings of the wire spellings, generated
  from the Rust type.

## Behavior

- The vocabulary SHALL contain exactly four strengths, spelled on the wire as
  `proven`, `bounded-checked`, `tested`, and `observed`:
  - `proven`: the claim is established by a proof over every case it covers;
  - `bounded-checked`: the claim was checked exhaustively within a stated bound;
  - `tested`: the claim was exercised by tests over a selected set of cases;
  - `observed`: the claim was seen to hold in observed behavior.
- Every advanced verification result SHALL carry exactly one strength.
- The strengths are unordered. The type SHALL expose no ordering, comparison,
  ranking, declared numeric representation, or default strength, and no
  operation SHALL promote a result from one strength to another. The listing
  order exists only for stable iteration and rendering; discriminants are not
  part of the contract.
- Rendering and serialization SHALL produce the exact wire spelling. Parsing and
  deserialization SHALL accept only an exact wire spelling.
- The type SHALL be reachable through a `claim-strength` Cargo feature that
  depends on `serde` alone and is included in the `full` feature.
- Other qualifiers of a result are not claim strengths and are not part of
  this vocabulary.

## Error Conditions

A label that is not exactly one of the four wire spellings, including a
case, separator, or whitespace variant and any value outside the vocabulary, is
refused with a typed error that carries the refused label. An ownership registry
that misdeclares the owner, type, cardinality, ordering, or values is refused
with a typed claim-strength reason.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-022-CON-1 | The `claim-strength` feature SHALL NOT activate `serde_json` or any other optional dependency beyond `serde`. | Dependency | Test (TC-153) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-022-AC-1 | Each of the four strengths renders, serializes, parses, and deserializes as exactly its declared wire spelling. | Test (TC-150) |
| FR-022-AC-2 | Every label outside the four wire spellings, including case, separator, and whitespace variants, is refused with a typed error carrying the refused label. | Test (TC-151) |
| FR-022-AC-3 | The listing of strengths names every variant exactly once, and a consumer cannot compare, order, rank, or default a strength. | Test (TC-152) |
| FR-022-AC-4 | A minimal consumer compiles with default features disabled and only `claim-strength` enabled, and resolves no `serde_json`. | Test (TC-153) |
| FR-022-AC-5 | The ownership registry and the generated Python, TypeScript, and Rust listings publish exactly the four wire spellings, the registry declares Engineering Assurance ownership, one strength per advanced result, and no order, and each misdeclaration is refused. | Test (TC-150) |

## Dependencies

- **Upstream**: FR-008 (the verification-semantics ownership registry) and
  FR-014 (the versioned Rust library boundary).
- **Downstream**: consumers that record or transcribe the strength of an
  advanced result import this type rather than defining their own.
