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

Tracked by Linear EA-97. This requirement describes the Engineering Assurance reusable workflows in this repository and the adopter caller workflows in quoin, quire-cli and quire-rs, including the properties that make a signing job safe. Engineering Assurance's own tag-attestation caller is deferred to EA-122 until a retained evidence store exists.

| File | Trigger | Job | Permissions | Runs |
| --- | --- | --- | --- | --- |
| `.github/workflows/release-attestation.yml` | `workflow_call`, required inputs `tag`, `ea_tooling_tag`, `quoin_tooling_tag`, `record_path`, `allowed_signers_path`; inputs `artifact_name` (default `release-artifacts`), `evidence_artifact` (default `release-evidence`) | `verify` | `contents: read` | Engineering Assurance tooling and the Quoin test-result export only: `release-verify` using `record_path` and `allowed_signers_path`, role step ([FR-067](./FR-067-release-attestations-bind-to-the-signed-release-tag.md)), the digests of the artifact files, and the export of [FR-069](./FR-069-test-result-export-is-consumed-as-data.md) from the evidence store in `evidence_artifact` |
| same | same | `attest` | `contents: read`, `id-token: write`, `attestations: write` | Engineering Assurance tooling and the attestation actions; no repository code |
| `.github/workflows/change-receipt.yml` | `workflow_call`, input `commit` (string) | `gates` | `contents: read` | The repository's gates from the receipt policy ([FR-075](./FR-075-push-to-main-receipt-workflow.md)); nothing else |
| same | same | `record` | `contents: read`, `id-token: write` | Engineering Assurance tooling only; no repository code |
| same | same | `notes` | `contents: write` | Engineering Assurance tooling only: fetch, merge and push the notes ref ([FR-076](./FR-076-receipt-storage-in-git-notes.md)) |
| `.github/workflows/missing-receipt-check.yml` | `schedule`, `workflow_dispatch` | `check` | `contents: read` | `receipt-missing` ([FR-077](./FR-077-missing-receipt-check.md)) |
| `.github/workflows/receipt.yml` | `push` to the default branch | `receipt` | calls `change-receipt.yml` with `contents: write`, `id-token: write` | the reusable workflow |

A repository that adopts the features adds its own callers. `ci.yml` is unchanged. The approval review gate workflow and the main-branch ruleset belong to [FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md) and the EA-109 work; this specification does not define them.

Each caller uploads the release files produced by its `build` job with `actions/upload-artifact`; the common `artifact_name` input default is `release-artifacts`. The common `evidence_artifact` input default is `release-evidence`. The reusable workflow downloads the named artifacts in its `verify` job. The `attest` job invokes no caller or repository code. In `verify`, the workflow passes the caller's exact published Quoin package version in `quoin_tooling_tag` through `env` and installs it with `npm install --ignore-scripts --registry="$NPMJS_REGISTRY" "@agent-ix/quoin@$QUOIN_TOOLING_TAG"`, where `NPMJS_REGISTRY` is explicitly set to the HTTPS registry origin with host `registry.npmjs.org`; it does not use the runner's ambient npm registry configuration, `latest`, or a caller-built Quoin binary.

For adopters with a release path, attestation is added beside that path. Engineering Assurance has no release path or retained evidence store; its tag-attestation caller remains deferred to EA-122:

| Repository | Existing release path preserved | Attestation caller | Per-job permissions | Retained evidence source |
| --- | --- | --- | --- | --- |
| engineering-assurance | None | Deferred to EA-122 until a retained evidence store exists; no EA-owned tag-attestation caller is present | N/A until a caller exists | No retained store exists; the reusable workflow fails closed with `no_retained_evidence_store` before writing a statement if its evidence artifact is absent or invalid |
| quoin | Manually dispatched `.github/workflows/release.yml` npm repack/publish flow and `.github/workflows/native-release.yml` release flow | `.github/workflows/tag-attestation.yml`, triggered by a push of a `v*` tag, with separate `build` and `attest` jobs | `build`: `contents: read`; `attest`: calls the reusable workflow with `contents: read`, `id-token: write`, and `attestations: write` | Run documented `make evidence`; upload the generated `spec/evidence` store only when it is valid, has a current run record for the tagged commit, and has real bindings from passing cases. Otherwise fail closed with `no_retained_evidence_store` and write no statement. The committed empty skeleton is not a valid store. |
| quire-cli | Manually dispatched `.github/workflows/release.yml` flow | `.github/workflows/tag-attestation.yml`, triggered by a push of a `v*` tag, with separate `build` and `attest` jobs | `build`: `contents: read`; `attest`: calls the reusable workflow with `contents: read`, `id-token: write`, and `attestations: write` | No retained store exists; fail closed with `no_retained_evidence_store` and write no statement |
| quire-rs | Manually dispatched `.github/workflows/wheels.yml` release flow | `.github/workflows/tag-attestation.yml`, triggered by a push of a `v*` tag, with separate `build` and `attest` jobs | `build`: `contents: read`; `attest`: calls the reusable workflow with `contents: read`, `id-token: write`, and `attestations: write` | Its `spec/evidence` contains benchmark measurement JSON only, not retained release test runs and bindings; fail closed with `no_retained_evidence_store` and write no statement |

