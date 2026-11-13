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

A fifth fixture, the **licence fixture** (added for `ix://agent-ix/quoin/FR-150`, PLAT-1671), is placed under
`scan/` and is an npm package with a `lockfileVersion` 3 lock file pinning five third-party entries. Four declare
a licence: one with a permissive SPDX identifier, one with a copyleft SPDX identifier, one with a second,
different permissive SPDX identifier (so a policy can allow the first, deny the copyleft one and list neither of the
others), and one with an SPDX expression of two identifiers. The fifth declares none. An expected-result file beside
it names, per entry purl, the CycloneDX field expected on the component (`id` or `expression`) and its text, or that
the component has no `licenses` entry. The fixture is fictional and pins no real package name that could be mistaken
for a vendored copy; a quoin test policy may ban any one of its purls.

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
- The licence fixture SHALL yield an artifact in which the component of each lock entry that declares a licence carries that licence in `licenses`, and the component of the entry that declares none carries no `licenses` entry.
- If no component of the licence fixture's artifact carries `licenses`, then the licence-fixture test SHALL fail.
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
| FR-039-AC-7 | Design review confirms Engineering Assurance contains no CycloneDX component reader, no quoin dependency and no quoin launch path in production code. Test code that checks the artifact in AC-1 to AC-4 and AC-8 is the only reader. | Inspection |
| FR-039-AC-8 | The licence fixture's retained artifact has the declared package count with `pkg:npm/` purls; for each purl the component carries the CycloneDX field (`license.id` or `expression`) and the text that the expected-result file names, and the entry that declares no licence has no `licenses` entry. | Test |

## Dependencies

- **Upstream**: [FR-038](./FR-038-syft-sbom-producer-request.md); `ix://agent-ix/quoin/FR-041` defines the adapter and its component-to-entry rule.
- **Downstream**: any assurance case that discharges an `sca-sbom` obligation through the consumer's quoin pipeline; `ix://agent-ix/quoin/FR-150`, whose tests consume the licence fixture's captured output.
