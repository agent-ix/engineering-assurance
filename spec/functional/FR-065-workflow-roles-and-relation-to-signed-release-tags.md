---
id: FR-065
title: "Name the CI workflow roles and keep release decisions as person-signed tags"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-041"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-047"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-054"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-058"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-007"
    type: "references"
---

# FR-065: Name the CI workflow roles and keep release decisions as person-signed tags

## Description

Tracked by Linear EA-105 and PLAT-1685. This requirement fixes the roles that CI workflows play and how change
approvals relate to release decisions. It writes no workflow; the code changes that follow add the workflow files
under their own review.

Three workflow roles exist:

- The statement writer runs when a pull request is merged. It reads the approvals from the source adapters
  ([FR-059](./FR-059-source-adapter-interface-and-github-review-adapter.md)), writes and signs one approval statement
  per approval ([FR-058](./FR-058-approval-statement-and-predicate.md)) with its workflow identity, and stores them where
  the verifier of [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md) can read them.
- The approval gate runs on the pull request as a required check. It verifies the statements and evaluates the role
  policy ([FR-061](./FR-061-role-policy-and-distinct-individuals.md)).
- The record publisher runs when a record is to be published. It calls the sink interface
  ([FR-054](./FR-054-record-sink-interface-and-implementations.md)) with the repository's resolved sink.

Each role is a separate workflow identity that the workflow-identity policy lists independently, so trusting the
statement writer does not trust the publisher.

Two approval mechanisms coexist. A change approval is a platform-recorded statement: the platform records the approval
and CI signs the statement. A release decision stays a person-signed annotated tag, verified under
[FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md), [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md),
[FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md),
[FR-042](./FR-042-create-signed-approval-tag-for-a-change-assurance-record.md) and
[FR-047](./FR-047-release-record-and-signed-release-tag.md). The approval statement does not supersede that path, and
[NFR-007](../non-functional/NFR-007-no-release-without-a-person.md) is unchanged.

## Behavior

- The release verifier SHALL NOT accept an approval statement as a release decision.
- The release verifier SHALL NOT change its result because an approval statement is present or absent.
- The approval gate SHALL NOT sign any statement.
- The record publisher SHALL NOT sign any approval statement.
- The statement writer SHALL NOT publish a record to a sink.
- The workflow-identity policy SHALL list the identity of each workflow role separately.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-065-AC-1 | A release commit with a valid approval statement but an unsigned release tag yields `not_released`, and a signed valid release yields the same `released` result with the statement removed. | Test |
| FR-065-AC-2 | A release-verify request that offers an approval statement as a member is refused as an invalid request. | Test |
| FR-065-AC-3 | A workflow-identity policy that lists only the statement writer's identity does not verify a record signed by the publisher's identity, and the reverse holds. | Test |
| FR-065-AC-4 | Static inspection shows no signing call in the gate code or the publisher code, and no sink call in the statement writer code. | Inspection |
| FR-065-AC-5 | Static inspection shows no code path in the release module that reads an approval statement. | Inspection |

## Dependencies

- **Upstream**: [FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md),
  [FR-047](./FR-047-release-record-and-signed-release-tag.md),
  [FR-054](./FR-054-record-sink-interface-and-implementations.md),
  [FR-058](./FR-058-approval-statement-and-predicate.md),
  [NFR-007](../non-functional/NFR-007-no-release-without-a-person.md).
- **Downstream**: the workflow code changes of EA-97, EA-98, EA-105 and PLAT-1685.
