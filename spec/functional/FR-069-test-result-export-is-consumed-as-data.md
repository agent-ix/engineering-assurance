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

Tracked by Linear EA-97. Quoin exports the retained evidence as an in-toto Test Result v0.1 statement through a pure mapping from its run records, tracked by Linear PLAT-1687: the statement's subject is supplied by the caller, and the export runs no git, no network and no signing. Engineering Assurance consumes that statement as data when it builds the test-result statement of [FR-066](./FR-066-release-attestation-set-and-subjects.md). It passes the artifact files and their SHA-256 digests as the caller-supplied subject, checks that the returned statement's subject equals what it supplied, validates the predicate against the closed shape (`result`, `configuration`, `passedTests`, `warnedTests`, `failedTests`), and signs through the attestation action with that predicate unchanged; it never edits the predicate. A missing, unreadable or invalid export, or a statement whose subject differs from the supplied one, stops the release attestation with `test_result_export_invalid`. A failed or stale result is carried as the export states it: the export lists it in `failedTests` and sets `result` to `FAILED`, and it is never counted as passing (the rule of [FR-051](./FR-051-release-evidence-snapshot-placement.md)).

This requirement depends on PLAT-1687 and states what it requires of that dependency, beyond the ticket's current text, so that the consumer does not have to reinterpret the export. Of the export of PLAT-1687, Engineering Assurance requires that: the predicate has exactly `result`, `configuration`, `passedTests`, `warnedTests` and `failedTests`; each entry is the criterion or test id the evidence store records; a failed or stale result is listed in `failedTests` and never in `passedTests`; `result` is `FAILED` when any entry is failed or stale, `WARNED` when any is warned and `PASSED` otherwise; the output is the RFC 8785 canonical form and is byte-identical for an identical store and subject; and an unreadable or invalid store exits non-zero with a named error and writes no file. Engineering Assurance adapts to the statement shape that PLAT-1687 produces (a full statement with a caller-supplied subject) and defines no second export shape.

## Behavior

- The release attestation workflow SHALL supply the artifact files and their SHA-256 digests as the subject of the export.
- The release attestation workflow SHALL check that the exported statement's subject equals the supplied subject.
- The release attestation workflow SHALL validate the exported predicate against the closed predicate shape before use.
- The release attestation workflow SHALL fail with `test_result_export_invalid` when the export is missing, unreadable, invalid or has a different subject.
- The release attestation workflow SHALL sign the exported predicate unchanged.
- The release attestation workflow SHALL NOT modify the exported predicate.
- The release attestation workflow SHALL NOT run any Quoin evidence command that reads git or the network.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-069-AC-1 | A fixture export with one passing, one failing and one stale entry, whose stale and failing entries are in `failedTests` and whose `result` is `FAILED`, yields a signed statement whose predicate equals the exported predicate byte for byte, with the supplied artifact files as subject. | Test |
| FR-069-AC-2 | A missing export, an export with an unknown predicate member, an export whose `result` is `PASSED` while `failedTests` is not empty, an export whose subject differs from the supplied one, and an export that is not JSON each fail with `test_result_export_invalid` and write no statement. | Test |
| FR-069-AC-3 | Static inspection shows the workflow and the statement builder call no Quoin evidence command that reads git or the network. | Inspection |

## Dependencies

- **Upstream**: [FR-066](./FR-066-release-attestation-set-and-subjects.md), [FR-051](./FR-051-release-evidence-snapshot-placement.md).
- **Cross-repository**: Linear PLAT-1687 (the Quoin export) and the requirements stated on it above.
- **Downstream**: [FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md).
