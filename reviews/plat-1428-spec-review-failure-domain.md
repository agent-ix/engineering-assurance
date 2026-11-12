---
id: SR-16410
title: "Failure-domain review of the Syft SBOM producer specification (PLAT-1428)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/engineering-assurance; spec/functional/FR-038-syft-sbom-producer-request.md, spec/functional/FR-039-syft-sbom-evidence-ingestion.md (PR #212 diff vs origin/main)"
review_set: subset
---

## Summary

Ticket: PLAT-1428. Unstated failure modes in the Syft request shape and the
component-count criteria. Checked against the quoin SBOM adapter
(`quoin-evidence/src/adapters/sbom.rs`, which emits one entry per top-level
`components[]` element of any `type`, nested components not counted), the
FR-019 executor (`src/producer_execution.rs:1475-1517`: working directory is
the capability root, `input_artifact` arguments resolve to
`/proc/<pid>/fd/<n>`, `output_artifact` arguments to the relative path), and
the Syft configuration reference (`file.metadata.selection` default
`owned-by-package`, env `SYFT_FILE_METADATA_SELECTION`).

Clean units: the FR-038 non-completion states (`unavailable`, `failed`,
`timed_out`) and the stale-output rule (FR-038-AC-6) follow FR-019 exactly;
FR-039-AC-6 and FR-039-AC-7 keep a refused or non-completed run from reading as
an empty inventory.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-039-AC-4 cannot hold as written. Syft's default `file.metadata.selection: owned-by-package` emits `file` components for an image scan, and an image SBOM also carries an `operating-system` component; the quoin adapter counts every top-level component, and FR-039 forbids Engineering Assurance from filtering. Fix in FR-038: set `SYFT_FILE_METADATA_SELECTION=none`, and state whether the OS component is part of the declared count. Resolves the author's open "container scans emit os/file components" gap. | spec/functional/FR-039-syft-sbom-evidence-ingestion.md:58 |
| FND-002 | medium | "The number of packages the fixture declares" is undefined for the Rust, npm and Python fixtures: whether the root crate in `Cargo.lock`, the npm root `""` entry, workspace members and dev dependencies count. The adapter also counts top-level components only; the spec should state the fixtures produce no nested components (the author's open gap). | spec/functional/FR-039-syft-sbom-evidence-ingestion.md:55-57 |
| FND-003 | medium | The capability root holds the scan target, the SBOM output and the Syft cache, and it is also the working directory. A `dir:` selector covering the root makes Syft catalogue its own cache and earlier output, and a `.syft.yaml` in the root is read as configuration. The spec should require the SBOM path and cache outside the scanned subtree and say how repository Syft config is kept out. | spec/functional/FR-038-syft-sbom-producer-request.md:37-39 |
| FND-004 | medium | The image archive is declared in `inputs` but scanned through the literal `oci-archive:<relative path>`, so Syft reads the live file, not the FR-019 retained snapshot (descriptor arguments resolve to `/proc/<pid>/fd/<n>` and cannot carry the scheme prefix). The declaration only costs a copy of up to 1 GiB. State what the input entry is for, or drop it. | spec/functional/FR-038-syft-sbom-producer-request.md:38,40 |

## Verdict

FND-001 makes one acceptance criterion unpassable against the real tool and
adapter; fix it with the environment entry and an explicit count rule before
implementation. The other three are real edge cases a reader would trip on.
