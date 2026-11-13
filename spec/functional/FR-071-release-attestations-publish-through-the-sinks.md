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

Tracked by Linear EA-97. A release is published as a record of kind `release_attestation` through the sink interface ([FR-054](./FR-054-record-sink-interface-and-implementations.md)), with the repository's resolved sink ([FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md)), the split of [FR-056](./FR-056-public-and-private-record-split.md) and the leak check of [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md). The public form carries the digests of the attestation statements and the release commit, the verdict `release` with value `released` or `failed`, and timestamps; the full private form carries the rest. The release is not published when the leak check refuses.

The attestation action writes to the public Sigstore transparency log for a public repository and to the private GitHub instance for a private one. That upload is a public-log write in the sense of [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md). The workflow therefore runs the attestation action for a public repository only when its resolved sink is `public_log` and the leak check passed. A public repository whose resolved sink is the private store or `none` gets no attestation: the workflow fails with `release_attestation_public_log_required`, because the release set cannot be signed without the public log.

## Behavior

- The release attestation workflow SHALL hand the release record to the sink interface as a record of kind `release_attestation`.
- The release attestation workflow SHALL run the leak check of FR-057 before the attestation action for a public repository.
- The release attestation workflow SHALL run the attestation action for a public repository only when the resolved sink is `public_log`.
- The release attestation workflow SHALL fail with `release_attestation_public_log_required` for a public repository whose resolved sink is the private store or `none`.
- The release attestation workflow SHALL NOT call the attestation action when the leak check refuses.
- The release attestation workflow SHALL write the full private form to the private store before the leak check.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-071-AC-1 | A public repository on the public sink with a clean leak check publishes a trimmed record of kind `release_attestation` whose bytes hold digests, verdict and timestamps only. | Test |
| FR-071-AC-2 | A public repository referencing a private Linear team in the release record is refused by the leak check, and the stand-in attestation action is not called. | Test |
| FR-071-AC-3 | A public repository configured `private_store` or `none` fails with `release_attestation_public_log_required` and the stand-in log receives no call. | Test |
| FR-071-AC-4 | A private repository on the private store attests through the private instance stand-in and makes no public-log call. | Test |
| FR-071-AC-5 | A refused release still has its full private form in the private store. | Test |

## Dependencies

- **Upstream**: [FR-054](./FR-054-record-sink-interface-and-implementations.md), [FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md), [FR-056](./FR-056-public-and-private-record-split.md), [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md), [FR-066](./FR-066-release-attestation-set-and-subjects.md).
- **Downstream**: [FR-070](./FR-070-release-and-receipt-workflow-files.md).
