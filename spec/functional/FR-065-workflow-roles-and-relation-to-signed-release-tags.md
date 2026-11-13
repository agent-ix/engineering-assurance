---
id: FR-065
title: "Name the CI workflow roles, gate on reviews before merge, and keep release decisions as person-signed tags"
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
  - target: "ix://agent-ix/engineering-assurance/FR-061"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-007"
    type: "references"
---

# FR-065: Name the CI workflow roles, gate on reviews before merge, and keep release decisions as person-signed tags

## Description

Tracked by Linear EA-105 and PLAT-1685. This requirement fixes the roles that CI workflows play and how change
approvals relate to release decisions. It writes no workflow; the code changes that follow add the workflow files
under their own review.

Three workflow roles exist:

- The review gate runs on the pull request as a required check, before the merge. A statement does not yet exist
  then, because the merged commit does not exist. The gate reads the platform reviews through the source adapter
  ([FR-059](./FR-059-source-adapter-interface-and-github-review-adapter.md)) and evaluates them against the role
  policy with the role evaluation function of [FR-061](./FR-061-role-policy-and-distinct-individuals.md). It signs and
  publishes nothing.
- The statement writer runs when a pull request is merged. It selects the counting approvals with the same
  evaluation function, writes and signs one approval statement per counted approval
  ([FR-058](./FR-058-approval-statement-and-predicate.md)) with its workflow identity, and hands each statement to the
  sink interface ([FR-054](./FR-054-record-sink-interface-and-implementations.md)) with the repository's resolved sink,
  the split of FR-056 and the leak check of FR-057. The statement is the post-merge record.
- The record publisher runs when a record is to be published. It calls the sink interface for records other than the
  statements the writer hands over.

The relation between the gate and the statements is that the review gate decides whether a change may merge, from the
reviews; the statements record afterwards what the reviews were, and the verifier of
[FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md) checks the statements against the same
policy and the same function, so an incomplete role set that landed anyway is reported `unsatisfied`.

Each role is a separate workflow identity that the workflow-identity policy lists independently, so trusting the
statement writer does not trust the publisher.

Two approval mechanisms coexist and do not mix. A change approval for the role gate is a platform-recorded review and
its statement. A release decision stays a person-signed annotated tag, verified under
[FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md), [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md),
[FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md),
[FR-042](./FR-042-create-signed-approval-tag-for-a-change-assurance-record.md) and
[FR-047](./FR-047-release-record-and-signed-release-tag.md). A signed approval tag of a change-assurance record
(FR-041, FR-042, [FR-043](./FR-043-approval-trail-of-a-change-assurance-record.md)) does not count toward the role
evaluation: only platform reviews and their statements count. The approval statement does not supersede the tag path,
and [NFR-007](../non-functional/NFR-007-no-release-without-a-person.md) is unchanged.

## Behavior

- The review gate SHALL evaluate the platform reviews against the role policy before the merge.
- The review gate SHALL use the role evaluation function of FR-061.
- The statement writer SHALL use the role evaluation function of FR-061 to select the counted approvals.
- The statement verifier SHALL check the written statements against the same policy and function.
- The role evaluation SHALL NOT count a signed approval tag as an approval.
- The release verifier SHALL NOT accept an approval statement as a release decision.
- The release verifier SHALL NOT change its result because an approval statement is present or absent.
- The review gate SHALL NOT sign any statement.
- The record publisher SHALL NOT sign any approval statement.
- The statement writer SHALL NOT publish a record other than approval statements.
- The workflow-identity policy SHALL list the identity of each workflow role separately.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-065-AC-1 | A release commit with a valid approval statement but an unsigned release tag yields `not_released`, and a signed valid release yields the same `released` result with the statement removed. | Test |
| FR-065-AC-2 | A release-verify request that offers an approval statement as a member is refused as an invalid request. | Test |
| FR-065-AC-3 | A workflow-identity policy that lists only the statement writer's identity does not verify a record signed by the publisher's identity, and the reverse holds. | Test |
| FR-065-AC-4 | Static inspection shows no signing call in the review gate code or the publisher code, and no sink call other than for approval statements in the statement writer code. | Inspection |
| FR-065-AC-5 | Static inspection shows no code path in the release module that reads an approval statement. | Inspection |
| FR-065-AC-6 | Before the merge, with reviews from three individuals for three required roles and no statement in existence, the review gate returns `satisfied`; with one role missing it returns `unsatisfied` naming the role. | Test |
| FR-065-AC-7 | After the merge, the statements written from those reviews verify, and the verifier's role verdict equals the review gate's verdict for the same policy. | Test |
| FR-065-AC-8 | A change-assurance record carrying a validly signed approval tag by a person listed in the role policy, and no platform review, yields `unsatisfied` for that person's role. | Test |
| FR-065-AC-9 | Approval statements are stored and published through the sink interface: a statement for a private repository goes to the private store with no public-log call, and a statement refused by the leak check is sent nowhere. | Test |

## Dependencies

- **Upstream**: [FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md),
  [FR-047](./FR-047-release-record-and-signed-release-tag.md),
  [FR-054](./FR-054-record-sink-interface-and-implementations.md),
  [FR-058](./FR-058-approval-statement-and-predicate.md),
  [FR-061](./FR-061-role-policy-and-distinct-individuals.md),
  [NFR-007](../non-functional/NFR-007-no-release-without-a-person.md).
- **Downstream**: the workflow code changes of EA-97, EA-98, EA-105 and PLAT-1685.
