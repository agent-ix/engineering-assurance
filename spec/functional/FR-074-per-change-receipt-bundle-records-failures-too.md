---
id: FR-074
title: "Write one CI-signed receipt per merged commit and record failures as well as passes"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-072"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-073"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-069"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-064"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-056"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-010"
    type: "references"
---

# FR-074: Write one CI-signed receipt per merged commit and record failures as well as passes

## Description

Tracked by Linear EA-98.. A receipt is the evidence for one commit that landed on the default branch. It has three statements, each signed by the change-receipt workflow identity: the change-record statement ([FR-072](./FR-072-change-record-predicate.md)), the test-result statement of the gates the workflow re-ran (the shape of [FR-069](./FR-069-test-result-export-is-consumed-as-data.md)), and one verification summary whose result is `PASSED` or `FAILED` and whose input attestations are the other two by digest. The agent BOM is not a statement: it is bound by digest in the change record and kept in the private form ([FR-073](./FR-073-agent-bom-in-cyclonedx-1-7.md)). The signature is made through the delegated stack ([NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md)) and never by an agent ([NFR-011](../non-functional/NFR-011-signing-jobs-run-no-agent.md)).

A failing change gets a receipt too. A failed gate, an incomplete role set, a missing contract or a leak-check refusal produce a receipt with `status` `failed` and the matching failure codes, and a verification summary with result `FAILED`; a failure is never dropped to look like a pass. A receipt exists for every commit, so a commit with no receipt is itself the finding ([FR-077](./FR-077-missing-receipt-check.md)).

The full private form (ticket text, criteria text, model names, the agent BOM) goes to the private store first ([FR-056](./FR-056-public-and-private-record-split.md)); the receipt is recorded as a record of kind `change_record`.

## Behavior

- The receipt workflow SHALL write the change-record statement, the test-result statement and the verification summary for each commit.
- The receipt workflow SHALL sign all three statements with its own workflow identity.
- The receipt workflow SHALL set the verification summary result to `FAILED` when the change record has status `failed`.
- The receipt workflow SHALL write a receipt for a failing change.
- The receipt workflow SHALL list the input attestations of the summary by digest.
- The receipt workflow SHALL write the full private form to the private store before signing.
- The receipt workflow SHALL NOT sign with any credential an agent holds.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-074-AC-1 | A passing fixture change yields three signed statements, a `passed` change record and a `PASSED` summary whose input digests equal the other two statements. | Test |
| FR-074-AC-2 | A change whose gate command exits non-zero yields a receipt with status `failed`, failure `gate_failed` and a `FAILED` summary. | Test |
| FR-074-AC-3 | A change with an incomplete role set yields status `failed` with `approvals_incomplete`, and the same receipt is produced for a commit that landed with no pull request. | Test |
| FR-074-AC-4 | Altering one statement of the fixture receipt makes the summary's input digest check fail. | Test |
| FR-074-AC-5 | The private store holds the full private form for the fixture change before the statements are signed, and holds it also for the failed receipt. | Test |

## Dependencies

- **Upstream**: [FR-072](./FR-072-change-record-predicate.md), [FR-073](./FR-073-agent-bom-in-cyclonedx-1-7.md), [FR-069](./FR-069-test-result-export-is-consumed-as-data.md), [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md), [FR-056](./FR-056-public-and-private-record-split.md), [NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md), [NFR-011](../non-functional/NFR-011-signing-jobs-run-no-agent.md).
- **Downstream**: [FR-075](./FR-075-push-to-main-receipt-workflow.md), [FR-076](./FR-076-receipt-storage-in-git-notes.md), [FR-078](./FR-078-receipt-verify.md).
