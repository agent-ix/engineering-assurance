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

Tracked by Linear EA-97. The signed source release workflow supplies the `release-evidence` artifact and its version-2 DSSE execution-provenance envelope as defined by Quoin FR-186. The payload type is `application/vnd.agent-ix.execution-provenance.v2+json`; its payload contains exactly `version`, `sourceRevision`, `testMatrix`, and `runs`. `version` is `2`; `sourceRevision` is the full resolved commit for the signed tag; `testMatrix` is the complete canonical Quire Test Matrix computed at that commit; and `runs` is sorted by suite id with exactly `suite`, `recordPath`, and lowercase `sha256` of each exact run-record byte sequence. The source workflow signs the payload through the configured OIDC trust path. The release-evidence verifier authenticates the source workflow identity and artifact provenance under the configured trust policy before accepting the payload.

Quoin exports the retained evidence as an in-toto Test Result v0.1 statement through a pure mapping from its run records, tracked by Linear PLAT-1786. The statement's subject is supplied by the release-evidence verifier, and the export runs no Git, network, or signing operation. The `verify` job of the reusable workflow ([FR-070](./FR-070-release-and-receipt-workflow-files.md)) validates the envelope signature, payload version and shape, source revision, complete matrix, and every present run-record digest before invoking the export. It passes the verified envelope unchanged as `--execution-provenance`, alongside artifact files and their SHA-256 digests as the subject; every subject descriptor carries `annotations.revision` equal to `sourceRevision`. Quoin verifies the envelope and digests locally, derives the required population from the complete Test Matrix and maintained suite declarations for that revision, and returns one verdict per required obligation.

The `verify` job has `contents: read` and no signing permission. It checks that the returned statement's subject equals the supplied subject, validates the predicate against the closed shape (`result`, `configuration`, `passedTests`, `warnedTests`, `failedTests`), and passes the predicate unchanged to the attestation job; it never edits the predicate. An absent or unreadable export, malformed provenance, invalid signature, untrusted producer, mismatched subject revision, or changed matrix/run bytes stops the release attestation with `test_result_export_invalid`. With valid provenance, a missing binding, empty symbol set, or missing required run record marks only the affected obligation in `failedTests`; the resulting `FAILED` statement is signed unchanged and never counted as passing under [FR-051](./FR-051-release-evidence-snapshot-placement.md).

This requirement defines the producer and consumer boundary for the version-2 execution provenance and the export of PLAT-1786. Of the Quoin export, Engineering Assurance requires that the predicate has exactly `result`, `configuration`, `passedTests`, `warnedTests` and `failedTests`; each entry is an obligation in the complete computed Test Matrix; a failed, stale, or incomplete obligation is listed in `failedTests` and never in `passedTests`; `result` is `FAILED` when any entry is failed, stale, or incomplete, `WARNED` when no entry is failed and any is warned, and `PASSED` otherwise; the output is RFC 8785 canonical form and byte-identical for an identical store, subject, and provenance; and an unreadable or invalid store exits non-zero with a named error and writes no file. Engineering Assurance consumes the full statement with its verified subject and provenance; it defines no second export shape.

## Behavior

- The release attestation workflow SHALL run the export in its `verify` job and not in the `attest` job.
- The release attestation workflow SHALL supply the artifact files and their SHA-256 digests as the subject of the export.
- The signed source release workflow SHALL include the version-2 DSSE execution-provenance envelope and all matrix and run-record bytes committed by that envelope in the `release-evidence` artifact.
- The release-evidence verifier SHALL authenticate the DSSE envelope's source workflow identity under the configured trust policy before passing the exact envelope to Quoin.
- The release-evidence verifier SHALL require the envelope payload's `sourceRevision` to equal the resolved commit of the signed tag and each subject descriptor's `annotations.revision`.
- The release-evidence verifier SHALL require payload `testMatrix` to be the complete canonical Test Matrix computed at `sourceRevision`.
- The release-evidence verifier SHALL verify each present run record's exact bytes against the matching `runs` digest.
- The release attestation workflow SHALL pass the verified envelope as `--execution-provenance`.
- The release attestation workflow SHALL NOT construct, edit, or replace the envelope payload.
- The release attestation workflow SHALL check that the exported statement's subject equals the supplied subject.
- The release attestation workflow SHALL validate the exported predicate against the closed predicate shape before use.
- The release attestation workflow SHALL fail with `test_result_export_invalid` when the export is missing, unreadable, invalid or has a different subject.
- The release attestation workflow SHALL fail with `test_result_export_invalid` when provenance is missing, malformed, untrusted, incorrectly signed, or bound to a different subject revision or matrix/run digest.
- When valid provenance identifies a required obligation but its binding, symbols, or expected run record are absent, the export SHALL return a `FAILED` verdict for that obligation.
- The release attestation workflow SHALL sign that result unchanged.
- The release attestation workflow SHALL sign the exported predicate unchanged.
- The release attestation workflow SHALL NOT modify the exported predicate.
- The release attestation workflow SHALL NOT run any Quoin evidence command that reads git or the network.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-069-AC-1 | A source-workflow-signed version-2 DSSE payload with the exact required fields, full matrix, source revision, and run-record digests is authenticated by the verifier; with matching subject annotations and bytes, an export with passing, failing, and stale obligations yields a signed statement whose predicate equals the exported predicate byte for byte. | Test |
| FR-069-AC-2 | A missing export, unknown predicate member, `PASSED` result with non-empty `failedTests`, mismatched subject, malformed payload, invalid/untrusted signature, matrix/run digest mismatch, or non-JSON export fails with `test_result_export_invalid` and writes no statement. | Test |
| FR-069-AC-3 | Static inspection shows the workflow and the statement builder call no Quoin evidence command that reads git or the network. | Inspection |
| FR-069-AC-4 | The export runs in the `verify` job, with the evidence store taken from the `evidence_artifact` input and the subject computed by the workflow; the `attest` job runs no Quoin command, and a caller-built export is not accepted. | Test |
| FR-069-AC-5 | With valid authenticated provenance and a complete matrix, omission of a required binding, symbol set, or run record produces `FAILED` only for affected obligations; it does not invalidate the signed provenance or cause whole-export refusal. | Test |
| FR-069-AC-6 | The `verify` job passes the source-workflow-signed envelope byte for byte to Quoin as `--execution-provenance`; changing its payload, source revision, matrix, or run-record bytes is rejected before attestation. | Test |

## Dependencies

- **Upstream**: [FR-066](./FR-066-release-attestation-set-and-subjects.md), [FR-051](./FR-051-release-evidence-snapshot-placement.md).
- **Cross-repository**: Linear PLAT-1786 (the Quoin export) and Quoin FR-185/FR-186.
- **Downstream**: [FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md).
