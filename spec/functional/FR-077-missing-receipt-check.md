---
id: FR-077
title: "List commits on the default branch that have no valid receipt and fail closed on unknown status"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-076"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-078"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-075"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-070"
    type: "references"
---

# FR-077: List commits on the default branch that have no valid receipt and fail closed on unknown status

## Description

Tracked by Linear EA-98.. A commit pushed by a ruleset bypass, or a commit whose workflow failed to run, has no receipt, and nothing else would say so. The `receipt-missing` command lists the first-parent commits of the default branch from `adopted_at` of the receipt policy ([FR-075](./FR-075-push-to-main-receipt-workflow.md)) that have no valid receipt, and the workflow `missing-receipt-check.yml` ([FR-070](./FR-070-release-and-receipt-workflow-files.md)) runs it on a schedule and on demand and fails when the list is not empty. Each listed commit has a reason: `no_note` (no note under the notes ref), `invalid_receipt` (the receipt does not verify under [FR-078](./FR-078-receipt-verify.md)) or `status_unknown` (the note or the verification could not be read). A commit with unknown status counts as missing. A valid receipt whose status is `failed` is not missing: it is listed separately as `failed_receipt`, so a recorded failure is visible without being called absent.

## Behavior

- The `receipt-missing` command SHALL list each first-parent commit from `adopted_at` that has no note under the notes ref with the reason `no_note`.
- The `receipt-missing` command SHALL list each commit whose receipt does not verify with the reason `invalid_receipt`.
- The `receipt-missing` command SHALL list a commit whose receipt cannot be read with the reason `status_unknown`.
- The `receipt-missing` command SHALL list a commit with a verified failed receipt as `failed_receipt` and not as missing.
- The missing-receipt workflow SHALL fail when the list of missing commits is not empty.
- The `receipt-missing` command SHALL NOT list a commit before `adopted_at`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-077-AC-1 | A fixture history of five commits, one without a note, one with a tampered note and one with an unreadable note, lists exactly those three with the reasons `no_note`, `invalid_receipt` and `status_unknown`. | Test |
| FR-077-AC-2 | A commit with a verified failed receipt is listed as `failed_receipt` and does not fail the workflow by itself. | Test |
| FR-077-AC-3 | A commit pushed by a ruleset bypass with no receipt is listed with `no_note`. | Test |
| FR-077-AC-4 | Commits before `adopted_at` are not listed. | Test |
| FR-077-AC-5 | An empty list leaves the workflow green, and a non-empty list fails it. | Test |

## Dependencies

- **Upstream**: [FR-076](./FR-076-receipt-storage-in-git-notes.md), [FR-078](./FR-078-receipt-verify.md), [FR-075](./FR-075-push-to-main-receipt-workflow.md), [FR-070](./FR-070-release-and-receipt-workflow-files.md).