Each additive caller runs independently of its preserved release path: its result does not gate an existing publish job or workflow. In particular, the npm Trusted Publisher contract remains in quoin's existing `release.yml`.

Quoin's tag-attestation `build` job SHALL run the documented `make evidence` producer before uploading `spec/evidence` as the `release-evidence` workflow artifact. It SHALL upload only a successful producer result whose generated store validates, contains at least one run record for the tagged commit, and contains at least one real binding backed by a passing case in those records. Missing, empty, malformed, stale, or unbound output SHALL fail closed with `no_retained_evidence_store` before upload or statement creation; the committed empty `bindings.json` skeleton is never a valid store. Engineering Assurance, quire-cli and quire-rs do not currently have a retained release test store with run records and `bindings.json`; quire-rs's benchmark measurement JSON is not a retained release Test Result source. Their tag-attestation runs fail closed with `no_retained_evidence_store` and write no statement until a valid store exists. A caller SHALL NOT fabricate an empty evidence store or submit a caller-built Test Result statement. When present, the reusable workflow obtains the Test Result only by running the existing Quoin export contract over retained run records and `bindings.json` ([FR-069](./FR-069-test-result-export-is-consumed-as-data.md), [Quoin FR-185](ix://agent-ix/quoin/FR-185)).

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
- Each adopter SHALL upload its release files from the caller's `build` job using the shared `artifact_name` default.
- Each adopter that has a retained Quoin release evidence store SHALL upload that existing store using the shared `evidence_artifact` default.
- Quoin's tag-attestation `build` job SHALL run `make evidence` before producing its evidence artifact.
- Quoin's tag-attestation `build` job SHALL upload its generated store only when the producer succeeds and the store validates with a current run record for the tagged commit and at least one real binding from a passing case.
- Quoin's tag-attestation caller SHALL fail with `no_retained_evidence_store` before upload or statement creation when the generated store is absent, empty, malformed, stale, or has no real bindings.
- The committed empty `bindings.json` skeleton SHALL NOT count as a retained evidence store.
- A caller without a retained evidence store SHALL fail with `no_retained_evidence_store` before writing any statement.
- Benchmark measurement JSON without retained release test runs and `bindings.json` SHALL NOT satisfy the retained evidence requirement.
- A caller SHALL NOT create an empty retained evidence store or supply a caller-built Test Result statement.
- The reusable workflow SHALL obtain a Test Result only through the Quoin export contract for retained run records and `bindings.json`.
- An additive adopter attestation caller SHALL NOT gate an existing release or publish workflow.
- Each adopter's `build` job SHALL declare only `contents: read`.
- Each adopter's `attest` job SHALL only call the reusable workflow.
- Each adopter's `attest` job SHALL grant the reusable workflow `contents: read`, `id-token: write` and `attestations: write`.
- Each adopter's `attest` job SHALL NOT execute caller or repository code.
- Each adopter SHALL pass an `ea_tooling_tag` equal to the ref in the reusable-workflow `uses` value.
- The reusable workflow SHALL use `ea_tooling_tag` to download the matching trusted Engineering Assurance tooling.
- Each caller SHALL pass `quoin_tooling_tag` as an exact published version of `@agent-ix/quoin`.
- The `verify` job SHALL pass `quoin_tooling_tag` through `env`, not interpolate the workflow input into a `run` value.
- The `verify` job SHALL set `NPMJS_REGISTRY` explicitly to the HTTPS registry origin with host `registry.npmjs.org`.
- The `verify` job SHALL install Quoin with `npm install --ignore-scripts --registry="$NPMJS_REGISTRY" "@agent-ix/quoin@$QUOIN_TOOLING_TAG"`.
- The `verify` job SHALL NOT install Quoin as `latest` or use a caller-built Quoin binary.
- Each caller SHALL pass an explicit `record_path`.
- The reusable workflow SHALL read the `allowed_signers_path` from the protected base branch, never from the tagged tree.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-070-AC-1 | Engineering Assurance's test parses only its checked-in `.github/workflows/release-attestation.yml` reusable workflow and confirms its `workflow_call` trigger, jobs and per-job permissions match the table; each adopter's tests parse only its own checked-in caller workflows. No parsed workflow declares `pull_request_target` or `workflow_run`. | Test |
| FR-070-AC-2 | No `run:` value in any file contains an expression over `github.ref_name`, `github.head_ref`, an event title or body, a commit message or `inputs.`; each such value appears only under `env:`. | Test |
| FR-070-AC-3 | Every `uses:` in a job with `id-token: write` ends in a 40-hex SHA, and every checkout step has `persist-credentials: false`. | Test |
| FR-070-AC-4 | The `gates` job holds only `contents: read`, the `record` job runs no step that executes a repository file or a command from the receipt policy, and the `notes` job holds no `id-token`. | Test |
| FR-070-AC-5 | No step in any workflow file listed in this requirement invokes an agent harness or model client. Engineering Assurance inspects only its reusable workflow; each adopter inspects only its own checked-in workflow files. | Inspection |
| FR-070-AC-6 | A `change-receipt.yml` run with a `commit` input that is not the pushed commit fails with `receipt_input_commit_mismatch`, and a `release-attestation.yml` run with a `tag` input that is not the triggering tag fails with `release_input_tag_mismatch`, in both cases before any signing step. | Test |
| FR-070-AC-7 | The `gates` job declares no `secrets` and no `environment`, and its `runs-on` is a GitHub-hosted label or an ephemeral runner label. | Test |
| FR-070-AC-8 | Each adopter (Quoin, quire-cli and quire-rs) preserves its existing manual release path, including its `workflow_dispatch` trigger, publish gating and permissions, and adds a separate `tag-attestation.yml` workflow triggered by `push` of a `v*` tag. The additive workflow grants only `contents: read` to `build` and calls the attestation reusable workflow with `contents: read`, `id-token: write` and `attestations: write`; where its caller invokes a shared release workflow, it passes `publish: false`. The existing manual publish input continues to gate publishing, and the tag-attestation path neither starts nor gates manual publishing. Each adopter verifies this contract with a test in its own repository, parsing only its own checked-in workflow files. Engineering Assurance tests cover only its reusable workflow; its own tag-attestation caller is deferred to EA-122 until a retained store exists. Engineering Assurance does not retain copies or byte-pinned snapshots of adopter workflows. | Test |
| FR-070-AC-9 | Every caller uploads its release files from `build` under the shared `artifact_name` default `release-artifacts`; Quoin's build job runs `make evidence` and uploads the generated `spec/evidence` store under the shared `evidence_artifact` default `release-evidence` only after it passes the validity conditions in AC-10; the reusable workflow downloads named artifacts in `verify`. | Test |
| FR-070-AC-10 | Quoin's tag-attestation build job runs documented `make evidence` before upload. It uploads only when the command succeeds and the generated store validates, contains at least one run record for the tagged commit, and contains at least one real binding backed by a passing case in those records. Missing, empty, malformed, stale, or unbound output, including the committed empty `bindings.json` skeleton, fails with `no_retained_evidence_store` before upload or statement creation. The Quoin exporter reads retained run records and `bindings.json`, uses artifact subjects computed by the reusable workflow, and performs no Git or network access. | Test |
| FR-070-AC-11 | Engineering Assurance has no retained evidence store or tag-attestation caller; its reusable workflow fails with `no_retained_evidence_store` before writing a statement when the evidence artifact is absent or invalid. Quire-cli has no retained store, and quire-rs has only benchmark measurement JSON without retained release test runs or `bindings.json`; their tag-attestation callers fail with `no_retained_evidence_store` before writing any statement. No adopter uploads an empty store or accepts a caller-built Test Result. | Test |
| FR-070-AC-12 | The reusable workflow declares `ea_tooling_tag`, `record_path` and `allowed_signers_path`; it uses `ea_tooling_tag` to download the matching trusted Engineering Assurance tooling and reads `allowed_signers_path` from the protected base branch even when the tagged tree contains a different file. Each adopter's own test confirms its caller passes an `ea_tooling_tag` equal to the static reusable-workflow ref after `@` and supplies `record_path` explicitly. | Test |
| FR-070-AC-13 | A YAML-parsing integration test confirms every caller passes an exact published `@agent-ix/quoin` version in `quoin_tooling_tag`, maps it through `env`, explicitly sets `NPMJS_REGISTRY` to the HTTPS registry origin with host `registry.npmjs.org`, and installs exactly that version with `npm install --ignore-scripts --registry="$NPMJS_REGISTRY" "@agent-ix/quoin@$QUOIN_TOOLING_TAG"`; no `run` value interpolates the workflow input, and the job uses neither ambient npm registry configuration, `latest`, nor a caller-built binary. | Test |

## Dependencies

- **Upstream**: [FR-066](./FR-066-release-attestation-set-and-subjects.md), [FR-068](./FR-068-verification-summary-signed-by-the-pinned-workflow.md), [FR-075](./FR-075-push-to-main-receipt-workflow.md), [FR-077](./FR-077-missing-receipt-check.md), [FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md), [NFR-011](../non-functional/NFR-011-signing-jobs-run-no-agent.md).
