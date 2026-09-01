# ADR-001: Verification semantics ownership and type fit

Status: accepted for specification and compatibility work

## Decision

Engineering Assurance owns the vocabulary that distinguishes a verification
definition, an execution, a check result, retained evidence, a measurement,
a diagnostic, and a report. It does not own a second persistence format for
those concepts.

The implemented types fit that vocabulary as follows:

| Semantic concept | Authoritative owner and type | Role |
| --- | --- | --- |
| Static definition and obligation | Quire `assurance-v1` obligation, symbol, relation, and locator facts | Defines what is to be checked and where it came from. |
| Verification execution | Native producer result plus Quoin `ProofAttestation` or evidence-adapter intake | Records that a declared producer invocation occurred; Quire and Quoin never invoke it. |
| Check result | Native structured result transcribed into a Quoin `RunEntry`, `FindingRecord`, measurement observation, or proof-attestation result | Preserves the producer's result without inventing an overall verdict. |
| Evidence | Quoin retained output, evidence-store record, audit report, and attestation | Preserves exact bytes, bindings, integrity, availability, and audit state. |
| Measurement | Engineering Assurance `MeasurementPlan`; Quoin `MeasurementCollection` and `MeasurementObservation` | Keeps the authored definition separate from an observation made under a producer tuple. |
| Diagnostic | Quire diagnostic or native producer diagnostic retained by Quoin when material | Explains inability, malformed input, or a finding; it is not evidence of success. |
| Report | Quoin report view and `VerificationReceipt`, with ix-flow decision references | Presents claims, evidence, counterevidence, gaps, owner, and action without a trust score. |
| Human decision | ix-flow retained decision event | Records approval, rejection, or revision against an exact subject; tools do not infer it. |

## Ownership consequences

- Engineering Assurance may publish schemas for authored definitions and for
  non-persisted interoperability fixtures. It must reference Quire, Quoin, and
  ix-flow records by identity and version instead of copying their fields into
  a new envelope.
- Quire parses and exports static facts only. It does not execute tests, proof
  tools, solvers, or consumer commands.
- Quoin validates, transcribes, retains, audits, and reports explicit inputs.
  It does not execute a producer and does not decide overall sufficiency.
- Native project tools own execution, domain result schemas, oracles, corpora,
  and domain-specific failure behavior.
- ix-flow owns human decision state and event history.

## Compatibility decision

PGM-01 v1/v2 history is immutable. Compatibility is a read-only mapping that
returns source-field references, mapped authoritative targets, explicit lossy
or unmapped fields, and an `unreadable` outcome for ambiguous records. It never
rewrites a legacy directory or synthesizes a pass, producer identity, retained
output identity, or human decision.

## Reconciled upstream work

- `quire-rs#384` supplies stable static assurance exports and shared graph facts.
- `quoin#267` supplies plan-governed operational and experiment observations.
- `quoin#281` supplies history-preserving measurement and portfolio views.
- `quoin#282` supplies change records, proof attestations, integrity checks, and
  verification receipts.

No upstream record family is reparented or superseded by this decision.

