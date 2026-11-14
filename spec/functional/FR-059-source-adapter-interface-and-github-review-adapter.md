---
id: FR-059
title: "Read approvals through a source adapter interface, with a GitHub review adapter first"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-058"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-060"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-061"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-062"
    type: "references"
---

# FR-059: Read approvals through a source adapter interface, with a GitHub review adapter first

## Description

Tracked by Linear PLAT-1685. A source adapter reads approvals from one approval source and returns them in a
source-neutral form. The interface has three operations: list the approvals recorded for a pull request, classify an
approver account as human or not ([FR-062](./FR-062-ai-agents-are-never-approvers.md)), and map an account to the
individual the role policy lists for it ([FR-061](./FR-061-role-policy-and-distinct-individuals.md)). The approval
statement of [FR-058](./FR-058-approval-statement-and-predicate.md) does not depend on which adapter produced the
approval, so a second adapter (Jira or another tracker) is added without a change to the predicate.

An adapter approval carries the source, the account's stable identifier and display name, the source's timestamp, the
commit the approval was given on, the pull request reference, the role, the meaning and the body of an unedited review.

The first adapter reads GitHub pull request reviews. Per reviewer, only that reviewer's latest review in the state
`APPROVED`, `CHANGES_REQUESTED` or `DISMISSED` decides: a review in `COMMENTED` changes nothing, a later
`CHANGES_REQUESTED` or a dismissal removes an earlier approval, and the reviewer's approval counts only if that latest
review is `APPROVED`. The approver is the review author's account, identified by its stable numeric id; the login is
display only. Matching to the policy is by source and stable id only, so a renamed or re-registered login grants
nothing.

The GitHub adapter accepts marker lines only from an unedited submitted review. If the source reports a last-edit
timestamp or any edit-history entry, the review yields no approval regardless of whether the edit added or removed a
marker line; the reviewer must submit a new review. If the adapter cannot establish that the review is unedited, it
fails closed and returns no approval. It does not reconstruct or persist prior review bodies. Two marker lines are
required, each at the start of a line, in a body whose line ends are LF or CRLF:

- `Signed-off-meaning:` followed by one optional space and a value of 1 to 256 bytes of UTF-8 with no control
  character, with leading and trailing ASCII spaces removed.
- `Role:` followed by one optional space and a role name of 1 to 32 characters from `a-z 0-9 _ -`, starting with a
  letter or digit.

Exactly one `Signed-off-meaning:` line and exactly one `Role:` line must be present; a body with none or more than one
of either produces no approval. The key text is matched case-sensitively. A review whose `Role:` names a role the
policy does not list for the reviewer's individual produces no approval.

Labels are out of scope as role carriers in this adapter: a label can be applied by anyone with triage access and is
not a person's act, so the adapter maps roles from accounts and the `Role:` line only. A reviewer whose account the
policy lists under no individual produces no approval.

Out of scope: any adapter other than GitHub, the writing of the statement (FR-058), and the commit binding checks
([FR-060](./FR-060-approval-binds-to-the-merged-head-commit.md)).

## Behavior

- The source adapter interface SHALL return approvals in a source-neutral form that has no GitHub-specific member.
- The GitHub adapter SHALL decide each reviewer's approval from that reviewer's latest review in the state
  `APPROVED`, `CHANGES_REQUESTED` or `DISMISSED`.
- The GitHub adapter SHALL return an approval only when that latest review is `APPROVED`.
- The GitHub adapter SHALL return no approval when the review has a last-edit timestamp or a non-empty edit history.
- The GitHub adapter SHALL fail closed and return no approval when it cannot establish whether the review body was edited.
- The reviewer SHALL submit a new review after editing a submitted review body for that approval to count.
- The GitHub adapter SHALL read the meaning and the role from lines that start with `Signed-off-meaning:` and `Role:`
  in the review body.
- The GitHub adapter SHALL return no approval for a body with zero or more than one line of either key.
- The GitHub adapter SHALL return no approval for a meaning that is empty, longer than 256 bytes, not UTF-8 or
  contains a control character.
- The GitHub adapter SHALL return no approval for a role name outside the role grammar.
- The GitHub adapter SHALL identify the approver by source and numeric account id.
- The GitHub adapter SHALL return no approval for an account that the policy lists under no individual.
- The GitHub adapter SHALL return no approval for a role that the reviewer's individual is not listed for.
- The adapter interface SHALL NOT require the statement predicate to change when an adapter is added.
- The GitHub adapter SHALL NOT derive a role from a label.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-059-AC-1 | A fixture `APPROVED` review with one meaning line and one `Role:` line the reviewer's individual is listed for yields one approval with source, approver, role, meaning, timestamp, commit and pull request reference. | Test |
| FR-059-AC-2 | Fixture reviews in the `COMMENTED`, `CHANGES_REQUESTED` and `DISMISSED` states, each with valid lines, yield no approval. | Test |
| FR-059-AC-3 | An `APPROVED` review with no meaning line, an empty meaning, a meaning over 256 bytes, a control character in the meaning, a mid-line `Signed-off-meaning`, no `Role:` line, or a role name outside the grammar, yields no approval; a CRLF-ended body with valid lines yields an approval. | Test |
| FR-059-AC-4 | An `APPROVED` review with two meaning lines, or two `Role:` lines, even when equal, yields no approval. | Test |
| FR-059-AC-5 | An `APPROVED` review followed by a `CHANGES_REQUESTED` review by the same reviewer on the same head yields no approval; a later dismissal yields none; a later `COMMENTED` review leaves the approval; a later `APPROVED` after `CHANGES_REQUESTED` restores it. | Test |
| FR-059-AC-6 | An `APPROVED` review with a last-edit timestamp or non-empty edit history yields no approval whether the edit adds or removes a valid marker line; missing edit metadata yields no approval; a newly submitted, unedited `APPROVED` review with valid lines by that eligible reviewer yields one approval. | Test |
| FR-059-AC-7 | An `APPROVED` review by an account listed under no individual, or naming a role the reviewer's individual is not listed for, yields no approval. | Test |
| FR-059-AC-8 | A policy that lists a login-like string in place of the numeric id matches no account; an account renamed to a listed login that has a different numeric id yields no approval. | Test |
| FR-059-AC-9 | A test adapter implementing the interface with a different source identifier produces approvals that FR-058 writes into statements with the unchanged predicate type and members. | Test |
| FR-059-AC-10 | An approval of the source-neutral form contains no member whose name is specific to GitHub, and a fixture pull request with a label naming a role yields no approval and no role. | Inspection |

## Dependencies

- **Upstream**: [FR-058](./FR-058-approval-statement-and-predicate.md),
  [FR-061](./FR-061-role-policy-and-distinct-individuals.md),
  [FR-062](./FR-062-ai-agents-are-never-approvers.md).
- **Downstream**: [FR-060](./FR-060-approval-binds-to-the-merged-head-commit.md).
