---
id: FR-069
title: "Consume the Quoin test-result export as data and fail on an unusable export"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-066"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-051"
    type: "references"
---

# FR-069: Consume the Quoin test-result export as data and fail on an unusable export

## Description

Tracked by Linear EA-97. [Quoin FR-186](ix://agent-ix/quoin/FR-186) is authoritative for the release-evidence producer contract, and [Quoin FR-185](ix://agent-ix/quoin/FR-185) is authoritative for export behavior and Test Result semantics. Engineering Assurance owns the reusable workflow's producer-trust and attestation boundary. Its verifier authenticates the source workflow identity against the configured trust policy and binds accepted evidence to the signed tag and workflow-computed subject before handing it to Quoin. The verifier passes the source-signed provenance unchanged to the export defined by [FR-070](./FR-070-release-and-receipt-workflow-files.md).

Engineering Assurance authenticates and transports the source-signed provenance. The Quoin exporter alone evaluates run-record integrity and absence under Quoin FR-185 and FR-186; the Engineering Assurance verifier does not reinterpret those inputs or outcomes. An absent required provenance envelope prevents the release workflow from producing a release statement.

The `verify` job has read-only repository permission and no signing permission. It accepts the Quoin-produced statement only when its subject matches the workflow-computed subject, then passes that result unchanged to the attestation job. The attestation job signs only the result accepted by the verify job, in accordance with [FR-051](./FR-051-release-evidence-snapshot-placement.md).

## Behavior

- The release attestation workflow SHALL run the export in its `verify` job and not in the `attest` job.
- The release attestation workflow SHALL supply the artifact files and their SHA-256 digests as the subject of the export.
- The release-evidence verifier SHALL authenticate the DSSE envelope's source workflow identity under the configured trust policy before passing the exact envelope to Quoin.
- The release-evidence verifier SHALL bind authenticated producer evidence to the signed-tag revision and the subject computed by the workflow.
- The release-evidence verifier SHALL pass the source-signed provenance unchanged to the Quoin exporter in accordance with FR-185.
- The release-evidence verifier SHALL transport the authenticated provenance and evidence artifact to the Quoin exporter without changing them.
- The Quoin exporter SHALL evaluate run-record integrity and absence in accordance with [Quoin FR-185](ix://agent-ix/quoin/FR-185) and [Quoin FR-186](ix://agent-ix/quoin/FR-186).
- The release attestation workflow SHALL use the Quoin export outcome without independently reinterpreting run-record evidence.
- The release attestation workflow SHALL check that the exported statement's subject equals the supplied subject.
- The release attestation workflow SHALL fail with `test_result_export_invalid` when the export is missing, unreadable, invalid or has a different subject.
- The release attestation workflow SHALL stop before producing a release statement when the required provenance envelope is absent.
- The release attestation workflow SHALL fail with `test_result_export_invalid` when the release-evidence verifier rejects producer identity or subject binding.
- The release attestation workflow SHALL sign that result unchanged.
- The release attestation workflow SHALL sign the exported predicate unchanged.
- The release attestation workflow SHALL NOT modify the exported predicate.
- The release attestation workflow SHALL NOT run any Quoin evidence command that reads git or the network.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-069-AC-1 | With authenticated provenance and a matching workflow-computed subject, an integration test confirms that EA passes through and signs a Quoin `FAILED` result unchanged, without evaluating or reclassifying run evidence. | Test |
| FR-069-AC-2 | An absent required provenance envelope, missing or unreadable export, mismatched subject, or producer identity that is invalid, untrusted, or not authorized for the selected signed tag stops attestation and produces no release statement. | Test |
| FR-069-AC-3 | Static inspection shows the workflow and the statement builder call no Quoin evidence command that reads git or the network. | Inspection |
| FR-069-AC-4 | The export runs in the `verify` job, with the evidence store taken from the `evidence_artifact` input and the subject computed by the workflow; the `attest` job runs no Quoin command, and a caller-built export is not accepted. | Test |
| FR-069-AC-5 | A boundary test confirms the verifier authenticates and transports provenance unchanged, while the Quoin exporter alone evaluates run-record presence and integrity under [Quoin FR-185](ix://agent-ix/quoin/FR-185) and [Quoin FR-186](ix://agent-ix/quoin/FR-186). | Test |
| FR-069-AC-6 | The verifier passes the source-workflow-signed provenance unchanged to Quoin; a change that invalidates producer authentication is rejected before attestation. | Test |

## Dependencies

- **Upstream**: [FR-066](./FR-066-release-attestation-set-and-subjects.md), [FR-051](./FR-051-release-evidence-snapshot-placement.md).
- **Cross-repository**: Linear PLAT-1786 (the Quoin export) and Quoin FR-185/FR-186.
- **Downstream**: [FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md).
