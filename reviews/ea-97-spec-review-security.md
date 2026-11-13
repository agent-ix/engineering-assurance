---
id: SR-20261113170517
title: "Security review of release attestations, receipts and their workflows (engineering-assurance PR 232)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-066..FR-078, spec/non-functional/NFR-011 (PR #232 diff, head 5bab3f1)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-070
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-075
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-076
    type: references
  - target: ix://agent-ix/engineering-assurance/NFR-011
    type: references
---

# SR-20261113170517: Security review of release attestations, receipts and their workflows (engineering-assurance PR 232)

## Summary

Ticket: EA-97 (EA-98). Security lens (spec-security-analysis); the SpecReview `analysis` enum has no security value, so this file carries `base`. Applicable practice: SLSA v1.0 build track and verification summary, in-toto
attestation framework, Sigstore keyless signing, and GitHub Actions hardening (least-privilege
`permissions`, no untrusted `${{ }}` interpolation into `run:`, third-party actions pinned by full commit
SHA in jobs that hold `id-token: write`). All are open standards and vendor guidance; nothing paywalled.

Checked clean: agents never sign, and the signer is the CI workflow identity (NFR-011, FR-074); identity
matching is exact and a pull-request ref is rejected (FR-068-AC-3, FR-078-AC-2); policies come from the protected
base branch; the public form is built by selection under closed grammars; git notes carry only statements
with no text or model name (FR-076-AC-5); the release-attestation path is gated by the leak check before
the public log (FR-071).

