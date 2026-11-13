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
  - target: "ix://agent-ix/engineering-assurance/FR-057"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-010"
    type: "references"
---

# FR-074: Write one CI-signed receipt per merged commit and record failures as well as passes

## Description

Tracked by Linear EA-98. A receipt is the evidence for one commit that landed on the default branch. It has three statements, each signed by the change-receipt workflow identity (the `receipt_writer` entry of [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md)): the change-record statement ([FR-072](./FR-072-change-record-predicate.md)), the test-result statement of the gates the workflow re-ran (the shape of [FR-069](./FR-069-test-result-export-is-consumed-as-data.md)), and one verification summary whose result is `PASSED` or `FAILED` and whose input attestations are the other two, each by the digest of its RFC 8785 canonical bytes. The agent BOM is not a statement: it is bound by digest in the change record and kept in the private form ([FR-073](./FR-073-agent-bom-in-cyclonedx-1-7.md)). The signature is made through the delegated stack ([NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md)) and never by an agent ([NFR-011](../non-functional/NFR-011-signing-jobs-run-no-agent.md)).

A failing change gets a receipt too. A failed gate, an incomplete role set, a missing contract or a leak-check refusal produce a receipt with `status` `failed` and the matching failure codes, and a verification summary with result `FAILED`; a failure is never dropped to look like a pass. A commit with no receipt is itself the finding ([FR-077](./FR-077-missing-receipt-check.md)).

The full private form goes to the private store first ([FR-056](./FR-056-public-and-private-record-split.md)). The signing configuration follows the resolved sink, as for releases ([FR-071](./FR-071-release-attestations-publish-through-the-sinks.md)): for a public repository the public transparency log is used only when the resolved sink is `public_log` and the leak check of [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md) passed, and a public repository whose resolved sink is the private store or `none` fails with `receipt_public_log_required`; a private repository signs only through the platform's private instance, and when it is unavailable the workflow fails with `receipt_private_signing_unavailable` and never uses the public log; a repository whose resolved sink is `none` fails with `receipt_private_store_required`, because the receipt input and statements are not retained. Statement-level outcomes in the change record use the sink record verdicts: a receipt with status `passed` is published as `recorded` and `failed` as `failed` ([FR-076](./FR-076-receipt-storage-in-git-notes.md)).

## Behavior

- The receipt workflow SHALL write the change-record statement, the test-result statement and the verification summary for each commit.
- The receipt workflow SHALL sign all three statements with its own workflow identity.
- The receipt workflow SHALL set the verification summary result to `FAILED` when the change record has status `failed`.
- The receipt workflow SHALL write a receipt for a failing change.
- The receipt workflow SHALL list the input attestations of the summary by the digest of their RFC 8785 canonical bytes.
- The receipt workflow SHALL write the full private form to the private store before signing.
- The receipt workflow SHALL use the public transparency log for a public repository only when the resolved sink is `public_log` and the leak check passed.
- The receipt workflow SHALL fail with `receipt_public_log_required` for a public repository whose resolved sink is the private store or `none`.
- The receipt workflow SHALL fail with `receipt_private_signing_unavailable` when a private repository's private signing instance is unavailable.
- The receipt workflow SHALL fail with `receipt_private_store_required` for a repository whose resolved sink is `none`.
- The receipt workflow SHALL NOT use a public transparency log for a private repository.
- The receipt workflow SHALL NOT sign with any credential an agent holds.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-074-AC-1 | A passing fixture change yields three signed statements, a `passed` change record and a `PASSED` summary whose input digests equal the other two statements. | Test |
| FR-074-AC-2 | A change whose gate command exits non-zero yields a receipt with status `failed`, failure `gate_failed` and a `FAILED` summary. | Test |
| FR-074-AC-3 | A change with an incomplete role set yields status `failed` with `approvals_incomplete`, and the same receipt is produced for a commit that landed with no pull request. | Test |
| FR-074-AC-4 | Altering one statement of the fixture receipt makes the summary's input digest check fail. | Test |
| FR-074-AC-5 | The private store holds the full private form for the fixture change before the statements are signed, and holds it also for the failed receipt. | Test |
| FR-074-AC-6 | A private repository's receipt, including a run with the private signing stand-in unavailable, makes zero calls to the stand-in public transparency log, and the unavailable run fails with `receipt_private_signing_unavailable`. | Test |
| FR-074-AC-7 | A public repository configured `private_store` or `none` fails with `receipt_public_log_required`, and a repository on `none` fails with `receipt_private_store_required`; the stand-in public log receives no call. | Test |

## Dependencies

- **Upstream**: [FR-072](./FR-072-change-record-predicate.md), [FR-073](./FR-073-agent-bom-in-cyclonedx-1-7.md), [FR-069](./FR-069-test-result-export-is-consumed-as-data.md), [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md), [FR-056](./FR-056-public-and-private-record-split.md), [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md), [FR-071](./FR-071-release-attestations-publish-through-the-sinks.md), [NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md), [NFR-011](../non-functional/NFR-011-signing-jobs-run-no-agent.md).
- **Downstream**: [FR-075](./FR-075-push-to-main-receipt-workflow.md), [FR-076](./FR-076-receipt-storage-in-git-notes.md), [FR-078](./FR-078-receipt-verify.md).
