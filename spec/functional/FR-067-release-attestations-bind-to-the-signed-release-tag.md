---
id: FR-067
title: "Attest a release only after the signed tag and the role policy verify"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-047"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-041"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-061"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-064"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-053"
    type: "references"
---

# FR-067: Attest a release only after the signed tag and the role policy verify

## Description

Tracked by Linear EA-97. The release attestation workflow attests a release only when the person-signed release tag verifies. It runs `release-verify` ([FR-047](./FR-047-release-record-and-signed-release-tag.md)), which runs the `approval-verify` verification of [FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md), against an allowed-signers file read from the protected base branch ([FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md)); the file of the tag's own commit is not trusted. Only a `released` result lets the workflow write the verification summary and the other statements; any other status fails the workflow and writes nothing.

The release commit lands through a pull request. The workflow also evaluates the role policy ([FR-061](./FR-061-role-policy-and-distinct-individuals.md)) over the approval statements bound to that pull request, verified under [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md), so the platform approvals of the change that adds the release record satisfy the same required roles as any change. The signed tag stays the release decision ([NFR-007](../non-functional/NFR-007-no-release-without-a-person.md)); the roles are an additional condition, not a second decision.

The binding to the tag is carried by the verification summary: its resource names the tag and the statement set names the release commit as a subject ([FR-068](./FR-068-verification-summary-signed-by-the-pinned-workflow.md)). A verifier that rereads the tag, and finds it altered, moved or re-signed by another key, gets a different `release-verify` result and rejects the set.

## Behavior

- The release attestation workflow SHALL run `release-verify` for the tag before writing any statement.
- The release attestation workflow SHALL read the allowed-signers file from the protected base branch.
- The release attestation workflow SHALL write no statement when `release-verify` reports a status other than `released`.
- The release attestation workflow SHALL evaluate the role policy over the approval statements of the release commit's pull request with the evaluation function of FR-061.
- The release attestation workflow SHALL write no statement when the role evaluation is not `satisfied`.
- The release attestation workflow SHALL fail visibly with the named error from the role evaluation when a required role is unstaffed.
- The release attestation workflow SHALL NOT treat a satisfied role evaluation as the release decision.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-067-AC-1 | A fixture release with a tag signed by the record's decision owner and a satisfied role evaluation yields the full attestation set. | Test |
| FR-067-AC-2 | The same release with an unsigned tag, a tag signed by an unmapped key, or a tag whose `Approved-Record-Digest` differs yields no statement and a failed workflow run reporting the `release-verify` status. | Test |
| FR-067-AC-3 | A release whose allowed-signers file is changed on the tag's commit but not on the protected base branch is verified against the base branch's file. | Test |
| FR-067-AC-4 | A release with a valid tag and a role policy of three roles with only two staffed verified statements yields no statement and names the missing role; a policy that lists an unstaffed role fails with `approval_role_unstaffed` naming it. | Test |
| FR-067-AC-5 | A fixture tag moved to another commit after attestation makes the verification of the set fail. | Test |
| FR-067-AC-6 | A release with a satisfied role evaluation and no signed tag yields no statement. | Test |

## Dependencies

- **Upstream**: [FR-047](./FR-047-release-record-and-signed-release-tag.md), [FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md), [FR-061](./FR-061-role-policy-and-distinct-individuals.md), [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md), [FR-053](./FR-053-snapshot-and-attestations-never-replace-the-decision.md), [NFR-007](../non-functional/NFR-007-no-release-without-a-person.md).
- **Downstream**: [FR-068](./FR-068-verification-summary-signed-by-the-pinned-workflow.md).
