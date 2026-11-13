---
id: FR-075
title: "Write the receipt in a push-to-main workflow that runs no agent"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-074"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-072"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-064"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-070"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-011"
    type: "references"
---

# FR-075: Write the receipt in a push-to-main workflow that runs no agent

## Description

Tracked by Linear EA-98.. The reusable workflow `change-receipt.yml` ([FR-070](./FR-070-release-and-receipt-workflow-files.md)) runs on every push to the default branch and writes the receipt of [FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md). It has no merge queue and does not block the push; the pre-merge decision is the review gate of [FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md), and the receipt is the post-merge record. Steps, in order: it reads the policy documents from the protected base branch ([FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md)), among them the receipt policy `engineering-assurance.receipt-policy/v1` at `.engineering-assurance/receipt-policy.json` (a closed schema with `protocol` and `gates`, a list of `{name, argv}` commands, and `adopted_at`, the first commit that needs a receipt); it re-runs each listed gate on the pushed commit in a job that holds no agent; it exports the results; it reads the receipt input of the change from the private store, keyed by the pull request reference, which the ticket-coder-reviewer flow wrote before the merge (a closed document `engineering-assurance.receipt-input/v1` holding the ticket, criteria, review results and session descriptor); it finds the approval statements of the pull request, verifies them under [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md) and evaluates the role policy ([FR-061](./FR-061-role-policy-and-distinct-individuals.md)); it builds the change record and BOM; it signs; and it stores and publishes the receipt ([FR-076](./FR-076-receipt-storage-in-git-notes.md)).

A direct push with no pull request, or a pull request with no receipt input, still gets a receipt: it is `failed` with `approvals_incomplete` or `contract_missing`. The subject is the pushed commit and tree, so a squash-merged change is covered by the merge commit.

## Behavior

- The receipt workflow SHALL run on push to the default branch.
- The receipt workflow SHALL read the receipt policy from the protected base branch.
- The receipt workflow SHALL re-run each gate listed in the receipt policy on the pushed commit.
- The receipt workflow SHALL read the receipt input from the private store by the pull request reference.
- The receipt workflow SHALL verify the approval statements of the pull request and evaluate the role policy with the function of FR-061.
- The receipt workflow SHALL write a failed receipt for a push without a pull request.
- The receipt workflow SHALL skip a commit before `adopted_at` and write nothing for it.
- The receipt workflow SHALL NOT run an agent in any job.
- The receipt workflow SHALL NOT block the push.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-075-AC-1 | A fixture squash-merged pull request with three covering approval statements and a receipt input yields a passed receipt on the merge commit. | Test |
| FR-075-AC-2 | A push with no pull request yields a failed receipt with `approvals_incomplete`, and a pull request with no receipt input yields a failed receipt with `contract_missing`. | Test |
| FR-075-AC-3 | A gate that exits non-zero yields a failed receipt, and the push is not blocked by the workflow. | Test |
| FR-075-AC-4 | A receipt policy on the pushed branch that differs from the protected base branch's does not change the gates that run. | Test |
| FR-075-AC-5 | A commit before `adopted_at` gets no receipt and is not reported by the missing-receipt check. | Test |
| FR-075-AC-6 | Static inspection shows the workflow has no agent step and the signing step runs after every gate step. | Inspection |

## Dependencies

- **Upstream**: [FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md), [FR-072](./FR-072-change-record-predicate.md), [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md), [FR-061](./FR-061-role-policy-and-distinct-individuals.md), [FR-070](./FR-070-release-and-receipt-workflow-files.md), [FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md), [NFR-011](../non-functional/NFR-011-signing-jobs-run-no-agent.md).
- **Downstream**: [FR-077](./FR-077-missing-receipt-check.md).