The SHA pin of third-party actions recommended below is load-bearing, not ceremony. If it is ignored, a moved
tag runs foreign code in a job that can mint the trusted signing identity.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The repository's gates run in the signing job. FR-075 re-runs the receipt-policy `argv` commands (repository code, written by agents) "in a job that holds no agent", and FR-075-AC-6 only checks that "the signing step runs after every gate step", that is, the same job. Code that runs in a job with `id-token: write` can mint the change-receipt OIDC token and sign any statement or forge the test result, so the evidence becomes the agent-written code's own claim. That defeats NFR-011's rationale. Fix: require the gates to run in a separate job with `permissions: contents: read` and no `id-token`/`attestations`/`contents: write`, handing results to the signing job as an artifact. The signing job runs only Engineering Assurance's own pinned tooling and executes nothing from the pushed commit. Add an NFR-011 metric "count of signing jobs that execute code from the repository under change: 0" and replace FR-075-AC-6 accordingly. The same rule applies to any build or export step in `release-attestation.yml`. | spec/functional/FR-075-push-to-main-receipt-workflow.md:22, 47; spec/non-functional/NFR-011-signing-jobs-run-no-agent.md |
| FND-002 | high | The workflow attack surface is unspecified. FR-070 names the five files but not the `workflow_call` inputs (names, types, how artifacts reach the reusable workflow) or the per-job `permissions`. It requires nothing on untrusted-input handling: tag names, branch names, commit messages, PR titles and inputs must reach scripts through `env:`, never by `${{ }}` interpolation into `run:`. It does not require pinning third-party actions (`actions/attest*`, checkout, installers) by full commit SHA in signing jobs, `persist-credentials: false` on checkout, or forbid `pull_request_target`/`workflow_run` triggers. "Each signing job SHALL declare the least permissions it needs" is not checkable against anything. Fix: add to FR-070 a per-file, per-job table of triggers, inputs and exact `permissions`, and Behavior bullets plus ACs (YAML-parsing tests) for env-only interpolation, SHA-pinned third-party actions in jobs holding `id-token: write`, and the trigger allowlist. | spec/functional/FR-070-release-and-receipt-workflow-files.md:24-30, 39 |
| FND-003 | high | The receipt signing path does not follow visibility. Receipts are Sigstore bundles in git notes (decision 8, direct signing), but FR-076 runs the leak check only "for a public repository" and names no non-public signing configuration. Keyless signing against the public-good Fulcio/Rekor for a private repository writes the private repository and workflow names to the public transparency log (an NFR-009 and FR-057 violation). For a public repository whose sink is `private_store` or `none`, FR-071's release rule (fail with `..._public_log_required`) has no receipt counterpart. Fix: in FR-074/FR-076 name the signing configuration per resolved sink (public log only after FR-057 passes; otherwise a configuration that writes to no public log, for example the platform's private instance, or fail with a named code), mirror FR-071's public-repository rule for receipts, and add ACs with a stand-in public log receiving zero calls for a private repository's receipt. | spec/functional/FR-076-receipt-storage-in-git-notes.md:22; spec/functional/FR-074-per-change-receipt-bundle-records-failures-too.md:22 |
| FND-004 | medium | "The receipt job holds `contents: write` for the notes ref only". A `GITHUB_TOKEN` permission is repository-wide and rulesets cannot target `refs/notes/*`, so the scoping claimed is not enforceable. Fix: state that `contents: write` is repository-wide; confine it to a final job that only pushes `refs/notes/assurance-receipts` with Engineering Assurance code and runs no repository code; rely on the default-branch ruleset for `main`. | spec/functional/FR-070-release-and-receipt-workflow-files.md:30 |
| FND-005 | medium | The digest of the agent BOM is public (in the note and the change-record predicate) and its content has low entropy: a harness, a model from a small set, and plugin versions. Anyone can enumerate candidate BOMs and confirm which model built a change, which defeats keeping model names private. Fix: require the BOM to carry a random `serialNumber` (`urn:uuid:` v4, or the record's `record_id`), and add an AC that two BOMs from identical session descriptors have different digests. | spec/functional/FR-072-change-record-predicate.md:32; spec/functional/FR-073-agent-bom-in-cyclonedx-1-7.md |
| FND-006 | medium | The verification summary can self-certify SLSA build levels. The build is out of scope (FR-066), the artifacts come from the caller, and the same reusable workflow writes the provenance and then "verifies" build levels from it. Build-track levels need the builder to be isolated from the caller, so a caller can hand any bytes to be attested under the Engineering Assurance identity. Fix: either build inside the reusable workflow (the caller passes a build command, and the provenance builder id is that workflow), or require `verifiedLevels` to be empty until a build-isolation requirement exists. Either way, state how artifacts reach the workflow. | spec/functional/FR-068-verification-summary-signed-by-the-pinned-workflow.md:22; spec/functional/FR-066-release-attestation-set-and-subjects.md:24, 28 |

## Verdict

Not mergeable: FND-001 to FND-003 are design-level holes in the signing boundary that the workflow code
would inherit. FND-004 to FND-006 belong to the same round.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | The reusable-workflow identity matches no trigger event and no caller workflow. Any consumer workflow that runs at the default branch ref (`schedule`, `workflow_dispatch`, `issue_comment`, or a `pull_request_target` in the consumer repository) can call `change-receipt.yml` and get a certificate that matches the `receipt_writer` entry. It can also pass any `commit` input, including a fork pull-request head that is reachable in the repository. Likewise `release.yml`'s `tag` input can name a tag other than the one that triggered the run. Fix: add the trigger `event` (`push`) and the caller `source_workflow_path` to the `release_attester` and `receipt_writer` entries in FR-064, matched against the certificate. Require the reusable workflows to reject an `inputs.commit` other than the pushed commit and an `inputs.tag` other than the triggering tag, or drop those inputs and read the trigger context. Add ACs. | spec/functional/FR-064-verify-approval-statement-with-workflow-identity-policy.md:40 |
| FND-008 | low | FR-075-AC-6's "the forged result file is verified against the digest of the artifact the signing job received" is vacuous. The `gates` job runs the merged code and can write any result file, and the digest only shows that the bytes downloaded equal the bytes uploaded, so a forged file passes. The `gates` job is also not stated to receive no secrets or to run on an ephemeral runner. Fix: drop the digest clause; state in FR-074 that gate results are as reported by the `gates` job, which runs the merged code (the receipt binds the commit, so a falsified result is attributable); require the `gates` job to receive no secrets and to run on a GitHub-hosted or ephemeral runner. | spec/functional/FR-075-push-to-main-receipt-workflow.md:51 |

## Dispositions

Round 1, head 428c2bd (fix commits 5bab3f1..428c2bd).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
