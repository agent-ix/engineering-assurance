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

Engineering Assurance SHALL implement compatibility classification, accepted
corpus access, evidence-state classification, verification-semantic validation,
bounded projections, and fixture generation in Rust without changing existing
observable identities or ownership.

## Inputs

- The accepted compatibility matrix and pinned corpus.
- Canonical semantic-reference and report fixtures.
- Producer outputs encoded as finite RFC 8259 JSON objects. Integer values may
  exceed the `i64` and `u64` ranges and remain exact; non-finite values and
  finite-number syntax that overflows the retained Python numeric domain are
  not accepted identity inputs.
- Historical PGM-01 v1/v2 records.
- Accepted portable verification-contract versions when available.
- Packaged AssuranceProfile and MeasurementPlan schemas/skeletons plus active
  artifacts from the pinned package corpus and recorded Quire consumers.
- An owner-reviewed, versioned registry whose unique canonical repository
  entries each contain the AP/MP artifact entries governed for that repository,
  and a candidate-revision snapshot of those artifacts.

## Outputs

- Versioned classifications, mappings, projections, and generated fixture bytes.
- A revision-bound assurance-contract compatibility report.
- Explicit errors for unknown, malformed, ambiguous, stale, or tampered inputs.

## Behavior

- The Rust implementation SHALL preserve every declared availability and
  non-success state without collapsing it to success.
- The Rust implementation SHALL preserve source-field references and declared
  lossy or unmapped limitations in historical views.
- The Rust implementation SHALL preserve the legacy canonical JSON bytes and
  digests for every identity domain that already depends on them.
- When Engineering Assurance prepares to mint an identity digest, the Rust
  implementation SHALL preserve the exact integer value and retained Python
  canonical spelling without conversion through a binary floating-point
  representation.
- If a producer output contains `NaN`, positive or negative infinity, or a
  numeric token whose retained Python interpretation is non-finite, then
  Engineering Assurance SHALL reject the output.
- When Engineering Assurance rejects a numeric identity input, the evidence
  result SHALL contain no identity digest.
- Parsed producer output MAY preserve numeric token distinctions for validation
  and diagnostics. Structural equality of parsed JSON values SHALL NOT define
  evidence identity; canonical output bytes and their digest define it.
- The Rust evidence API SHALL expose the stable wire spelling of each typed
  availability state, exact-one validation for state labels received through
  an untyped boundary, a direct result-validity predicate, and a validation
  error message accessor that does not require parsing formatted output.
- Each governing version identity SHALL use a non-empty token composed only of
  printable ASCII characters without whitespace.
- Engineering Assurance SHALL reject a mutable version alias, range operator,
  or `x` wildcard component while accepting an incidental lowercase or
  uppercase `x` inside immutable version metadata such as
  `1.2.3+linux-x86_64` or `1.2.3+X86`.
- The version-policy corpus SHALL retain distinct governed classes for leading
  or trailing ASCII space, interior ASCII space, lowercase `x` metadata, and
  uppercase `X` metadata so removal of any boundary fails qualification.
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
- Engineering Assurance SHALL resolve the assurance-contract consumer set from
  the committed owner registry, package metadata, and exact pinned gitlinks;
  it SHALL NOT infer release scope from arbitrary workstation directories.
- The consumer registry SHALL contain external consumer and compatibility-corpus
  repositories. It SHALL NOT contain the Engineering Assurance provider
  repository that stores the registry.
- The compatibility snapshot SHALL bind the clean Engineering Assurance
  provider candidate revision, module version, manifest, AP/MP schemas, and
  AP/MP skeletons separately from the consumer registry.
- The consumer-registry envelope SHALL carry the
  `engineering-assurance.consumer-registry/v1` discriminator, a `population`
  array, and `population_sha256`. Registry/v1 canonical population bytes SHALL
  be the UTF-8 compact JSON encoding of `population` with object keys sorted
  lexicographically, repository entries sorted by canonical identity, artifact
  entries sorted by repository-relative path, and no trailing newline.
  `population_sha256` SHALL be lowercase SHA-256 over exactly those bytes.
- Each population member SHALL be one unique repository entry for a canonical
  `ix://<org>/<repo>` identity. Each repository entry SHALL contain one or more
  artifact entries. The stable registry SHALL NOT bind a repository commit or
  artifact-content digest.
  Engineering Assurance SHALL reject a second repository entry for the same
  identity even when its contents match.
- Engineering Assurance SHALL record registry owner review as either `pending`
  with no reviewer attribution or `accepted` with reviewer kind `human`, a
  named reviewer, an acceptance date, and the exact accepted
  `population_sha256`.
- When registry owner review is pending, half-attributed, agent-attributed, or
  bound to another population digest, Engineering Assurance SHALL NOT use the
  registry to govern a compatibility snapshot.
- When the named human explicitly accepts the exact population digest,
  Engineering Assurance MAY transcribe that acceptance.
- Each artifact entry SHALL have a path unique within its repository entry.
- Each artifact entry SHALL classify the artifact as exactly one of `active`,
  `inactive`, `skeleton`, `template`, or `quarantined`.
- Engineering Assurance SHALL reject a second artifact entry for the same
  repository-relative path even when its classification matches.
