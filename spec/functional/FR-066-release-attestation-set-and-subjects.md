---
id: FR-066
title: "Emit a release as a fixed set of standard attestations over the release artifacts"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-047"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-051"
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

Tracked by Linear EA-97. A release produces one attestation set, written as in-toto statements and published through the GitHub attestation action. The set has four members: an SLSA Provenance v1 statement for the build, a CycloneDX SBOM statement whose document is produced by Syft ([FR-038](./FR-038-syft-sbom-producer-request.md), [FR-039](./FR-039-syft-sbom-evidence-ingestion.md), through the executor of [FR-019](./FR-019-bounded-producer-execution.md)), a test-result statement exported from the Quoin evidence store ([FR-069](./FR-069-test-result-export-is-consumed-as-data.md)), and one verification summary ([FR-068](./FR-068-verification-summary-signed-by-the-pinned-workflow.md)). The artifact files are produced by the caller's build job and handed to the reusable workflow as a workflow artifact; the reusable workflow builds nothing and computes each file's SHA-256 digest itself. The first three statements have the artifact files as their subjects. The verification summary has the same artifact-file subjects and also the release commit as a git commit subject ([FR-068](./FR-068-verification-summary-signed-by-the-pinned-workflow.md)).

The set does not carry the release evidence snapshot of [FR-051](./FR-051-release-evidence-snapshot-placement.md). The release commit subject of the summary binds the snapshot, because the release commit already holds the snapshot bytes; this answers the handoff that [FR-053](./FR-053-snapshot-and-attestations-never-replace-the-decision.md) leaves to this specification, and an attestation that carries the snapshot as content is not part of the set. The attestations are copies of facts the signed release commit and tag already fix; per [FR-053](./FR-053-snapshot-and-attestations-never-replace-the-decision.md) they never replace the release decision, and [FR-047](./FR-047-release-record-and-signed-release-tag.md) reports `released` without them.

No predicate is invented for the release: the four standard predicate types are used as defined by their owners, and nothing is added to them. The set is closed ([NFR-008](../non-functional/NFR-008-closed-schemas.md)).

Out of scope: the signed tag and the release record ([FR-047](./FR-047-release-record-and-signed-release-tag.md)), the build itself, and the SBOM tool (Syft is the only generator).

## Behavior

- The release attestation workflow SHALL write one SLSA Provenance v1 statement for the release artifacts.
- The release attestation workflow SHALL write one CycloneDX SBOM statement whose document comes from the Syft ingestion of FR-039.
- The release attestation workflow SHALL write one test-result statement from the export consumed under FR-069.
- The release attestation workflow SHALL write one verification summary statement as FR-068 defines.
- The release attestation workflow SHALL give the three statements other than the summary the artifact files as subjects.
- The release attestation workflow SHALL give the verification summary the artifact files and the release commit as subjects.
- The release attestation workflow SHALL compute the SHA-256 digest of each artifact file itself.
- The artifact subject verifier SHALL return `verified` when the requested SHA-256 artifact digest equals the in-toto subject digest and `wrong_subject` when they differ.
- The release attestation workflow SHALL NOT build the artifacts.
- The release attestation workflow SHALL NOT write the release evidence snapshot into any statement.
- The release attestation workflow SHALL NOT write a statement of a predicate type outside the four.
- The release attestation workflow SHALL NOT generate an SBOM with a tool other than Syft.
- The release verifier SHALL NOT depend on the presence of any of the four statements to report `released`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-066-AC-1 | A fixture release with two artifact files yields exactly four statements; the first three have the two files as subjects at their SHA-256 digests, and the summary has the two files and the release commit. | Test |
| FR-066-AC-2 | A statement set containing a fifth predicate type is rejected by the set validation. | Test |
| FR-066-AC-3 | The SBOM statement document equals the CycloneDX document that the Syft stand-in produced through the executor, byte for byte, and no other SBOM tool is invoked. | Test |
| FR-066-AC-4 | The `release-verify` result for the fixture release is identical with the four statements present and with them removed (FR-053). | Test |
| FR-066-AC-5 | Changing one artifact file after attestation makes the verification of every statement fail on the subject digest, and a digest supplied by the caller in place of the computed one is ignored. | Test |
| FR-066-AC-6 | No statement of the set contains the bytes of the snapshot file of the fixture release commit. | Test |
| FR-066-AC-7 | Static inspection shows the reusable workflow has no build step and receives the artifact files only as a downloaded workflow artifact. | Inspection |
| FR-066-AC-8 | For an in-toto statement with an artifact `sha256` subject, verification with the matching expected digest yields `verified`; verification with a different expected digest yields `wrong_subject`. | Test |

## Dependencies

- **Upstream**: [FR-047](./FR-047-release-record-and-signed-release-tag.md), [FR-051](./FR-051-release-evidence-snapshot-placement.md), [FR-053](./FR-053-snapshot-and-attestations-never-replace-the-decision.md), [FR-019](./FR-019-bounded-producer-execution.md), [FR-038](./FR-038-syft-sbom-producer-request.md), [FR-039](./FR-039-syft-sbom-evidence-ingestion.md), [NFR-008](../non-functional/NFR-008-closed-schemas.md).
- **Downstream**: [FR-067](./FR-067-release-attestations-bind-to-the-signed-release-tag.md), [FR-068](./FR-068-verification-summary-signed-by-the-pinned-workflow.md), [FR-070](./FR-070-release-and-receipt-workflow-files.md).
