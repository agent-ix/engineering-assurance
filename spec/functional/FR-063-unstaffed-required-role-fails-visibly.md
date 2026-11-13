---
id: FR-063
title: "Fail visibly with a named error when a required role has no staffed approver"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-061"
    type: "references"
---

# FR-063: Fail visibly with a named error when a required role has no staffed approver

## Description

Tracked by Linear PLAT-1685. A policy that requires a role no one staffs cannot be satisfied, and it must say so. A
role is unstaffed when the policy's `roles` entry for it lists no human account, or lists only identities classified
as not human ([FR-062](./FR-062-ai-agents-are-never-approvers.md)). The gate reports that failure with the error
`approval_role_unstaffed` and the role name. The same failure is reported for an unstaffed role whether or not any
statements exist, so a misconfigured policy is found before the first pull request waits on it.

Distinct from a role that is staffed but not yet signed: that role is reported `no_statement`
([FR-061](./FR-061-role-policy-and-distinct-individuals.md)).

## Behavior

- The approval gate SHALL return `unsatisfied` with the error `approval_role_unstaffed` for each unstaffed required
  role.
- The approval gate SHALL name the unstaffed role in the error.
- The approval gate SHALL report an unstaffed role when no statement exists.
- The approval gate SHALL NOT pass when a required role is unstaffed.
- The approval gate SHALL NOT drop an unstaffed role from the required set.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-063-AC-1 | A policy requiring `stakeholder` whose `roles` entry is an empty list yields `unsatisfied` with `approval_role_unstaffed` naming `stakeholder`, even when statements for the other roles are valid. | Test |
| FR-063-AC-2 | A role whose only listed accounts are agents yields `approval_role_unstaffed` naming it. | Test |
| FR-063-AC-3 | With two unstaffed roles, the gate names both. | Test |
| FR-063-AC-4 | A staffed role with no statement yet yields `no_statement`, not `approval_role_unstaffed`. | Test |
| FR-063-AC-5 | The gate evaluated with no statements at all reports the unstaffed role. | Test |

## Dependencies

- **Upstream**: [FR-061](./FR-061-role-policy-and-distinct-individuals.md),
  [FR-062](./FR-062-ai-agents-are-never-approvers.md).
