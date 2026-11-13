---
id: FR-059
title: "Read approvals through a source adapter interface, with a GitHub review adapter first"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-058"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-060"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-062"
    type: "references"
---

# FR-059: Read approvals through a source adapter interface, with a GitHub review adapter first

## Description

Tracked by Linear PLAT-1685. A source adapter reads approvals from one approval source and returns them in a
source-neutral form. The interface has three operations: list the approvals recorded for a pull request, classify an
approver identity as human or not ([FR-062](./FR-062-ai-agents-are-never-approvers.md)), and map an approval to a role
using the adapter's role mapping. The approval statement of [FR-058](./FR-058-approval-statement-and-predicate.md)
does not depend on which adapter produced the approval, so a second adapter (Jira or another tracker) is added
without a change to the predicate.

An adapter approval carries the approver identity as the source reports it, the source's timestamp, the commit the
approval was given on, the pull request reference, the text of the approval, and the review state.

The first adapter reads GitHub pull request reviews. Its approval is a review in the `APPROVED` state that has not
been dismissed. Its approver is the review author's account login with the account's stable numeric id. It takes the
meaning from the required `Signed-off-meaning:` line of the review body. A review with no such line, or with an empty
meaning, produces no approval.

The role mapping is part of the adapter's configuration in the role policy document
([FR-061](./FR-061-role-policy-and-distinct-individuals.md)). The GitHub adapter maps a role from the accounts the
policy lists for that role. A review by an account the policy lists for no role produces no approval.

Out of scope: any adapter other than GitHub, the writing of the statement (FR-058), and the commit binding checks
([FR-060](./FR-060-approval-binds-to-the-merged-head-commit.md)).

## Behavior

- The source adapter interface SHALL return approvals in a source-neutral form that has no GitHub-specific member.
- The GitHub adapter SHALL return an approval only for a review in the `APPROVED` state that is not dismissed.
- The GitHub adapter SHALL read the meaning from a line that starts with `Signed-off-meaning:` in the review body.
- The GitHub adapter SHALL return no approval for a review with no such line.
- The GitHub adapter SHALL return no approval for a line whose meaning is empty after trimming.
- The GitHub adapter SHALL report the approver as the account login together with the account's numeric id.
- The GitHub adapter SHALL map the role from the policy's account list for each role.
- The GitHub adapter SHALL return no approval for an account that the policy lists for no role.
- The adapter interface SHALL NOT require the statement predicate to change when an adapter is added.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-059-AC-1 | A fixture `APPROVED` review whose body has a `Signed-off-meaning:` line, by an account the policy lists for a role, yields one approval with source, approver, role, meaning, timestamp, commit and pull request reference. | Test |
| FR-059-AC-2 | Fixture reviews in the `COMMENTED`, `CHANGES_REQUESTED` and `DISMISSED` states, each with a valid meaning line, yield no approval. | Test |
| FR-059-AC-3 | An `APPROVED` review with no meaning line, with an empty meaning, or with the text `Signed-off-meaning` mid-line, yields no approval. | Test |
| FR-059-AC-4 | An `APPROVED` review by an account the policy lists for no role yields no approval. | Test |
| FR-059-AC-5 | A test adapter implementing the interface with a different source identifier produces approvals that FR-058 writes into statements with the unchanged predicate type and members. | Test |
| FR-059-AC-6 | An approval of the source-neutral form contains no member whose name is specific to GitHub. | Inspection |

## Dependencies

- **Upstream**: [FR-058](./FR-058-approval-statement-and-predicate.md),
  [FR-061](./FR-061-role-policy-and-distinct-individuals.md),
  [FR-062](./FR-062-ai-agents-are-never-approvers.md).
- **Downstream**: [FR-060](./FR-060-approval-binds-to-the-merged-head-commit.md).
