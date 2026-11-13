---
id: FR-058
title: "Record a change approval as a CI-signed in-toto statement bound to the pull request and the merged commit"
type: FR
relationships:
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
head commit is a predicate member.

The statement has the in-toto statement type, one subject (the repository and the merged commit as its git commit
digest), the predicate type `https://schemas.agent-ix.ai/attestation/approval/v1` and the predicate below. The
predicate type is an identity string; no document needs to be hosted at that address. The statement and predicate are
closed schemas ([NFR-008](../non-functional/NFR-008-closed-schemas.md)). The signature is made through the signing
stack named in [NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md).

The predicate members are:

- `source`: the approval source, a short identifier (`github` now; a tracker such as Jira later).
- `approver`: the approver identity exactly as the source reports it.
- `role`: the role the source adapter mapped the approval to ([FR-059](./FR-059-source-adapter-interface-and-github-review-adapter.md)).
- `meaning`: the text of the required `Signed-off-meaning:` line.
- `signed_at`: the approval timestamp as the source reports it, a UTC instant `YYYY-MM-DDTHH:MM:SSZ`.
- `head_commit`: the pull request head commit the approver reviewed.
- `pull_request`: the pull request reference as the source reports it (repository and number).
- `merged_commit`: the commit the pull request produced on the base branch.

Out of scope: the role policy ([FR-061](./FR-061-role-policy-and-distinct-individuals.md)), verification
([FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md)), the change-record that references
approval statements (EA-98), and the signed-tag path for release decisions
([FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md)).

## Behavior

- The statement writer SHALL produce one statement per approval.
- The statement writer SHALL set the statement subject to the repository and the merged commit.
- The statement writer SHALL set the predicate type to `https://schemas.agent-ix.ai/attestation/approval/v1`.
- The statement writer SHALL copy `approver` from the source's report without changing it.
- The statement writer SHALL take `meaning` from the `Signed-off-meaning:` line of the review body.
- The statement writer SHALL take `signed_at` from the source's timestamp.
- The statement writer SHALL NOT use the time of signing or the current time for `signed_at`.
- The statement writer SHALL NOT write a predicate member outside the closed set.
- The statement writer SHALL NOT write a statement for a review that has no `Signed-off-meaning:` line.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-058-AC-1 | A statement written for a fixture approval has exactly the predicate members `source`, `approver`, `role`, `meaning`, `signed_at`, `head_commit`, `pull_request` and `merged_commit`, the predicate type above, and a subject naming the repository and the merged commit. | Test |
| FR-058-AC-2 | A statement or predicate carrying an unknown member fails schema validation. | Test |
| FR-058-AC-3 | The `approver` in the statement equals the identity string the fixture source reported, byte for byte. | Test |
| FR-058-AC-4 | `signed_at` equals the source timestamp normalized to UTC, and differs from the time at which the statement was written. | Test |
| FR-058-AC-5 | The statement for a squash-merged pull request names the squash commit as the subject and the reviewed head commit as `head_commit`, and the two differ. | Test |
| FR-058-AC-6 | A review with no `Signed-off-meaning:` line yields no statement. | Test |
| FR-058-AC-7 | The statement is signed with the CI workflow identity and verifies under FR-064; the same statement with one predicate byte changed does not verify. | Test |

## Dependencies

- **Upstream**: [FR-059](./FR-059-source-adapter-interface-and-github-review-adapter.md),
  [NFR-008](../non-functional/NFR-008-closed-schemas.md),
  [NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md).
- **Downstream**: [FR-061](./FR-061-role-policy-and-distinct-individuals.md),
  [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md); the change-record of EA-98 and the
  release approvals of EA-97 reference approval statements.
