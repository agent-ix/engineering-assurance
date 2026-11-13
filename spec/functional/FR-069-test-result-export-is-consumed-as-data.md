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

Tracked by Linear EA-97. Quoin writes the in-toto test-result predicate file from its evidence store as a pure export: it runs no git, no network and no signing. Engineering Assurance consumes that file as data when it builds the test-result statement of [FR-066](./FR-066-release-attestation-set-and-subjects.md). It validates the file against the closed predicate shape (`result`, `configuration`, `passedTests`, `warnedTests`, `failedTests`), adds the subjects, and signs through the attestation action; it never edits the predicate. A missing, unreadable or invalid file stops the release attestation with `test_result_export_invalid`. A failed or stale result is carried as the export states it: the export lists it in `failedTests` and sets `result` to `FAILED`, and it is never counted as passing (the rule of [FR-051](./FR-051-release-evidence-snapshot-placement.md)).

This requirement depends on a Quoin capability that the Quoin specification must carry. The text of that capability, stated here as the dependency and not as a Quoin requirement id, is:

> Quoin SHALL provide a command that writes, from the retained evidence store and to a caller-named file, one in-toto Test Result v0.1 predicate with exactly result, configuration, passedTests, warnedTests and failedTests; each entry is the criterion or test id the evidence store records; a failed or stale result is listed in failedTests and never in passedTests; result is FAILED when any entry is failed or stale, WARNED when any is warned, PASSED otherwise; output is the RFC 8785 canonical form, byte-identical for an identical store; the command runs no git, no network and no signing; an unreadable or invalid store exits non-zero with a named error and writes no file.

## Behavior

- The release attestation workflow SHALL validate the exported test-result file against the closed predicate shape before use.
- The release attestation workflow SHALL fail with `test_result_export_invalid` when the file is missing, unreadable or invalid.
- The release attestation workflow SHALL carry the export's lists and `result` unchanged into the statement.
- The release attestation workflow SHALL NOT modify the exported predicate.
- The release attestation workflow SHALL NOT run any Quoin evidence command that reads git or the network.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-069-AC-1 | A fixture export with one passing, one failing and one stale entry, whose stale and failing entries are in `failedTests` and whose `result` is `FAILED`, yields a statement whose predicate equals the export byte for byte. | Test |
| FR-069-AC-2 | A missing export, an export with an unknown predicate member, an export whose `result` is `PASSED` while `failedTests` is not empty, and an export that is not JSON each fail with `test_result_export_invalid` and write no statement. | Test |
| FR-069-AC-3 | Static inspection shows the workflow and the statement builder call no Quoin evidence command that reads git or the network. | Inspection |

## Dependencies

- **Upstream**: [FR-066](./FR-066-release-attestation-set-and-subjects.md), [FR-051](./FR-051-release-evidence-snapshot-placement.md).
- **Cross-repository**: the Quoin capability quoted above.
- **Downstream**: [FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md).
