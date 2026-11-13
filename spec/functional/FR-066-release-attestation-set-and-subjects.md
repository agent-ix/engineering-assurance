---
id: FR-066
title: "Emit a release as a fixed set of standard attestations over the release artifacts"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-047"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-053"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-038"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-039"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-008"
    type: "references"
---

# FR-066: Emit a release as a fixed set of standard attestations over the release artifacts

## Description

Tracked by Linear EA-97. A release produces one attestation set, written as in-toto statements and published through the GitHub attestation action. The set has four members: an SLSA Provenance v1 statement for the build, a CycloneDX SBOM statement whose document is produced by Syft ([FR-038](./FR-038-syft-sbom-producer-request.md), [FR-039](./FR-039-syft-sbom-evidence-ingestion.md), through the executor of [FR-019](./FR-019-bounded-producer-execution.md)), a test-result statement exported from the Quoin evidence store ([FR-069](./FR-069-test-result-export-is-consumed-as-data.md)), and one verification summary ([FR-068](./FR-068-verification-summary-signed-by-the-pinned-workflow.md)). Every member has the same subjects: the release artifact files, each by its SHA-256 digest. The attestations are copies of facts that the signed release commit and tag already fix; per [FR-053](./FR-053-snapshot-and-attestations-never-replace-the-decision.md) they never replace the release decision, and [FR-047](./FR-047-release-record-and-signed-release-tag.md) reports `released` without them.

No attestation predicate is invented for the release: the four standard predicate types are used as defined by their owners, and nothing is added to them. The set is closed ([NFR-008](../non-functional/NFR-008-closed-schemas.md)): a release attestation of any other kind is not part of the set.

Out of scope: the signed tag and the release record ([FR-047](./FR-047-release-record-and-signed-release-tag.md)), the build itself, and the SBOM tool (Syft is the only generator).

## Behavior

- The release attestation workflow SHALL write one SLSA Provenance v1 statement for the release artifacts.
- The release attestation workflow SHALL write one CycloneDX SBOM statement whose document comes from the Syft ingestion of FR-039.
- The release attestation workflow SHALL write one test-result statement from the exported file of FR-069.
- The release attestation workflow SHALL write one verification summary statement as FR-068 defines.
- The release attestation workflow SHALL give all four statements the same subject set of artifact files and SHA-256 digests.
- The release attestation workflow SHALL NOT write a statement of a predicate type outside the four.
- The release attestation workflow SHALL NOT generate an SBOM with a tool other than Syft.
- The release verifier SHALL NOT depend on the presence of any of the four statements to report `released`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-066-AC-1 | A fixture release with two artifact files yields exactly four statements, each with the two files as subjects at their SHA-256 digests and no other subject besides the release commit subject of FR-068. | Test |
| FR-066-AC-2 | A statement set containing a fifth predicate type is rejected by the set validation. | Test |
| FR-066-AC-3 | The SBOM statement document equals the CycloneDX document that the Syft stand-in produced through the executor, byte for byte, and no other SBOM tool is invoked. | Test |
| FR-066-AC-4 | The `release-verify` result for the fixture release is identical with the four statements present and with them removed (FR-053). | Test |
| FR-066-AC-5 | Changing one artifact file after attestation makes the verification of every statement fail on the subject digest. | Test |

## Dependencies

- **Upstream**: [FR-047](./FR-047-release-record-and-signed-release-tag.md), [FR-053](./FR-053-snapshot-and-attestations-never-replace-the-decision.md), [FR-019](./FR-019-bounded-producer-execution.md), [FR-038](./FR-038-syft-sbom-producer-request.md), [FR-039](./FR-039-syft-sbom-evidence-ingestion.md), [NFR-008](../non-functional/NFR-008-closed-schemas.md).
- **Downstream**: [FR-067](./FR-067-release-attestations-bind-to-the-signed-release-tag.md), [FR-068](./FR-068-verification-summary-signed-by-the-pinned-workflow.md), [FR-070](./FR-070-release-and-receipt-workflow-files.md).
