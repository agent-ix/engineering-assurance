---
id: FR-061
title: "Require every role in the repository's approval policy to be signed by a distinct individual"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-055"
    type: "references"
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
stakeholder, so approval is not one flag. A per-repository role policy lists the required roles. The role evaluation
passes only when every listed role is covered by an approval from a distinct individual: one person cannot fill two
roles. A lighter repository lists one role. No approver, role name or number of roles is hardcoded.

The policy is one JSON document, `engineering-assurance.approval-policy/v1`, at
`.engineering-assurance/approval-policy.json`, read from the protected base branch
([FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md)). It is a closed schema
([NFR-008](../non-functional/NFR-008-closed-schemas.md)) with these members and no other:

- `protocol`: the literal `engineering-assurance.approval-policy/v1`.
- `required_roles`: a non-empty list of unique role names.
- `individuals`: a map from an individual name to a non-empty list of accounts `{source, id}`, where `id` is the
  source's stable account identifier. Individual names and role names use the grammar `a-z 0-9 _ -`, 1 to 32
  characters, starting with a letter or digit.
- `roles`: a map from each role name to the list of individual names that may fill it.
- `agents`: a list of accounts `{source, id}` that are agents ([FR-062](./FR-062-ai-agents-are-never-approvers.md));
  the default is the empty list.

An account belongs to at most one individual. Distinctness is by individual, not by account: two accounts of one
individual are one person. An account that no individual lists never fills a role. An individual may be listed for
several roles but fills at most one role for a pull request; the role that an approval fills is the one the review
names in its `Role:` line ([FR-059](./FR-059-source-adapter-interface-and-github-review-adapter.md)), and that role
must be one the individual is listed for. Distinctness rests on the accounts the policy lists; an individual with an
account the policy does not list is not detectable.

One evaluation function, a pure function of the policy and a set of approvals, decides the role evaluation. Its
input is a list of approvals, each `{source, account id, role}`. The function is used twice, on different inputs, and
nowhere else: before the merge on the platform reviews, by the review gate, and after the merge on verified statements,
by the verifier ([FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md),
[FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md)).

Role coverage is exact. The role set is satisfied if and only if a maximum bipartite matching between the required
roles and distinct individuals, where an individual is connected to a role when a counting approval of that
individual names the role, covers every required role. No greedy or order-dependent choice decides the verdict: an
assignment may be re-arranged freely. The verdict reports one deterministic assignment, the lexicographically smallest
by role name and then individual name, and when unsatisfied it names the required roles that lie outside a maximum
matching, as the roles that are not in the lexicographically smallest maximum matching, in ascending order of role name.

## Outputs

- The verdict, `engineering-assurance.approval-gate/v1`, with `status` `satisfied`, `unsatisfied` or `unavailable`
  ([FR-063](./FR-063-unstaffed-required-role-fails-visibly.md)), the `required_roles`, and per covered role the
  individual and the approval that covered it. On `unsatisfied` it adds `missing`, the uncovered roles in ascending
  order, each with the reason `no_approval` (no approval names the role), `shared_individual` (counting approvals exist
  but their individuals fill other roles) or `agent_only` (only non-human approvals name the role), and the named error
  of FR-063 where it applies.

## Behavior

- The role evaluation SHALL read the policy from the protected base branch.
- The role evaluation SHALL return `satisfied` only when each required role is covered.
- The role evaluation SHALL count an approval only when its account belongs to an individual listed for the approval's
  role.
- The role evaluation SHALL assign each covered role a different individual.
- The role evaluation SHALL return `satisfied` if and only if a matching of distinct individuals to roles covers every required role.
- The role evaluation SHALL report the lexicographically smallest assignment by role name and then individual name.
- The role evaluation SHALL name the roles outside a maximum matching in ascending order of role name.
- The role evaluation SHALL give the same verdict for the same policy and approvals whatever their order.
- The role evaluation SHALL NOT count an account that no individual lists.
- The role evaluation SHALL NOT count one individual for two roles.
- The role evaluation SHALL NOT pass with fewer roles than the policy lists.
- The role evaluation SHALL NOT name a role, approver or role count in code.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-061-AC-1 | A policy of roles `developer`, `engineer`, `stakeholder` with approvals from three different individuals, each for a role they are listed for, yields `satisfied` with one individual per role. | Test |
| FR-061-AC-2 | The same policy with one individual holding approvals for `developer` and `engineer` and nobody else covering either yields `unsatisfied`, with `missing` equal to `[engineer]` with reason `shared_individual`, and the same verdict for either order of the approvals. | Test |
| FR-061-AC-3 | A one-role policy with one approval yields `satisfied`; a policy with role names other than the three above is evaluated the same way. | Test |
| FR-061-AC-4 | A policy change on the pull request branch does not change the policy used; the base branch's document is used. | Test |
| FR-061-AC-5 | A policy with an unknown member, an empty or duplicated `required_roles`, a required role absent from `roles`, an individual named in `roles` but not declared, an account under two individuals, or an account both in an individual and in `agents`, is refused with `approval_policy_invalid`, and the evaluation is `unsatisfied`. | Test |
| FR-061-AC-6 | Over generated policies and generated approval sets, `satisfied` is returned if and only if an assignment of distinct individuals to all required roles exists (checked against an exhaustive search), and the verdict and the reported assignment do not change when the approvals are reordered. | Property |
| FR-061-AC-7 | An individual listed for `developer` and `engineer` who approves as `developer`, and a second individual listed for `engineer` who approves as `engineer`, yield `satisfied`; an approval naming a role the individual is not listed for is not counted. | Test |
| FR-061-AC-8 | Two accounts listed under one individual that approve for two roles count as one individual, and an account that no individual lists approving for a role counts for nothing. | Test |
| FR-061-AC-10 | Roles `A` and `B`, individual X approving `A` and `B` through two accounts and individual Y approving `A`, yield `satisfied` with the assignment `A` to Y and `B` to X, whatever the order of the approvals. | Test |
| FR-061-AC-11 | Roles `A` and `B` with individual X alone approving both yield `unsatisfied` with `missing` equal to `[B]`; roles `A`, `B` and `C` with X approving `A`, `B` and `C` and Y approving only `A` yield `unsatisfied` with `missing` equal to `[C]`. | Test |
| FR-061-AC-9 | The review gate and the statement verifier return the same verdict for the same policy when given the same approvals as reviews and as verified statements. | Test |

## Dependencies

- **Upstream**: [FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md),
  [FR-058](./FR-058-approval-statement-and-predicate.md),
  [FR-062](./FR-062-ai-agents-are-never-approvers.md),
  [NFR-008](../non-functional/NFR-008-closed-schemas.md).
- **Downstream**: [FR-063](./FR-063-unstaffed-required-role-fails-visibly.md),
  [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md),
  [FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md); the required-check, change-record and
  receipt requirements of EA-98 and EA-97 that evaluate this policy.
