---
id: FR-070
title: "Name the workflow files that the attestation and receipt features add"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-066"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-068"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-075"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-077"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-011"
    type: "references"
---

# FR-070: Name the workflow files that the attestation and receipt features add

## Description

Tracked by Linear EA-97. Peter cleared edits under `.github/workflows` for these features. This requirement names every workflow file the implementing change adds, so the review can check the diff against it. All files are in this repository.

- `.github/workflows/release-attestation.yml`: a reusable workflow (`workflow_call`). It verifies the signed tag and role policy ([FR-067](./FR-067-release-attestations-bind-to-the-signed-release-tag.md)), runs the attestation actions, and signs the verification summary ([FR-068](./FR-068-verification-summary-signed-by-the-pinned-workflow.md)). Its identity is the pinned signer.
- `.github/workflows/change-receipt.yml`: a reusable workflow (`workflow_call`) that writes the per-change receipt ([FR-075](./FR-075-push-to-main-receipt-workflow.md)).
- `.github/workflows/missing-receipt-check.yml`: a workflow on `schedule` and `workflow_dispatch` that lists commits with no receipt ([FR-077](./FR-077-missing-receipt-check.md)).
- `.github/workflows/release.yml`: this repository's own caller, on push of a tag matching `v*`, calling `release-attestation.yml`.
- `.github/workflows/receipt.yml`: this repository's own caller, on push to the default branch, calling `change-receipt.yml`.

A repository that adopts the features adds its own two callers. `ci.yml` is unchanged, and the approval review gate of [FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md) is a separate workflow outside this requirement. Jobs that sign hold `id-token: write` and `attestations: write`, and the receipt job holds `contents: write` for the notes ref only; no job runs an agent ([NFR-011](../non-functional/NFR-011-signing-jobs-run-no-agent.md)).


## Behavior

- The implementing change SHALL add the five workflow files this requirement names and no other workflow file.
- The reusable workflows SHALL declare `workflow_call` as their trigger.
- The missing-receipt workflow SHALL declare `schedule` and `workflow_dispatch` as its triggers.
- The caller workflows SHALL trigger on tag push `v*` and on push to the default branch respectively.
- Each signing job SHALL declare the least permissions it needs and no more.
- Each workflow file SHALL NOT run an agent step.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-070-AC-1 | The set of files added under `.github/workflows` by the implementing change equals the five named files. | Inspection |
| FR-070-AC-2 | The trigger and permission declarations of each file equal this requirement's list, checked by parsing the YAML. | Test |
| FR-070-AC-3 | No step in any of the five files invokes an agent harness or model client. | Inspection |

## Dependencies

- **Upstream**: [FR-066](./FR-066-release-attestation-set-and-subjects.md), [FR-068](./FR-068-verification-summary-signed-by-the-pinned-workflow.md), [FR-075](./FR-075-push-to-main-receipt-workflow.md), [FR-077](./FR-077-missing-receipt-check.md), [FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md), [NFR-011](../non-functional/NFR-011-signing-jobs-run-no-agent.md).
