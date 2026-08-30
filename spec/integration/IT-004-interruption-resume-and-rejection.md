---
id: IT-004
title: "Interrupted workflow resumes and retains human rejection"
type: IT
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-005"
    type: "verifies"
  - target: "ix://agent-ix/engineering-assurance/FR-006"
    type: "verifies"
  - target: "ix://agent-ix/engineering-assurance/NFR-002"
    type: "verifies"
---

# IT-004: Interrupted workflow resumes and retains human rejection

## Objective

Verify that a real ix-flow assurance run interrupted before its terminal gate
resumes from retained state and records an explicit human rejection without an
automated success transition.

## Target Integration

The integration boundary is the canonical onboarding skill invoking ix-flow with
a real workflow definition, state directory, run identifier, persisted items,
and human gate action.

## Preconditions

The canonical workflow bundle and ix-flow are installed. An empty temporary state
directory is available, and the harness can stop and restart the agent session
without deleting process-independent run state.

## Inputs

- A fictional repository fixture with sufficient validated items to reach the
  decision gate.
- A stable run identifier.
- A named fictional decision owner and an explicit rejection action.

## Test Procedure

1. Start onboarding and advance the selected workflow through one non-terminal
   phase.
   - IT-004-SC-01: ix-flow persists the completed phase and run identifier.
2. Terminate the agent session and start a new session with the same run
   identifier and state directory.
   - IT-004-SC-02: ix-flow reports the recorded phase and next valid action without
     repeating completed work.
3. Advance the run to its human terminal decision gate.
   - IT-004-SC-03: the run remains non-terminal until a human action is supplied.
4. Supply the named owner's explicit rejection action.
   - IT-004-SC-04: the workflow records its rejection terminal state and no success
     terminal state.

## Expected Results

The resumed run retains its prior state, waits at the human gate, and records the
explicit rejection exactly once. The onboarding layer does not own or override
the lifecycle decision.

## Metadata

- Priority: P0
- Target Integration: ix-flow run state and human gate
- Automation: Automated with scripted human input

## Dependencies

Real ix-flow execution and real filesystem state are required and are not mocked.
The human choice is scripted as explicit test input rather than inferred by the
agent.

## Traceability

Verifies [FR-005](../functional/FR-005-resumable-human-decisions.md),
[FR-006](../functional/FR-006-agent-evaluation-suite.md), and
[NFR-002](../non-functional/NFR-002-non-inventing-onboarding.md).
