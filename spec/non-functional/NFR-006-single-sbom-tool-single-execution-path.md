---
id: NFR-006
title: "One SBOM tool and one execution path"
type: NFR
relationships:
  - target: "ix://agent-ix/engineering-assurance/NFR-004"
    type: "extends"
---

# NFR-006: One SBOM tool and one execution path

## Statement

Engineering Assurance SHALL use Syft as its only SBOM generator. Syft SHALL run only
through the [FR-019](../functional/FR-019-bounded-producer-execution.md)
executor. Its output SHALL be read only through the Quoin SBOM adapter, so that
no second SBOM tool, process launcher or CycloneDX parser exists in the repository.

## Scope

Applies to the [FR-038](../functional/FR-038-syft-sbom-producer-request.md) and
[FR-039](../functional/FR-039-syft-sbom-evidence-ingestion.md) code, tests and
fixtures. It extends [NFR-004](./NFR-004-no-parallel-assurance-framework.md).

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| SBOM generators other than Syft | 0 | 0 | architecture-conformance |
| Syft launch sites outside the FR-019 executor | 0 | 0 | architecture-conformance |
| First-party CycloneDX component readers | 0 | 0 | architecture-conformance |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-006-AC-1 | The source tree names no SBOM generator other than Syft and launches Syft at no call site other than the FR-019 executor. | Inspection |
| NFR-006-AC-2 | Adding the Syft request builder changes no Cargo dependency, feature or lock entry used for process launch or JSON component parsing. | Inspection |
| NFR-006-AC-3 | The Syft request builder emits only documents that pass the published request schema of `producer-execute --schema request`. | Test |

## Verification

Inspect the source tree for SBOM tool names, process-launch call sites and
CycloneDX component readers, and check the request builder output against the
published request schema.