- Each non-active artifact entry SHALL name its owner and exclusion reason.
- The compatibility snapshot SHALL record the SHA-256 of the exact registry
  input bytes in addition to `population_sha256`, so any formatting or metadata
  mutation during one run is observable.
- For each registry repository entry, the compatibility snapshot SHALL record
  one clean candidate commit and the canonical repository identity.
- For each registry artifact entry, the compatibility snapshot SHALL record the
  artifact path and content digest, provider/module version and schema digests,
  validation outcome, and owner disposition.
- Engineering Assurance SHALL report repeated submodule checkouts of the same
  repository commit and artifact blob as references to one contract input
  rather than independent consumers.
- Engineering Assurance SHALL exclude skeletons, templates, quarantined
  evidence, and inactive branches only through an explicit recorded
  classification.
- Engineering Assurance SHALL NOT replace an accepted schema with an
  incompatible shape until every recorded active consumer has an owner-approved
  migration disposition and the prior shape remains explicitly addressable.
- Engineering Assurance SHALL reject a Quire host result that applies review
  selection or another governance effect from an AssuranceProfile that failed
  its installed versioned schema.
- Engineering Assurance SHALL report that host-contract violation without
  treating the profile as governing input. Any change to Quire's own behavior
  requires a separately accepted Quire specification.

## Error Conditions

An unknown contract version, missing provenance, ambiguous legacy mapping,
digest mismatch, malformed fixture, invalid or mutable governing-version token,
non-finite or numeric-overflow producer output, unsupported canonicalization version,
unresolved registry entry, dirty candidate revision, and unclassified artifact
each fail explicitly and preserve the original input bytes. A registry digest
change during a run, duplicate repository identity, duplicate artifact path, or
unknown classification also fails the snapshot without an owner disposition.
An installed module
schema that rejects an active artifact in the accepted compatibility corpus is
a release-blocking incompatibility, not an ordinary malformed-input case.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-015-CON-1 | Compatibility access SHALL be read-only. | Data Integrity | Test |
| FR-015-CON-2 | Generated foreign-language fixtures SHALL NOT be executed by qualification. | Security | Test |
| FR-015-CON-3 | The implementation SHALL NOT define a second persisted verification or evidence record family. | Responsibility | Test |
| FR-015-CON-4 | An agent SHALL NOT accept a consumer registry. | Responsibility | Test |
| FR-015-CON-5 | Engineering Assurance MAY prepare a pending consumer registry for human review. | Responsibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-015-AC-1 | The Rust and retained reference implementations produce byte-identical canonical fixtures, identity digests, compatibility classifications, and bounded reports over the accepted corpus and focused fictional cases, including a deterministic generated JSON-value corpus with signed and unsigned integers beyond `i64` and `u64`. | Test (TC-100) |
| FR-015-AC-2 | Every success, unavailable, not-computed, not-applicable, failed, inconclusive, malformed, stale, tampered, lossy, and unreadable case remains distinguishable after migration; the Rust API exposes stable wire spellings and rejects zero, duplicate, conflicting, or unknown untyped state labels. | Test (TC-102) |
| FR-015-AC-3 | Unknown versions, missing provenance, ambiguous mappings, malformed fixtures, digest mismatches, leading, trailing, or interior ASCII whitespace, non-printable or non-ASCII version tokens, true wildcard/range versions, non-finite numeric tokens, and finite-number syntax that overflows the retained numeric domain fail explicitly without an identity digest while source and corpus bytes remain unchanged; immutable metadata containing lowercase `x` or uppercase `X` remains accepted, every named version-policy boundary is independently represented in the corpus, and callers can inspect validity and the validation error message without parsing formatted output. | Test (TC-103) |
| FR-015-AC-4 | Static ownership and execution audits find no copied portable contract family, persisted evidence family, or executable foreign-language fixture (CON-2, CON-3). | Test (TC-104) |
| FR-015-AC-5 | A registry-derived, revision-bound snapshot separately binds the Engineering Assurance provider candidate and accounts for every active AssuranceProfile and MeasurementPlan in the pinned package corpus and registered external consumer set; the candidate installed module validates each artifact, or its owner has completed an explicit versioned migration before module replacement. | Test (TC-119) |
| FR-015-AC-6 | Legacy, current, malformed, and unsupported assurance-artifact shapes receive distinct versioned outcomes without changing source bytes, and an invalid AssuranceProfile contributes no review-selection decision. | Property (TC-120) |
| FR-015-AC-7 | The versioned registry represents multiple artifact entries under one unique repository entry and rejects an unknown discriminator/classification, empty artifact list, non-canonical ordering, non-canonical or duplicate repository identity, duplicate repository-relative artifact path, population-digest mismatch, any exact registry-byte change during a snapshot, and pending, half-attributed, agent-attributed, or wrong-population owner review; every accepted exclusion has a named owner and reason, and accepted review names the human and date (CON-4, CON-5). | Property (TC-122) |

## Dependencies

- **Upstream**: FR-008 through FR-011, FR-014, and the accepted portable
  verification-contract release consumed by the implementation.
- **Downstream**: FR-016 and FR-017 reuse these types and classifiers.
