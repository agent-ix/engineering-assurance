---
id: FR-005
title: "Use resumable workflows with human terminal decisions"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-003"
    type: "implements"
---

# FR-005: Use resumable workflows with human terminal decisions

## Description

When onboarding enters a governed assurance workflow, the onboarding skill SHALL
use ix-flow as the authority for run state, valid transitions, and terminal
decision gates.

## Inputs

- Canonical workflow name.
- Stable run identifier and state directory.
- Inventory and validation items required by the selected workflow.
- Named human decision owner and their explicit terminal choice.

## Outputs

- ix-flow run state and next valid actions.
- Resumed execution from the last completed phase after interruption.
- A terminal outcome selected through a human gate.

## Behavior

- The onboarding skill SHALL pass a stable run identifier when it starts or
  resumes a workflow.
- When a recorded run exists, the onboarding skill SHALL resume that run without
  replacing completed items or phases.
- The onboarding skill SHALL leave every transition into a terminal phase at the
  `hitl` gate type declared by the canonical workflow.
- When the named owner rejects a run, the onboarding skill SHALL retain the
  workflow's rejection outcome.
- If no named owner supplies a terminal choice, then the onboarding skill SHALL
  leave the run at the decision gate.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-005-AC-1 | Reinvocation with an interrupted run identifier preserves completed phases and returns the next valid action. | Test (TC-026) |
| FR-005-AC-2 | Every transition into a terminal phase remains configured as `hitl`. | Test (TC-027) |
| FR-005-AC-3 | An explicit human rejection produces the workflow's rejection terminal state and no success state. | Test (TC-028) |
| FR-005-AC-4 | With no human terminal choice, the run remains non-terminal at the decision gate. | Test (TC-029) |
| FR-005-AC-5 | A requested automatic terminal-gate override fails closed. | Test (TC-030) |

## Dependencies

- **Upstream**: [US-003](../usecase/US-003-resume-and-decide-workflow.md).
- **Downstream**: ix-flow owns run persistence and transition enforcement.
