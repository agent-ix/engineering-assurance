---
id: IT-004
title: "Interrupted workflow resumes and retains human terminal decisions"
type: IT
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-005"
    type: "verifies"
  - target: "ix://agent-ix/engineering-assurance/FR-006"
    type: "verifies"
  - target: "ix://agent-ix/engineering-assurance/NFR-002"
    type: "verifies"
---

# IT-004: Interrupted workflow resumes and retains human terminal decisions

## Objective

Verify that a real ix-flow assurance run interrupted before its terminal gate
resumes from retained state and records explicit human acceptance and rejection
on equivalent runs without an automated terminal transition.

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
- A named fictional decision owner and explicit acceptance and rejection actions
  applied to separate equivalent runs.

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
4. Fork two equivalent decision-ready fixture runs with distinct run identifiers.
   - IT-004-SC-04: neither run has a terminal event before a human action.
5. Supply the named owner's explicit rejection to one run and explicit acceptance
   to the other.
   - IT-004-SC-05: each workflow records exactly one attributed terminal event,
     with opposite choices and no synthesized or duplicate terminal state.

## Expected Results

The resumed run retains its prior state and waits at the human gate. Equivalent
runs record explicit acceptance and rejection exactly once with owner, workflow
version, run id, and timestamp. The onboarding layer does not own or override the
lifecycle decision.

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
