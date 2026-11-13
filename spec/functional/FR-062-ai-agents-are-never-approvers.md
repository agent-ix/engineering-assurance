---
id: FR-062
title: "Never accept an AI agent identity as an approver for any role"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-059"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-061"
    type: "references"
---

# FR-062: Never accept an AI agent identity as an approver for any role

## Description

Tracked by Linear PLAT-1685. An approval is a person's act. The source adapter classifies each approver identity as
human or not; an identity that is not human cannot satisfy any role. An identity is not human when the source reports
its account as a bot or app, or when the policy's `agents` list ([FR-061](./FR-061-role-policy-and-distinct-individuals.md))
names it. An identity the adapter cannot classify is treated as not human.

The classification rests on what the source reports and on the policy; Engineering Assurance cannot detect a person
operating an agent's credentials, and it makes no claim to.

## Behavior

- The source adapter SHALL classify each approver identity as human or not human.
- The source adapter SHALL classify an identity the source reports as a bot or app account as not human.
- The source adapter SHALL classify an identity listed in the policy's `agents` list as not human.
- The source adapter SHALL classify an identity it cannot classify as not human.
- The approval gate SHALL NOT count a statement whose approver is not human toward any role.
- The statement writer SHALL NOT write a statement for an approval whose approver is not human.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-062-AC-1 | An `APPROVED` fixture review by an account of type bot yields no approval and no statement. | Test |
| FR-062-AC-2 | An `APPROVED` review by a user-type account listed in the policy's `agents` yields no approval. | Test |
| FR-062-AC-3 | A review by an account whose type the fixture source does not report yields no approval. | Test |
| FR-062-AC-4 | A statement signed by a valid workflow identity whose `approver` is a listed agent is not counted by the gate for any role, and the gate reports `agent_only` for a role that only that statement would cover. | Test |
| FR-062-AC-5 | Over generated statement sets that include agent identities, the number of roles covered by agent identities is 0. | Property |

## Dependencies

- **Upstream**: [FR-059](./FR-059-source-adapter-interface-and-github-review-adapter.md),
  [FR-061](./FR-061-role-policy-and-distinct-individuals.md).
