---
id: FR-061
title: "Require every role in the repository's approval policy to be signed by a distinct individual"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-058"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-062"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-063"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-008"
    type: "references"
---

# FR-061: Require every role in the repository's approval policy to be signed by a distinct individual

## Description

Tracked by Linear PLAT-1685. Regulated practice uses separate sign-offs, for example developer, engineer and
stakeholder, so approval is not one flag. A per-repository role policy lists the required roles. The approval gate
passes only when every listed role is covered by a verified approval statement from a distinct individual: one person
cannot fill two roles. A lighter repository lists one role. No approver, role name or number of roles is hardcoded.

The policy is one JSON document, `engineering-assurance.approval-policy/v1`, at
`.engineering-assurance/approval-policy.json`, read from the repository's protected base branch and never from the
pull request under change. It is a closed schema ([NFR-008](../non-functional/NFR-008-closed-schemas.md)). Its
members are `protocol`, `required_roles` (a non-empty list of unique role names), `roles` (for each role name, the
`source` and the list of account identifiers the source adapter maps to the role), and `agents` (a list of identities
that are agents, read by [FR-062](./FR-062-ai-agents-are-never-approvers.md); default empty).

Distinct individual is judged on the approver identity as the source reports it. The same source identity in two
statements is one individual; identities from different sources are distinct unless the policy's `roles` lists both
under a shared `individual` key, which is the only way the policy equates identities. The gate does not fall back to
fewer roles, and does not count one statement for two roles.

Out of scope: how statements are verified ([FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md))
and the unstaffed-role failure ([FR-063](./FR-063-unstaffed-required-role-fails-visibly.md)).

## Outputs

- The gate verdict, `engineering-assurance.approval-gate/v1`, with `status` `satisfied` or `unsatisfied`, the
  `required_roles`, and per role the approver and statement that covered it. On `unsatisfied` it adds `missing`, the
  roles with no distinct covering individual, each with the reason `no_statement`, `shared_individual` or
  `agent_only`, and the named error from [FR-063](./FR-063-unstaffed-required-role-fails-visibly.md) where it applies.

## Behavior

- The approval gate SHALL read the policy from the protected base branch.
- The approval gate SHALL return `satisfied` only when each required role is covered by a verified statement.
- The approval gate SHALL assign each required role a different individual.
- The approval gate SHALL return `unsatisfied` when no assignment of distinct individuals to all required roles exists.
- The approval gate SHALL name each role it could not cover.
- The approval gate SHALL NOT count one individual for two roles.
- The approval gate SHALL NOT pass with fewer roles than the policy lists.
- The approval gate SHALL NOT name a role, approver or role count in code.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-061-AC-1 | A policy of roles `developer`, `engineer`, `stakeholder` with verified statements from three different individuals yields `satisfied` with one approver per role. | Test |
| FR-061-AC-2 | The same policy with one individual holding statements for `developer` and `engineer` yields `unsatisfied`, with `missing` naming the role left uncovered with reason `shared_individual`. | Test |
| FR-061-AC-3 | A one-role policy with one verified statement yields `satisfied`; a policy with role names other than the three above is evaluated the same way. | Test |
| FR-061-AC-4 | A policy change on the pull request branch does not change the policy used; the base branch's document is used. | Test |
| FR-061-AC-5 | A policy with an unknown member, an empty or duplicated `required_roles`, or a required role absent from `roles` is refused with `approval_policy_invalid`, and the gate is `unsatisfied`. | Test |
| FR-061-AC-6 | Over generated policies and generated statement sets, `satisfied` is returned only when an assignment of distinct individuals to all roles exists. | Property |
| FR-061-AC-7 | Two statements from the same source identity for two roles, and one more statement from a second identity for the second role, yield `satisfied` with the second identity covering that role. | Test |
| FR-061-AC-8 | Two identities from different sources are two individuals, and become one individual when the policy equates them under a shared `individual` key. | Test |

## Dependencies

- **Upstream**: [FR-058](./FR-058-approval-statement-and-predicate.md),
  [FR-062](./FR-062-ai-agents-are-never-approvers.md),
  [NFR-008](../non-functional/NFR-008-closed-schemas.md).
- **Downstream**: [FR-063](./FR-063-unstaffed-required-role-fails-visibly.md),
  [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md); the required-check, change-record and
  receipt requirements of EA-98 and EA-97 that evaluate this policy.
