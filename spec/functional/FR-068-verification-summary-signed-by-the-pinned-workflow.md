---
id: FR-068
title: "Sign one verification summary of the release with a pinned Engineering Assurance reusable workflow"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-067"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-064"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-066"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-010"
    type: "references"
---

# FR-068: Sign one verification summary of the release with a pinned Engineering Assurance reusable workflow

## Description

Tracked by Linear EA-97. The release set contains one SLSA Verification Summary Attestation v1, signed by the Engineering Assurance reusable workflow `release-attestation.yml` ([FR-070](./FR-070-release-and-receipt-workflow-files.md)). The signing identity is the identity of that workflow at the ref listed in the workflow-identity policy ([FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md)); a consumer verifies the set with the stock `gh attestation verify` and its `--signer-workflow` option naming that workflow, or with the Engineering Assurance `attestation-verify` command, which also performs the checks of this requirement.

The summary states what the workflow verified: the verifier id is the workflow identity, the time is the verification time, the resource is the repository at the release tag reference, the policy names the role policy of [FR-061](./FR-061-role-policy-and-distinct-individuals.md), the input attestations are the other three statements by digest, and the result is `PASSED` only when [FR-047](./FR-047-release-record-and-signed-release-tag.md) reported `released`, the role evaluation was satisfied, and the three input statements verified. The levels it lists are the build levels that the verifier verified from the provenance statement and no others; with none verified the list is empty. The statement's subjects are the artifact files of [FR-066](./FR-066-release-attestation-set-and-subjects.md) and the release commit as a git commit digest. The summary adds no custom member to the third-party predicate. Signing uses the delegated stack of [NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md).

## Behavior

- The reusable workflow SHALL sign the verification summary with its own workflow identity.
- The reusable workflow SHALL set the result to `PASSED` only when `release-verify` reported `released`, the role evaluation was satisfied and the three input statements verified.
- The reusable workflow SHALL list in the summary only the build levels it verified from the provenance statement.
- The reusable workflow SHALL name the other three statements as input attestations by digest.
- The reusable workflow SHALL add the release commit as a subject by git commit digest.
- The attestation verifier SHALL reject a summary whose signing identity is not the workflow identity listed in the workflow-identity policy.
- The attestation verifier SHALL reject a summary signed by the listed workflow at a ref other than the listed ref.
- The reusable workflow SHALL NOT add a member to the verification summary predicate.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-068-AC-1 | The fixture summary verifies with the `attestation-verify` command and with a stand-in for `gh attestation verify --signer-workflow` naming the reusable workflow. | Test |
| FR-068-AC-2 | A summary with result `PASSED` is produced only for the case in which all three conditions hold, over every combination of the three conditions. | Property |
| FR-068-AC-3 | A summary signed by another workflow, or by the listed workflow at a pull request ref, is rejected as `untrusted_signer`. | Test |
| FR-068-AC-4 | The summary's input attestations equal the digests of the three other statements, and changing one of them makes the verification fail. | Test |
| FR-068-AC-5 | The summary carries the release commit as a git commit subject and the tag reference in its resource, and the verifier fails it when the tag reference resolves to a different commit. | Test |
| FR-068-AC-6 | The summary carries no predicate member beyond those of the third-party schema, and its level list is empty for a fixture with no verified provenance level. | Test |

## Dependencies

- **Upstream**: [FR-066](./FR-066-release-attestation-set-and-subjects.md), [FR-067](./FR-067-release-attestations-bind-to-the-signed-release-tag.md), [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md), [NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md).
- **Downstream**: [FR-070](./FR-070-release-and-receipt-workflow-files.md), [FR-071](./FR-071-release-attestations-publish-through-the-sinks.md).
