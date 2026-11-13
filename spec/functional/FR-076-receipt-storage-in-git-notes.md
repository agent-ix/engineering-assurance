---
id: FR-076
title: "Store the receipt in git notes and publish only the trimmed record through the sink"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-074"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-054"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-056"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-057"
    type: "references"
---

# FR-076: Store the receipt in git notes and publish only the trimmed record through the sink

## Description

Tracked by Linear EA-98.. The receipt is stored in a git note on the commit, under the notes ref `refs/notes/assurance-receipts`, because the GitHub attestation store cannot be looked up by a git commit. The note content is JSON Lines with one Sigstore bundle per statement (three lines), at most 1 MiB. A note holds only the signed statements, which carry no text and no model name ([FR-072](./FR-072-change-record-predicate.md)); the full private form is never written to a note.

A git note on a public repository is a public write. For a public repository the receipt workflow runs the leak check of [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md) over the references of the record before it writes the note or signs through a public log. If the check refuses, the workflow writes a failed receipt that carries the commit as subject, the failure `leak_check_refused` and no `ticket` reference. The receipt is also published through the sink interface ([FR-054](./FR-054-record-sink-interface-and-implementations.md)) with the repository's resolved sink: the public sink gets only the trimmed record ([FR-056](./FR-056-public-and-private-record-split.md)); the private store has the full form.

## Behavior

- The receipt workflow SHALL write the three signed bundles of a receipt to a note on the commit under the notes ref `refs/notes/assurance-receipts`.
- The receipt workflow SHALL write the note as JSON Lines, one bundle per line, within 1 MiB.
- The receipt workflow SHALL run the leak check before writing a note to a public repository.
- The receipt workflow SHALL write a failed receipt without a ticket reference when the leak check refuses.
- The receipt workflow SHALL publish the receipt as a record of kind `change_record` through the sink interface.
- The receipt workflow SHALL NOT write the private form, ticket text, criteria text, a model name or the agent BOM to a note.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-076-AC-1 | A fixture receipt appears as a note of three JSON lines on the pushed commit under the notes ref, and the note survives a second push to the default branch. | Test |
| FR-076-AC-2 | A note whose content would exceed 1 MiB is refused with a named error and no note is written. | Test |
| FR-076-AC-3 | A public repository whose record references a private Linear team gets a failed receipt with `leak_check_refused`, no `ticket` member and no public-log call for the refused references. | Test |
| FR-076-AC-4 | The public sink record of the fixture receipt holds digests, verdict and timestamps only (FR-056), and the private store holds the full form. | Test |
| FR-076-AC-5 | The note bytes of the fixture receipt contain none of the fixture's ticket title, criterion text or model name. | Test |

## Dependencies

- **Upstream**: [FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md), [FR-054](./FR-054-record-sink-interface-and-implementations.md), [FR-056](./FR-056-public-and-private-record-split.md), [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md).
- **Downstream**: [FR-077](./FR-077-missing-receipt-check.md), [FR-078](./FR-078-receipt-verify.md).
