---
id: FR-063
title: "Fail visibly with a named error when a required role has no staffed approver"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-061"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-062"
    type: "references"
---

# FR-063: Fail visibly with a named error when a required role has no staffed approver

## Description

Tracked by Linear PLAT-1685. A policy that requires a role no one staffs cannot be satisfied, and it must say so. A
role is unstaffed when the policy's `roles` entry for it lists no individual, or when every account of every
individual it lists is classified as not human ([FR-062](./FR-062-ai-agents-are-never-approvers.md)). The evaluation
reports that failure with the error `approval_role_unstaffed` and the role name, whether or not any approval exists, so
a misconfigured policy is found before the first pull request waits on it.

Classifying a listed account that has never reviewed needs a lookup of its account type from the source. A lookup
that fails is not a classification: the evaluation returns `status` `unavailable` with the retryable error
`approval_source_unavailable`, names the accounts whose lookup failed, and does not report `approval_role_unstaffed`
for a role whose staffing depends on them. A role that is staffed but has no approval yet is reported
`no_approval` ([FR-061](./FR-061-role-policy-and-distinct-individuals.md)).

## Behavior

- The role evaluation SHALL return `unsatisfied` with the error `approval_role_unstaffed` for each unstaffed required
  role.
- The role evaluation SHALL name the unstaffed role in the error.
- The role evaluation SHALL report an unstaffed role when no approval exists.
- The role evaluation SHALL return `unavailable` with `approval_source_unavailable` when an account-type lookup fails.
- The role evaluation SHALL name each account whose lookup failed.
- The role evaluation SHALL NOT pass when a required role is unstaffed.
- The role evaluation SHALL NOT drop an unstaffed role from the required set.
- The role evaluation SHALL NOT report `approval_role_unstaffed` for a role whose staffing depends on a failed lookup.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-063-AC-1 | A policy requiring `stakeholder` whose `roles` entry is an empty list yields `unsatisfied` with `approval_role_unstaffed` naming `stakeholder`, even when approvals for the other roles are valid. | Test |
| FR-063-AC-2 | A role whose only listed accounts are agents or bot-type accounts yields `approval_role_unstaffed` naming it. | Test |
| FR-063-AC-3 | With two unstaffed roles, the evaluation names both. | Test |
| FR-063-AC-4 | A staffed role with no approval yet yields `no_approval`, not `approval_role_unstaffed`. | Test |
| FR-063-AC-5 | The evaluation run with no approvals at all reports the unstaffed role. | Test |
| FR-063-AC-6 | A role whose listed accounts' type lookup fails yields `unavailable` with `approval_source_unavailable` naming the accounts, and not `approval_role_unstaffed`; the same role after the lookup succeeds is evaluated normally. | Test |

## Dependencies

- **Upstream**: [FR-061](./FR-061-role-policy-and-distinct-individuals.md),
  [FR-062](./FR-062-ai-agents-are-never-approvers.md).
