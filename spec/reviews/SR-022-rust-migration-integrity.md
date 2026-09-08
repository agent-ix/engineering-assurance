---
id: SR-022
title: "Integrity review of the Rust-native Engineering Assurance migration"
type: SpecReview
analysis: integrity
scope: "StR-003, ADR-002, FR-014..FR-018, NFR-005"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---

## Summary

The authoritative owner boundaries and canonicalization versioning are strong.
Integrity is still conditional because the active validator resolves duplicate
module/archetype providers by first-wins, so a clean result does not itself prove
which Engineering Assurance contract bytes were applied.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-054 | high | Validation emits `DuplicateModuleName: engineering-assurance` and duplicate AssuranceProfile/MeasurementPlan archetypes with first-wins behavior. Until review evidence selects one provider root and binds its manifest/schema digests, 55/55 grammar-clean does not establish that the candidate was checked against the intended module bytes. | ADR-002:78-105; FR-015:63-65; Quire 0.31.0 validation output | correct-requirement-no-evidence |
| FND-055 | medium | The registry-derived snapshot records artifact and schema digests, but no acceptance criterion requires the registry document itself to be canonicalized, digest-bound, immutable during a run, or rejected when it changes between census and disposition. | FR-015-AC-5; FR-018:43-55; TC-119 | missing-requirement |

## Integrity chain

The intended chain is registry bytes → registry digest → canonical consumer
identity → clean commit → artifact path/blob → provider/module/schema digest →
validation outcome → owner disposition. FND-054 and FND-055 are the two missing
bindings.

## Repeat-review dispositions

| ID | Disposition |
| --- | --- |
| FND-054 | Fixed for this review: an isolated copy of the exact candidate `spec/` validated 63/63 against the single installed Engineering Assurance provider, and its manifest/AP-schema/MP-schema SHA-256 values equal the repository bytes (`80e4…f72`, `070f…535`, `aae2…05a`). The ordinary repository-root invocation still reports ambient duplicate discovery and is not the bound result. |
| FND-055 | Fixed in the specification: FR-015-AC-7 and TC-122 bind registry/v1 canonical bytes and reject a digest or byte change during the snapshot. |
