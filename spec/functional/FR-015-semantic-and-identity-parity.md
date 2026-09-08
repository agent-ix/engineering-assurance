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
---

# FR-015: Preserve semantic, identity, and compatibility behavior in Rust

## Description

Engineering Assurance SHALL implement compatibility classification, accepted
corpus access, evidence-state classification, verification-semantic validation,
bounded projections, and fixture generation in Rust without changing existing
observable identities or ownership.

## Inputs

- The accepted compatibility matrix and pinned corpus.
- Canonical semantic-reference and report fixtures.
- Historical PGM-01 v1/v2 records.
- Accepted portable verification-contract versions when available.
- Packaged AssuranceProfile and MeasurementPlan schemas/skeletons plus active
  artifacts from the pinned package corpus and recorded Quire consumers.

## Outputs

- Versioned classifications, mappings, projections, and generated fixture bytes.
- Explicit errors for unknown, malformed, ambiguous, stale, or tampered inputs.

## Behavior

- The Rust implementation SHALL preserve every declared availability and
  non-success state without collapsing it to success.
- The Rust implementation SHALL preserve source-field references and declared
  lossy or unmapped limitations in historical views.
- The Rust implementation SHALL preserve the legacy canonical JSON bytes and
  digests for every identity domain that already depends on them.
- Engineering Assurance SHALL assign a new explicit version to a different
  canonicalization algorithm.
- Engineering Assurance SHALL NOT use a different canonicalization algorithm
  to rewrite or silently reinterpret an existing identity.
- Generated foreign-language fixtures SHALL be inert data derived from one
  canonical semantic source.
- Portable verification contracts SHALL be consumed by version rather than
  copied into a second Engineering Assurance contract family.
- Engineering Assurance SHALL treat each published assurance-artifact schema
  as a versioned cross-repository contract.
- Engineering Assurance SHALL NOT replace an accepted schema with an
  incompatible shape until every recorded active consumer has an owner-approved
  migration disposition and the prior shape remains explicitly addressable.
- Quire SHALL NOT apply review selection or another governance effect from an
  AssuranceProfile that fails its installed versioned schema.

## Error Conditions

An unknown contract version, missing provenance, ambiguous legacy mapping,
digest mismatch, malformed fixture, and unsupported canonicalization version
each fail explicitly and preserve the original input bytes. An installed module
schema that rejects an active artifact in the accepted compatibility corpus is
a release-blocking incompatibility, not an ordinary malformed-input case.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-015-CON-1 | Compatibility access SHALL be read-only. | Data Integrity | Test |
| FR-015-CON-2 | Generated foreign-language fixtures SHALL NOT be executed by qualification. | Security | Test |
| FR-015-CON-3 | The implementation SHALL NOT define a second persisted verification or evidence record family. | Responsibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-015-AC-1 | The Rust and retained reference implementations produce byte-identical canonical fixtures, identity digests, compatibility classifications, and bounded reports over the accepted corpus and focused fictional cases. | Property (TC-100) |
| FR-015-AC-2 | Every success, unavailable, not-computed, not-applicable, failed, inconclusive, malformed, stale, tampered, lossy, and unreadable case remains distinguishable after migration. | Property (TC-102) |
| FR-015-AC-3 | Unknown versions, missing provenance, ambiguous mappings, malformed fixtures, and digest mismatches fail explicitly while source and corpus bytes remain unchanged. | Property (TC-103) |
| FR-015-AC-4 | Static ownership and execution audits find no copied portable contract family, persisted evidence family, or executable foreign-language fixture (CON-2, CON-3). | Test (TC-104) |
| FR-015-AC-5 | The candidate installed module validates every active AssuranceProfile and MeasurementPlan in the pinned package corpus and recorded Quire consumer set, or the artifact owner has completed an explicit versioned migration before the module replacement. | Test (TC-119) |
| FR-015-AC-6 | Legacy, current, malformed, and unsupported assurance-artifact shapes receive distinct versioned outcomes without changing source bytes, and an invalid AssuranceProfile contributes no review-selection decision. | Property (TC-120) |

## Dependencies

- **Upstream**: FR-008 through FR-011, FR-014, and the accepted portable
  verification-contract release consumed by the implementation.
- **Downstream**: FR-016 and FR-017 reuse these types and classifiers.
