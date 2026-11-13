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
  - target: "ix://agent-ix/engineering-assurance/FR-060"
    type: "references"
---

# FR-067: Attest a release only after the signed tag and the role policy verify

## Description

Tracked by Linear EA-97. The release attestation workflow attests a release only when the person-signed release tag verifies. It runs `release-verify` ([FR-047](./FR-047-release-record-and-signed-release-tag.md)), which runs the `approval-verify` verification of [FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md), against an allowed-signers file read from the protected base branch ([FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md)); the file of the tag's own commit is not trusted. Only a `released` result lets the workflow write any statement or publish any record; any other status fails the workflow and writes nothing.

The workflow then evaluates the role policy ([FR-061](./FR-061-role-policy-and-distinct-individuals.md)). The role step is attestation workflow code and lives outside the release module, which reads no approval statement ([FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md)). The signed tag stays the only release decision ([NFR-007](../non-functional/NFR-007-no-release-without-a-person.md)); the roles are an additional condition. The role step works as follows.

- The pull request is found from the statements, not from the tag: the statements to evaluate are those whose `merged_commit` equals the commit the tag peels to ([FR-060](./FR-060-approval-binds-to-the-merged-head-commit.md)). The tagged commit is the release commit of [FR-047](./FR-047-release-record-and-signed-release-tag.md), so a squash or merge-commit merge of the release pull request must produce it and the tag must point at that merged commit, not at the pull request head. A rebase merge binds only the rebased tip; a tag on any other rebased commit finds no statements.
- If the source reports a merged pull request whose merge result is the tagged commit but no statements exist yet, the statement writer has not run: the result is `release_roles_statements_pending`, a retryable result, and nothing is written.
- If no merged pull request has the tagged commit as its merge result (a direct push, or a rebased commit that is not the tip), the result is `release_roles_no_pull_request`.
- If the repository's sink is `none`, statements are not retained ([FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md)) and the result is `release_roles_statements_not_retained`.
- Otherwise the statements are verified under [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md) and evaluated with the function of [FR-061](./FR-061-role-policy-and-distinct-individuals.md); an unsatisfied evaluation, including `approval_role_unstaffed`, writes nothing and reports the evaluation's named result.

The verification summary names the tag in its resource and the release commit as a subject ([FR-068](./FR-068-verification-summary-signed-by-the-pinned-workflow.md)). A verifier that rereads the tag, and finds it altered, moved or re-signed by another key, gets a different `release-verify` result and rejects the set.

## Behavior

- The release attestation workflow SHALL run `release-verify` for the tag before writing any statement.
- The release attestation workflow SHALL read the allowed-signers file from the protected base branch.
- The release attestation workflow SHALL write nothing when `release-verify` reports a status other than `released`.
- The release attestation workflow SHALL select the statements whose `merged_commit` equals the commit the tag peels to.
- The release attestation workflow SHALL evaluate the role policy over those statements with the evaluation function of FR-061.
- The release attestation workflow SHALL write nothing and report `release_roles_statements_pending` when the source reports a merged pull request for the tagged commit and no statement exists.
- The release attestation workflow SHALL write nothing and report `release_roles_no_pull_request` when no merged pull request has the tagged commit as its merge result.
- The release attestation workflow SHALL write nothing and report `release_roles_statements_not_retained` when the repository's sink is `none`.
- The release attestation workflow SHALL write nothing and report `approval_role_unstaffed` naming the role when a required role is unstaffed.
- The release attestation workflow SHALL write nothing when the role evaluation is not `satisfied`.
- The release module SHALL NOT read an approval statement.
- The release attestation workflow SHALL NOT treat a satisfied role evaluation as the release decision.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-067-AC-1 | A fixture release with a tag signed by the record's decision owner, a squash-merged release pull request and a satisfied role evaluation yields the full attestation set. | Test |
| FR-067-AC-2 | The same release with an unsigned tag, a tag signed by an unmapped key, or a tag whose `Approved-Record-Digest` differs yields no statement and a failed workflow run reporting the `release-verify` status. | Test |
| FR-067-AC-3 | A release whose allowed-signers file is changed on the tag's commit but not on the protected base branch is verified against the base branch's file. | Test |
| FR-067-AC-4 | A release with a valid tag and a policy of three roles with two covered yields nothing and names the missing role; a policy that lists an unstaffed role yields `approval_role_unstaffed` naming it. | Test |
| FR-067-AC-5 | A tag pushed before the statement writer ran yields `release_roles_statements_pending`; after the statements exist the same run succeeds. | Test |
| FR-067-AC-6 | A tag on a commit pushed directly, and a tag on a rebased commit that is not the tip, yield `release_roles_no_pull_request`. | Test |
| FR-067-AC-7 | A repository with sink `none` yields `release_roles_statements_not_retained`. | Test |
| FR-067-AC-8 | A tag pointing at the release pull request's head commit instead of the merged commit finds no statements and yields `release_roles_no_pull_request`. | Test |
| FR-067-AC-9 | A fixture tag moved to another commit after attestation makes the verification of the set fail, and a satisfied role evaluation with no signed tag yields no statement. | Test |
| FR-067-AC-10 | Static inspection shows no code path of the release module reads an approval statement and that the role step is in the attestation workflow code (FR-065-AC-5). | Inspection |

## Dependencies

- **Upstream**: [FR-047](./FR-047-release-record-and-signed-release-tag.md), [FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md), [FR-061](./FR-061-role-policy-and-distinct-individuals.md), [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md), [FR-053](./FR-053-snapshot-and-attestations-never-replace-the-decision.md), [FR-060](./FR-060-approval-binds-to-the-merged-head-commit.md), [NFR-007](../non-functional/NFR-007-no-release-without-a-person.md).
- **Downstream**: [FR-068](./FR-068-verification-summary-signed-by-the-pinned-workflow.md).
