---
id: FR-006
title: "Evaluate onboarding behavior through supported agents"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-001"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/US-003"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/US-004"
    type: "implements"
---

# FR-006: Evaluate onboarding behavior through supported agents

## Description

The onboarding release gate SHALL execute a reproducible agent-evaluation suite
covering the required repository, producer-failure, interruption, and human
rejection scenarios.

## Inputs

- Fictional repository fixtures with fixed revisions and declared expectations.
- Installed onboarding bundle and exact supported-agent versions.
- Real Quire, Quoin, and ix-flow executables required by each scenario.

## Outputs

- Per-scenario transcript, exact tool versions, command count, elapsed time,
  observed outcome, and pass or fail result.
- Aggregate result that remains failed while any required scenario fails or lacks
  evidence.

## Behavior

- The suite SHALL evaluate an existing repository with an applicable valid
  profile.
- The suite SHALL evaluate a repository for which no profile is justified.
- The suite SHALL evaluate malformed and unavailable producer outcomes.
- The suite SHALL evaluate interruption followed by resume.
- The suite SHALL evaluate explicit human rejection.
- If a required executable is unavailable, then the suite SHALL report the
  affected scenario as not executed and fail the release gate.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-006-AC-1 | The suite contains and executes all five required scenario classes. | Test (TC-031) |
| FR-006-AC-2 | Every executed scenario records exact agent and tool versions, command count, elapsed time, transcript reference, and observed outcome. | Test (TC-032) |
| FR-006-AC-3 | A missing executable leaves the scenario not executed and the aggregate gate failed. | Test (TC-033) |
| FR-006-AC-4 | The aggregate gate passes only when all required scenarios pass and have complete observations. | Test (TC-034) |

## Dependencies

- **Upstream**: [FR-001](./FR-001-inventory-before-proposal.md),
  [FR-004](./FR-004-evidence-state-and-provenance.md), and
  [FR-005](./FR-005-resumable-human-decisions.md).
