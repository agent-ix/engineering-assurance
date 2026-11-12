---
id: FR-039
title: "Ingest Syft CycloneDX output through the existing SBOM evidence adapter"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-038"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-041"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-006"
    type: "constrains"
---

# FR-039: Ingest Syft CycloneDX output through the existing SBOM evidence adapter

## Description

When the [FR-038](./FR-038-syft-sbom-producer-request.md) run is `completed`,
Engineering Assurance SHALL hand the retained CycloneDX JSON artifact to the
Quoin SBOM adapter defined by `ix://agent-ix/quoin/FR-041`, so that each
component becomes one run entry and Engineering Assurance contains no
CycloneDX parser of its own.

## Inputs

- The retained `sbom` artifact of a `completed` FR-038 result.
- Fictional fixtures, authored in this repository, each with a known component
  count: a Rust crate with a lock file, an npm package with a lock file, a
  Python project with a pinned requirements file, and a container image
  archive holding a small package database.

## Outputs

- The entries the Quoin SBOM adapter returns for the artifact, one per
  component, identified by purl as FR-041 defines.

## Behavior

- Engineering Assurance SHALL pass the artifact bytes to the Quoin adapter
  selected by `--adapter sbom` and SHALL NOT parse components, purls or counts
  itself.
- For each of the four fixtures, the number of entries SHALL equal the number of
  packages the fixture declares.
- If the result state is not `completed`, then Engineering Assurance SHALL NOT
  call the adapter and SHALL report the FR-019 state unchanged.
- If the adapter refuses the document, then Engineering Assurance SHALL report
  that refusal and SHALL NOT treat the run as an empty inventory (FR-041-AC-4).
- Engineering Assurance SHALL NOT copy Quoin source or specification text into
  this repository.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-039-AC-1 | Running the Syft producer over the Rust fixture and ingesting the artifact yields entries whose count equals the fixture's declared package count, each identified by a `pkg:cargo/` purl. | Test |
| FR-039-AC-2 | The same over the npm fixture yields the declared package count with `pkg:npm/` purls. | Test |
| FR-039-AC-3 | The same over the Python fixture yields the declared package count with `pkg:pypi/` purls. | Test |
| FR-039-AC-4 | The same over the container image archive fixture yields a non-empty set of entries whose count equals the fixture's declared package count. | Test |
| FR-039-AC-5 | No source file in this repository defines a CycloneDX component reader; ingestion calls the Quoin adapter only. | Inspection |
| FR-039-AC-6 | A non-`completed` FR-038 result (`unavailable`, `failed`, `timed_out`) reaches ingestion as that state with no adapter call and no entries. | Test |
| FR-039-AC-7 | A `completed` result whose artifact is not a CycloneDX or SPDX document is reported as an adapter refusal, not as zero entries. | Test |

## Dependencies

- **Upstream**: [FR-038](./FR-038-syft-sbom-producer-request.md); Quoin `ix://agent-ix/quoin/FR-041` for the adapter and its component-to-entry rule.
- **Downstream**: any assurance case that discharges an `sca-sbom` obligation.
