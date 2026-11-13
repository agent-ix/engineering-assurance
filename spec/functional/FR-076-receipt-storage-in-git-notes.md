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

Tracked by Linear EA-98. The receipt is stored in a git note on the commit, under the notes ref `refs/notes/assurance-receipts`, because the GitHub attestation store cannot be looked up by a git commit. The note content is JSON Lines with one Sigstore bundle per statement (three lines), at most 1 MiB; a larger note is refused with `receipt_note_too_large`. A note holds only the signed statements, which carry no text and no model name ([FR-072](./FR-072-change-record-predicate.md)); the full private form is never written to a note.

A commit that already has a verified receipt is skipped: the `notes` job writes no second note, so a re-run or a retry never doubles the note. Two pushes in quick succession run two receipt runs that each add a note for different commits. The `notes` job fetches the notes ref, merges it with `git notes merge` using the `cat_sort_uniq` strategy, and pushes; a rejected push is retried up to five times with a fresh fetch and merge, and on exhaustion the job fails with `receipt_note_conflict` and writes nothing. A note on a public repository is a public write: for a public repository the workflow runs the leak check of [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md) before writing the note. If the check refuses, the workflow writes a failed receipt that carries the commit as subject, the failure `leak_check_refused` and no `ticket` reference.

The receipt is also published through the sink interface ([FR-054](./FR-054-record-sink-interface-and-implementations.md)) with the repository's resolved sink. The public form carries `hashes.statement` equal to the digest of the verification summary (the summary names the other two statements by digest) and `hashes.subject_commit` equal to the commit, a verdict `change_record` that is `recorded` for a receipt with status `passed` and `failed` for status `failed` ([FR-056](./FR-056-public-and-private-record-split.md)), and timestamps; the private store has the full form.

## Behavior

- The receipt workflow SHALL write the three signed bundles of a receipt to a note on the commit under the notes ref `refs/notes/assurance-receipts`.
- The receipt workflow SHALL write the note as JSON Lines, one bundle per line.
- The receipt workflow SHALL refuse a note over 1 MiB with `receipt_note_too_large`.
- The receipt workflow SHALL skip a commit that already has a verified receipt and write no second note.
- The receipt workflow SHALL fetch and merge the notes ref with the `cat_sort_uniq` strategy before each push.
- The receipt workflow SHALL retry a rejected notes push up to five times.
- The receipt workflow SHALL fail with `receipt_note_conflict` when the retries are exhausted.
- The receipt workflow SHALL run the leak check before writing a note to a public repository.
- The receipt workflow SHALL write a failed receipt without a ticket reference when the leak check refuses.
- The receipt workflow SHALL publish the receipt as a record of kind `change_record` with the summary digest in `hashes.statement` and the commit in `hashes.subject_commit`.
- The receipt workflow SHALL map a receipt with status `passed` to the verdict `recorded` and `failed` to `failed`.
- The receipt workflow SHALL NOT write the private form, ticket text, criteria text, a model name or the agent BOM to a note.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-076-AC-1 | A fixture receipt appears as a note of three JSON lines on the pushed commit under the notes ref, and the note survives a second push to the default branch. | Test |
| FR-076-AC-2 | A note whose content would exceed 1 MiB is refused with `receipt_note_too_large` and no note is written. | Test |
| FR-076-AC-3 | Two concurrent receipt runs for two commits both end with their notes present on the notes ref; a fixture remote that rejects six pushes in a row fails the run with `receipt_note_conflict` and leaves no partial note. | Test |
| FR-076-AC-4 | A public repository whose record references a private Linear team gets a failed receipt with `leak_check_refused`, no `ticket` member and no public-log call for the refused references. | Test |
| FR-076-AC-5 | The public sink record of the fixture receipt holds the summary digest, the commit, the verdict `recorded` (`failed` for a failed receipt) and timestamps only, and the private store holds the full form. | Test |
| FR-076-AC-6 | The note bytes of the fixture receipt contain none of the fixture's ticket title, criterion text or model name. | Test |
| FR-076-AC-7 | A second run of the receipt workflow for a commit that already has a verified receipt writes no second note, and the note still has three lines. | Test |

## Dependencies

- **Upstream**: [FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md), [FR-054](./FR-054-record-sink-interface-and-implementations.md), [FR-056](./FR-056-public-and-private-record-split.md), [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md).
- **Downstream**: [FR-077](./FR-077-missing-receipt-check.md), [FR-078](./FR-078-receipt-verify.md).
