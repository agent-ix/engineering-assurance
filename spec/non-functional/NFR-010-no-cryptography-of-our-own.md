---
id: NFR-010
title: "Engineering Assurance implements no cryptography of its own"
type: NFR
quality_attribute: security
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-058"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-064"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-056"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-057"
    type: "constrains"
---

# NFR-010: Engineering Assurance implements no cryptography of its own

## Statement

Engineering Assurance shall create and verify every statement and record signature through Sigstore signing and
verification libraries and in-toto statement types.

## Scope

- Applies to: the approval statement of FR-058, its verification (FR-064), and the signature member of the public
  record form (FR-056). Engineering Assurance owns this code; Quoin runs no git and no network call, and receives results as data.

## Rationale

Signature and certificate rules are security-critical and already specified by the delegated libraries. Own code would
add an unreviewed implementation. A keyless signature uploaded to a public log discloses the signing repository and
workflow, so the upload is part of publishing and is gated like it. The git-signature path keeps its own delegation under FR-036 and FR-040.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Count of signature, certificate or hash-based signing primitives implemented in the approval and sink modules | 0 | 0 | Source inspection |
| Count of public-log uploads made without a passing FR-057 check | 0 | 0 | Test with a stand-in public log client |
| Count of statements or record signatures accepted whose verification did not run through the delegated library | 0 | 0 | Test with a stand-in library |

## Verification

A source inspection checks the first count; a stand-in public log client checks the second (FR-057-AC-10); a test that substitutes the delegated library with a stand-in that rejects
every signature checks the third.

## Dependencies

- **Upstream**: none.
- **Downstream**: [FR-058](../functional/FR-058-approval-statement-and-predicate.md),
  [FR-064](../functional/FR-064-verify-approval-statement-with-workflow-identity-policy.md).
