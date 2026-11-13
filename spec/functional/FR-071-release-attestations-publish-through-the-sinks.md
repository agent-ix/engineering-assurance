---
id: FR-071
title: "Publish release attestations through the record sinks and never write a private repository to a public log"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-054"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-055"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-056"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-057"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-066"
    type: "references"
---

# FR-071: Publish release attestations through the record sinks and never write a private repository to a public log

## Description

Tracked by Linear EA-97. A release is published as a record of kind `release_attestation` through the sink interface ([FR-054](./FR-054-record-sink-interface-and-implementations.md)), with the repository's resolved sink ([FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md)), the split of [FR-056](./FR-056-public-and-private-record-split.md) and the leak check of [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md), and only after [FR-067](./FR-067-release-attestations-bind-to-the-signed-release-tag.md) reported `released`. The public form carries `hashes.statement` equal to the digest of the verification summary (`sha256:` followed by 64 lowercase hexadecimal characters computed over the RFC 8785 canonical bytes of the named JSON document; the summary names the other three statements by digest), `hashes.subject_commit` equal to the release commit, the verdict `release` with value `released`, and timestamps. The full private form carries the rest. No release is published for any other outcome.

The attestation action writes to the public Sigstore transparency log for a public repository and to the platform's private instance for a private one. The public write is a public-log write in the sense of [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md). For a public repository the workflow runs the attestation action only when the resolved sink is `public_log` and the leak check passed; a public repository whose resolved sink is the private store or `none` fails with `release_attestation_public_log_required`. For a private repository the action uses the private instance only; if that instance is unavailable the workflow fails with `release_attestation_private_signing_unavailable` and never falls back to the public log. A repository whose resolved sink is `none` fails with `release_attestation_private_store_required`, because the statements the role step needs are not retained.

## Behavior

- The release attestation workflow SHALL hand the release record to the sink interface as a record of kind `release_attestation` only after `release-verify` reported `released`.
- The release attestation workflow SHALL set `hashes.statement` to the digest of the verification summary and `hashes.subject_commit` to the release commit.
- The release attestation workflow SHALL run the leak check of FR-057 before the attestation action for a public repository.
- The release attestation workflow SHALL run the attestation action for a public repository only when the resolved sink is `public_log`.
- The release attestation workflow SHALL fail with `release_attestation_public_log_required` for a public repository whose resolved sink is the private store or `none`.
- The release attestation workflow SHALL fail with `release_attestation_private_signing_unavailable` when a private repository's private signing instance is unavailable.
- The release attestation workflow SHALL fail with `release_attestation_private_store_required` for a repository whose resolved sink is `none`.
- The release attestation workflow SHALL write the full private form to the private store before the leak check.
- The release attestation workflow SHALL NOT call the attestation action when the leak check refuses.
- The release attestation workflow SHALL NOT use a public transparency log for a private repository.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-071-AC-1 | A public repository on the public sink with a clean leak check publishes a trimmed record of kind `release_attestation` whose bytes hold the summary digest, the release commit, the verdict `released` and timestamps only. | Test |
| FR-071-AC-2 | A public repository referencing a private Linear team in the release record is refused by the leak check, and the stand-in attestation action is not called. | Test |
| FR-071-AC-3 | A public repository configured `private_store` or `none` fails with `release_attestation_public_log_required` and the stand-in log receives no call. | Test |
| FR-071-AC-4 | A private repository on the private store attests through the private instance stand-in and the stand-in public log receives zero calls; with the private instance stand-in unavailable it fails with `release_attestation_private_signing_unavailable` and the public log stand-in still receives zero calls. | Test |
| FR-071-AC-5 | A repository on sink `none` fails with `release_attestation_private_store_required`. | Test |
| FR-071-AC-6 | A refused release still has its full private form in the private store, and a `not_released` outcome publishes no record. | Test |

## Dependencies

- **Upstream**: [FR-054](./FR-054-record-sink-interface-and-implementations.md), [FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md), [FR-056](./FR-056-public-and-private-record-split.md), [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md), [FR-066](./FR-066-release-attestation-set-and-subjects.md), [FR-067](./FR-067-release-attestations-bind-to-the-signed-release-tag.md).
- **Downstream**: [FR-070](./FR-070-release-and-receipt-workflow-files.md).
