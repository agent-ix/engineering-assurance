---
id: FR-070
title: "Constrain the attestation and receipt workflows: files, jobs, triggers, permissions and untrusted input"
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

# FR-070: Constrain the attestation and receipt workflows: files, jobs, triggers, permissions and untrusted input

## Description

Tracked by Linear EA-97. This requirement describes the workflow files the features add, and the properties that make a signing job safe. All files are in this repository.

| File | Trigger | Job | Permissions | Runs |
| --- | --- | --- | --- | --- |
| `.github/workflows/release-attestation.yml` | `workflow_call`, inputs `tag`, `artifact_name`, `evidence_artifact` (strings) | `verify` | `contents: read` | Engineering Assurance tooling and the Quoin test-result export only: `release-verify`, role step ([FR-067](./FR-067-release-attestations-bind-to-the-signed-release-tag.md)), the digests of the artifact files, and the export of [FR-069](./FR-069-test-result-export-is-consumed-as-data.md) from the evidence store in `evidence_artifact` |
| same | same | `attest` | `contents: read`, `id-token: write`, `attestations: write` | Engineering Assurance tooling and the attestation actions; no repository code |
| `.github/workflows/change-receipt.yml` | `workflow_call`, input `commit` (string) | `gates` | `contents: read` | The repository's gates from the receipt policy ([FR-075](./FR-075-push-to-main-receipt-workflow.md)); nothing else |
| same | same | `record` | `contents: read`, `id-token: write` | Engineering Assurance tooling only; no repository code |
| same | same | `notes` | `contents: write` | Engineering Assurance tooling only: fetch, merge and push the notes ref ([FR-076](./FR-076-receipt-storage-in-git-notes.md)) |
| `.github/workflows/missing-receipt-check.yml` | `schedule`, `workflow_dispatch` | `check` | `contents: read` | `receipt-missing` ([FR-077](./FR-077-missing-receipt-check.md)) |
| `.github/workflows/release.yml` | `push` of tags matching `v*` | `build`, `attest` | `build`: `contents: read`; `attest` calls `release-attestation.yml` with `contents: read`, `id-token: write`, `attestations: write` | the caller's build, then the reusable workflow |
| `.github/workflows/receipt.yml` | `push` to the default branch | `receipt` | calls `change-receipt.yml` with `contents: write`, `id-token: write` | the reusable workflow |

A repository that adopts the features adds its own two callers. `ci.yml` is unchanged. The approval review gate workflow and the main-branch ruleset belong to [FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md) and the EA-109 work; this specification does not define them.

The repository's gates, builds and any other code from the repository under change run only in jobs that hold `contents: read` and nothing else; they hand results to the signing jobs as workflow artifacts, and a signing job executes no code from the repository ([NFR-011](../non-functional/NFR-011-signing-jobs-run-no-agent.md)). `contents: write` is a repository-wide permission and cannot be limited to the notes ref; it is confined to the `notes` job, which runs only Engineering Assurance tooling that pushes `refs/notes/assurance-receipts`, and the default branch stays protected by its ruleset.

Values that an outside party can influence (tag and branch names, commit messages, pull request titles, workflow inputs) reach scripts only through `env:` and never by interpolation into `run:`. Third-party actions in a job that holds `id-token: write` are pinned by full commit SHA: this pin is load-bearing, because a moved tag would run foreign code holding the signing identity. Checkout steps set `persist-credentials: false`. The reusable workflows take the commit and the tag from the trigger context and reject a `commit` input that differs from the pushed commit and a `tag` input that differs from the tag that triggered the run, so a workflow in another repository context cannot ask them to sign something else. The `gates` job receives no secrets and runs on a GitHub-hosted or ephemeral runner, so code from the repository under change cannot reach a signing credential or a persistent host.

## Behavior

- Each workflow file SHALL declare the triggers of the table and no other.
- Each job SHALL declare exactly the permissions of the table.
- Each workflow file SHALL NOT declare the `pull_request_target` or `workflow_run` trigger.
- Each job SHALL pass an untrusted context value to a script only through `env:`.
- Each job SHALL NOT interpolate an untrusted context value into a `run:` script.
- Each job that holds `id-token: write` SHALL reference every third-party action by a full 40-character commit SHA.
- Each checkout step SHALL set `persist-credentials: false`.
- The `gates` job SHALL NOT hold `id-token: write`, `attestations: write` or `contents: write`.
- Each signing job SHALL NOT run code from the repository under change.
- The reusable workflows SHALL reject a `commit` input other than the pushed commit with `receipt_input_commit_mismatch`.
- The reusable workflows SHALL reject a `tag` input other than the triggering tag with `release_input_tag_mismatch`.
- The `gates` job SHALL NOT receive any secret.
- The `gates` job SHALL run on a GitHub-hosted or ephemeral runner.
- Each workflow file SHALL NOT run an agent step.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-070-AC-1 | Parsing the five files yields exactly the triggers, jobs and per-job permissions of the table, and no file declares `pull_request_target` or `workflow_run`. | Test |
| FR-070-AC-2 | No `run:` value in any file contains an expression over `github.ref_name`, `github.head_ref`, an event title or body, a commit message or `inputs.`; each such value appears only under `env:`. | Test |
| FR-070-AC-3 | Every `uses:` in a job with `id-token: write` ends in a 40-hex SHA, and every checkout step has `persist-credentials: false`. | Test |
| FR-070-AC-4 | The `gates` job holds only `contents: read`, the `record` job runs no step that executes a repository file or a command from the receipt policy, and the `notes` job holds no `id-token`. | Test |
| FR-070-AC-5 | No step in any of the five files invokes an agent harness or model client. | Inspection |
| FR-070-AC-6 | A `change-receipt.yml` run with a `commit` input that is not the pushed commit fails with `receipt_input_commit_mismatch`, and a `release-attestation.yml` run with a `tag` input that is not the triggering tag fails with `release_input_tag_mismatch`, in both cases before any signing step. | Test |
| FR-070-AC-7 | The `gates` job declares no `secrets` and no `environment`, and its `runs-on` is a GitHub-hosted label or an ephemeral runner label. | Test |

## Dependencies

- **Upstream**: [FR-066](./FR-066-release-attestation-set-and-subjects.md), [FR-068](./FR-068-verification-summary-signed-by-the-pinned-workflow.md), [FR-075](./FR-075-push-to-main-receipt-workflow.md), [FR-077](./FR-077-missing-receipt-check.md), [FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md), [NFR-011](../non-functional/NFR-011-signing-jobs-run-no-agent.md).
