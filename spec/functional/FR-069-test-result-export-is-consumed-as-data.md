---
id: FR-069
title: "Consume the Quoin test-result export as data and fail on an unusable export"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-066"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-049"
    type: "references"
---

# FR-069: Consume the Quoin test-result export as data and fail on an unusable export

## Description

Tracked by Linear EA-97. Quoin writes the in-toto test-result predicate file from its evidence store as a pure export: it runs no git, no network and no signing. Engineering Assurance consumes that file as data when it builds the test-result statement of [FR-066](./FR-066-release-attestation-set-and-subjects.md). It validates the file against the closed test-result predicate shape, adds the subjects, and signs through the attestation action; it never edits the predicate. A missing, unreadable or invalid file stops the release attestation; a failed or stale result inside a valid file is carried as the export states it, never counted as passing (the rule of [FR-051](./FR-051-release-evidence-snapshot-placement.md)). The requirement on the export itself belongs to the Quoin evidence specification, which this repository cites by capability and not by copy.

## Behavior

- The release attestation workflow SHALL validate the exported test-result file against the closed predicate shape before use.
- The release attestation workflow SHALL fail with `test_result_export_invalid` when the file is missing, unreadable or invalid.
- The release attestation workflow SHALL carry failed and stale results in the statement exactly as the export states them.
- The release attestation workflow SHALL NOT modify the exported predicate.
- The release attestation workflow SHALL NOT run any Quoin evidence command that reads git or the network.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-069-AC-1 | A fixture export with one passing, one failing and one stale result yields a statement whose predicate equals the export byte for byte. | Test |
| FR-069-AC-2 | A missing export, an export with an unknown predicate member, and an export that is not JSON each fail with `test_result_export_invalid` and write no statement. | Test |
| FR-069-AC-3 | Static inspection shows the workflow and the statement builder call no Quoin evidence command that reads git or the network. | Inspection |

## Dependencies

- **Upstream**: [FR-066](./FR-066-release-attestation-set-and-subjects.md), [FR-051](./FR-051-release-evidence-snapshot-placement.md).
- **Downstream**: [FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md).
