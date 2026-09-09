---
id: SR-058
title: "Base review of the assurance-contract registry model and census"
type: SpecReview
analysis: base
scope: "ADR-002 assurance-contract census; FR-015-AC-5..AC-7; TC-119, TC-120, TC-122; TASK-017"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---

## Summary

This focused QUOIN base review checks whether TASK-017 can represent the real
AssuranceProfile and MeasurementPlan population before Rust implementation.
The diagnostic census inspected the exact default-branch trees of all 294
non-archived repositories visible in the `agent-ix` organization on
2026-09-09. It found AP/MP instances in 13 repositories. Engineering
Assurance's own two package skeletons are a fourteenth classified repository
entry, but are not active consumer instances.

The scan is proposal evidence, not the release-scope authority. The committed
registry becomes authoritative only after the named human owner accepts its
population, classifications, and candidate revisions.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-118 | high | Closed in the specification: FR-015 required one unique row per repository while saying each row classified one artifact. Real repositories contain as many as 22 AP/MP artifacts, so the old model could only omit artifacts or violate its duplicate-repository rule. Repository entries now bind one candidate commit and contain one or more independently classified artifact entries with unique repository-relative paths. | FR-015 behavior and AC-7; TC-122; TASK-017 | wrong-requirement |
| FND-119 | medium | Closed in the specification: ADR-002's named current-shape census omitted `ix://agent-ix/quire-code-rs`, whose default branch contains `spec/assurance/MP-001-graph-quality-observation.md`. The named list now includes it. | ADR-002 assurance-contract census; `quire-code-rs@e1b7fc3` blob `213fbeed` | missing-requirement |
| FND-120 | high | Closed in the specification: “owner-reviewed” had no encoded acceptance state or attribution rule, allowing an agent-prepared or half-attributed registry to govern by assertion. FR-015 now permits only unattributed `pending` or human-attributed `accepted`, binds acceptance to the exact canonical population digest, and withholds snapshots for every other state. | FR-015 behavior, CON-4, CON-5, and AC-7; TC-122; TASK-017 | missing-requirement |
| FND-121 | high | Closed in the specification: a registry “digest over its canonical bytes” was self-referential once the digest and acceptance were fields in those bytes. Registry/v1 now defines a canonical `population` digest, owner acceptance references that digest, and each snapshot separately binds the exact registry input-byte digest to detect any within-run mutation. | FR-015 behavior and AC-7; TC-122 | wrong-requirement |

## Proposed owner registry population

The following population is the complete exact-tree result. Counts are artifact
entries, not repository rows. Unless the owner records another classification,
the proposed classification is `active` for these 53 instances.

| Canonical repository | Candidate commit | AP | MP |
| --- | --- | ---: | ---: |
| `ix://agent-ix/qa-corpus` | `820c0e76ff3b4395c6b52ccf863de51446fd5e37` | 1 | 2 |
| `ix://agent-ix/quire-analyze` | `5f37678928aea451447d1ee6d42deefb95cb2f5a` | 1 | 1 |
| `ix://agent-ix/quire-code-rs` | `e1b7fc303f3df3c9e3673358ce990525b7ff321e` | 0 | 1 |
| `ix://agent-ix/quire-contract-codegen` | `240fad84a9565ab723ba9844e18faea4e5d96f66` | 1 | 1 |
| `ix://agent-ix/quire-contract-ir` | `690bde7f2dc58662cf9ff0595c2c0e3b17107c6f` | 1 | 1 |
| `ix://agent-ix/quire-contract-runtime` | `7caefac3b7181d7665caa8f83c6694134fac63ae` | 1 | 1 |
| `ix://agent-ix/quire-rs` | `8b8020e665c61a11bc74f3f23b84617c80c0c442` | 1 | 8 |
| `ix://agent-ix/quire-verification` | `a57a88d9b259d7c48e95d375da0a8b01b3be7c0d` | 1 | 1 |
| `ix://agent-ix/quoin` | `7072d65db7e236e819d99be42e094ba548e9496a` | 1 | 21 |
| `ix://agent-ix/tl-mltl` | `9b3b68bc78589fa32805e105d9f8d97a0ad280b3` | 1 | 1 |
| `ix://agent-ix/tl-parse` | `103e6fd1573fa87534efec206f5d3e3b9c3f9d82` | 1 | 1 |
| `ix://agent-ix/tl-rewrite` | `1ccab45dc64d5a344740256c5a91c2e10555decc` | 1 | 1 |
| `ix://agent-ix/tl-syntax` | `26b801d4a68ebfe720062cfdb3c66b070ab60e92` | 1 | 1 |

The proposed `ix://agent-ix/engineering-assurance` repository entry binds the
TASK-017 implementation candidate and classifies exactly these two artifacts as
`skeleton`, owned by Engineering Assurance with reason `installed module
authoring template; not a governed consumer instance`:

- `engineering_assurance/skeletons/AssuranceProfile.md` (blob `e0188f0c...`)
- `engineering_assurance/skeletons/MeasurementPlan.md` (blob `d74500c0...`)

The installed provider candidate is module version `0.2.0`, manifest SHA-256
`80e4f2c8...fc31f72`, AssuranceProfile schema SHA-256
`070f8802...6cf3535`, and MeasurementPlan schema SHA-256
`aae266f2...2bcfd05a`.

## Gitlink accounting

- Engineering Assurance pins `ix://agent-ix/qa-corpus@4b390c29...` at `corpus`.
- `quire-rs@8b8020e6...` pins `ix://agent-ix/qa-corpus@7442f277...`
  at `corpus`.
- `quoin@7072d65d...` pins `ix://agent-ix/qa-corpus@7b81343e...`
  at `corpus`.

These are references to the same canonical repository at different commits,
not three additional consumer repositories. TASK-017 must bind each gitlink
reference and deduplicate only identical `(repository, commit, artifact blob)`
contract inputs; it must not collapse different revisions.

## Criterion review

- **Complete:** the cardinality model can represent every observed artifact,
  the census covers all 294 non-archived organization repositories rather than
  a workstation directory or a 200-result client subset, and owner review has
  a fail-closed acceptance state.
- **Clear:** repository identity/commit and artifact path/classification are
  separate levels; duplicate rules apply at their respective levels.
- **Consistent:** the model preserves one population authority and treats
  package-corpus gitlinks as references, not independent repository identities.
- **Testable:** TC-122 now names one/many/empty cardinality cases as well as
  duplicate repository and duplicate repository-relative path cases.
- **Traceable and necessary:** both corrections directly unblock FR-015-AC-5
  through AC-7 and TASK-017 without expanding Engineering Assurance ownership.

## Review disposition

**CONDITIONAL.** FND-118 through FND-121 are closed by the specification changes.
The proposed population and classifications still require explicit human-owner
acceptance, and TASK-017 implementation remains blocked on the independent
TASK-016 review. This review does not authorize #60 consumer migration.
