---
id: NFR-006
title: "One SBOM tool and one execution path"
type: NFR
relationships:
  - target: "ix://agent-ix/engineering-assurance/NFR-004"
    type: "extends"
  - target: "ix://agent-ix/engineering-assurance/FR-038"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-039"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-044"
    type: "constrains"
---

# NFR-006: One SBOM tool and one execution path

## Statement

Engineering Assurance SHALL use Syft as its only SBOM generator.

Syft SHALL run only through the [FR-019](../functional/FR-019-bounded-producer-execution.md) executor.

Engineering Assurance SHALL NOT parse SBOM components in production code.

The advisory scanner of [FR-044](../functional/FR-044-osv-scanner-advisory-scan-producer-request.md), OSV-Scanner, SHALL
read the Syft SBOM retained by the FR-019 executor and no other SBOM.

The advisory scanner SHALL NOT generate an SBOM.

The advisory scanner SHALL run only through the [FR-019](../functional/FR-019-bounded-producer-execution.md) executor.

## Scope

Applies to the [FR-038](../functional/FR-038-syft-sbom-producer-request.md),
[FR-039](../functional/FR-039-syft-sbom-evidence-ingestion.md) and
[FR-044](../functional/FR-044-osv-scanner-advisory-scan-producer-request.md) code, tests and
fixtures. It extends [NFR-004](./NFR-004-no-parallel-assurance-framework.md).

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| SBOM generators other than Syft | 0 | 0 | architecture-conformance |
| Syft launch sites outside the FR-019 executor | 0 | 0 | architecture-conformance |
| OSV-Scanner launch sites outside the FR-019 executor | 0 | 0 | architecture-conformance |
| OSV-Scanner requests carrying a directory, image or lockfile target or an SBOM output format | 0 | 0 | architecture-conformance |
| Production CycloneDX component readers | 0 | 0 | architecture-conformance |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-006-AC-1 | Design review confirms Syft is the only SBOM generator and that it is launched only by the FR-019 executor. | Inspection |
| NFR-006-AC-2 | The Syft request used by the tests passes the published request schema of `producer-execute --schema request`. | Test |
| NFR-006-AC-3 | Design review confirms OSV-Scanner is launched only by the FR-019 executor and that its request carries no directory, image or lockfile target and no SBOM output format. | Inspection |
| NFR-006-AC-4 | The OSV-Scanner request used by the tests passes the published request schema of `producer-execute --schema request`. | Test |

## Verification

Review the design for SBOM tools, process-launch sites and component readers,
and check the Syft request against the published request schema.
