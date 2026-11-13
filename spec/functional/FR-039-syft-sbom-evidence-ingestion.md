---
id: FR-039
title: "Retain the Syft CycloneDX artifact as the input the Quoin SBOM adapter accepts"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-038"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-041"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-150"
    type: "references"
---

# FR-039: Retain the Syft CycloneDX artifact as the input the Quoin SBOM adapter accepts

## Description

Engineering Assurance SHALL end its SBOM work at the retained CycloneDX artifact
of a `completed` [FR-038](./FR-038-syft-sbom-producer-request.md) run. Ingestion
as evidence is done by the consumer's own pipeline through
`quoin evidence record --adapter sbom`, the adapter defined by
`ix://agent-ix/quoin/FR-041`; quoin does not generate the SBOM and Engineering
Assurance does not ingest it (FR-041-CON-1). Engineering Assurance therefore has
no CycloneDX parser, no quoin dependency and no launch path for quoin. The test
that the adapter turns the artifact into one entry per component belongs to the
quoin ticket PLAT-1430, not here.

## Inputs

Fictional fixtures authored in this repository, each placed under `scan/`:
a Rust crate with a lock file, an npm package with a lock file, a Python project
with a pinned requirements file, and a container image archive holding a small
package database and no `os-release` file.

A fifth fixture, the **licence fixture** (added for `ix://agent-ix/quoin/FR-150`, PLAT-1671), is an npm package
with a lock file whose pinned third-party entries declare licences: one entry with a permissive SPDX identifier,
one with a copyleft SPDX identifier, one with an SPDX expression of two identifiers, and one with no `license`
field. An expected-result file beside it names each entry's purl and the licence value its lock entry declares.
The fixture is fictional and pins no real package name that could be mistaken for a vendored copy.

## Outputs

The retained `sbom` artifact of the FR-038 result, unchanged.

## Behavior

- The fixture's **declared package count** is the number of third-party packages
  its manifest or lock file pins. The root crate, the npm root and the Python
  project itself are not counted. Dev dependencies are not present in the
  fixtures. Workspace members are not used.
- The container fixture has no `os-release`, so the artifact has no
  `operating-system` component, and `SYFT_FILE_METADATA_SELECTION=none` (FR-038)
  keeps `file` components out.
- Each fixture SHALL yield an artifact whose top-level `components` array holds
  exactly the declared package count and no nested components, because the quoin
  adapter reads the top-level array only.
- The licence fixture SHALL yield an artifact whose top-level `components` array holds exactly its declared package count, and whose components carry `licenses` entries only for the lock entries that declare a licence.
- The test SHALL fail if every component of the licence fixture's artifact lacks `licenses`, because the fixture exists to supply licence-bearing Syft output.
- Engineering Assurance SHALL NOT launch quoin, link quoin or read components in
  production code.
- If the FR-038 result state is not `completed`, then Engineering Assurance SHALL
  report that state unchanged and SHALL NOT present an artifact.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-039-AC-1 | The Rust fixture's retained artifact has a top-level `components` array of the declared package count, each with a `pkg:cargo/` purl. | Test |
| FR-039-AC-2 | The npm fixture's artifact has the declared package count with `pkg:npm/` purls. | Test |
| FR-039-AC-3 | The Python fixture's artifact has the declared package count with `pkg:pypi/` purls. | Test |
| FR-039-AC-4 | The container fixture's artifact has the declared package count, each component of type `library` and named for a declared package, no `operating-system` component and no `file` component. | Test |
| FR-039-AC-5 | The Syft-dependent tests obtain Syft as FR-038-CON-1 states; a missing Syft fails them. | Test |
| FR-039-AC-6 | A non-`completed` FR-038 result (`unavailable`, `failed`, `timed_out`) is reported as that state with no artifact presented. | Test |
| FR-039-AC-7 | Design review confirms Engineering Assurance contains no CycloneDX component reader, no quoin dependency and no quoin launch path in production code. Test code that counts the array in AC-1 to AC-4 is the only reader. | Inspection |
| FR-039-AC-8 | The licence fixture's retained artifact has the declared package count with `pkg:npm/` purls; the components of the entries that declare a licence carry a `licenses` entry whose value matches the expected-result file; the entry that declares none carries no `licenses` entry; and at least one component carries a `licenses` entry. | Test |

## Dependencies

- **Upstream**: [FR-038](./FR-038-syft-sbom-producer-request.md); `ix://agent-ix/quoin/FR-041` defines the adapter and its component-to-entry rule.
- **Downstream**: any assurance case that discharges an `sca-sbom` obligation through the consumer's quoin pipeline.
