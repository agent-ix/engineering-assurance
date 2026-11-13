---
id: FR-058
title: "Record a change approval as a CI-signed in-toto statement bound to the pull request and the merged commit"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-056"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-059"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-064"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-065"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-008"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-010"
    type: "references"
---

# FR-058: Record a change approval as a CI-signed in-toto statement bound to the pull request and the merged commit

## Description

Tracked by Linear PLAT-1685. A change approval is recorded by the platform where it happened. CI reads the approval
from its source, writes an in-toto statement, and signs it with the CI workflow identity. The statement names the pull
request and the merged commit, so that it survives a squash merge: the merged commit is the subject, and the reviewed
head commit is a predicate member. The statement is the post-merge record of an approval; the pre-merge decision is
taken on the platform reviews ([FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md)).

The statement has the in-toto statement type, one subject (the repository and the merged commit as its git commit
digest), the approval predicate type and the predicate below. The predicate type is the URI with scheme `https`, host
`schemas.agent-ix.ai` and path `/attestation/approval/v1`; it is an identity string and no document needs to be hosted
at that address. The code change that implements this requirement writes the literal URI after the content-rights
check admits the organization's own `schemas.agent-ix.ai/attestation/` prefix. The statement and predicate are closed
schemas ([NFR-008](../non-functional/NFR-008-closed-schemas.md)). The signature is made through the signing stack named
in [NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md).

The predicate members are:

- `source`: the approval source, a short identifier (`github` now; a tracker such as Jira later).
- `approver`: the source and the account's stable identifier joined by a colon, for example `github:12345`. For
  GitHub the identifier is the numeric account id. A login is never part of `approver`.
- `approver_name`: the display name or login the source reports, informational only and never used to match or count.
- `role`: the role named by the review's required `Role:` line ([FR-059](./FR-059-source-adapter-interface-and-github-review-adapter.md)).
- `meaning`: the value of the review's required `Signed-off-meaning:` line.
- `signed_at`: the approval timestamp as the source reports it, a UTC instant `YYYY-MM-DDTHH:MM:SSZ`.
- `head_commit`: the pull request head commit the approver reviewed.
- `pull_request`: the pull request reference as the source reports it (repository and number).
- `merged_commit`: the commit the pull request produced on the base branch.

A statement is stored and published as a record: through the sink interface of
[FR-054](./FR-054-record-sink-interface-and-implementations.md), with the repository's resolved sink, the split of
[FR-056](./FR-056-public-and-private-record-split.md) and the leak check of
[FR-057](./FR-057-fail-closed-pre-publish-leak-check.md). The meaning, the approver and the pull request reference are
in the private form only.

Out of scope: the role policy ([FR-061](./FR-061-role-policy-and-distinct-individuals.md)), verification
([FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md)), and the change-record that references
approval statements (EA-98).

## Behavior

- The statement writer SHALL produce one statement per counted approval.
- The statement writer SHALL set the statement subject to the repository and the merged commit.
- The statement writer SHALL set the predicate type to the URI with scheme `https`, host `schemas.agent-ix.ai` and path
  `/attestation/approval/v1`.
- The statement writer SHALL build `approver` from the source and the account's stable identifier.
- The statement writer SHALL take `meaning` and `role` from the review body as submitted.
- The statement writer SHALL take `signed_at` from the source's timestamp.
- The statement writer SHALL hand each statement to the sink interface as a record of kind `approval`.
- The statement writer SHALL NOT use the time of signing or the current time for `signed_at`.
- The statement writer SHALL NOT write a predicate member outside the closed set.
- The statement writer SHALL NOT write a statement for a review that has no valid `Signed-off-meaning:` and `Role:` line.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-058-AC-1 | A statement written for a fixture approval has exactly the predicate members `source`, `approver`, `approver_name`, `role`, `meaning`, `signed_at`, `head_commit`, `pull_request` and `merged_commit`, the approval predicate type, and a subject naming the repository and the merged commit. | Test |
| FR-058-AC-2 | A statement or predicate carrying an unknown member fails schema validation. | Test |
| FR-058-AC-3 | `approver` equals `github:` followed by the fixture account's numeric id; a fixture account renamed between two reviews keeps the same `approver`. | Test |
| FR-058-AC-4 | `signed_at` equals the source timestamp normalized to UTC, and differs from the time at which the statement was written. | Test |
| FR-058-AC-5 | The statement for a squash-merged pull request names the squash commit as the subject and the reviewed head commit as `head_commit`, and the two differ. | Test |
| FR-058-AC-6 | A review with no valid `Signed-off-meaning:` line or no valid `Role:` line yields no statement. | Test |
| FR-058-AC-7 | The statement is signed with the CI workflow identity and verifies under FR-064; the same statement with one predicate byte changed does not verify. | Test |
| FR-058-AC-8 | A statement published for a public repository's public sink yields a public record of kind `approval` with the statement digest and none of `meaning`, `approver`, `approver_name` or `pull_request` in its bytes; the private store holds them under the same `record_id`. | Test |
| FR-058-AC-9 | A statement for a private repository whose resolved sink is the private store makes no public transparency log call. | Test |

## Dependencies

- **Upstream**: [FR-054](./FR-054-record-sink-interface-and-implementations.md),
  [FR-056](./FR-056-public-and-private-record-split.md),
  [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md),
  [FR-059](./FR-059-source-adapter-interface-and-github-review-adapter.md),
  [NFR-008](../non-functional/NFR-008-closed-schemas.md),
  [NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md).
- **Downstream**: [FR-061](./FR-061-role-policy-and-distinct-individuals.md),
  [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md); the change-record of EA-98 and the
  release approvals of EA-97 reference approval statements.
