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
  human-prompt count, manual-translation count, repeated-prompt count, observed
  outcome, terminal decision event when applicable, and pass or fail result.
- A machine-readable evaluation envelope retaining suite and fixture revisions,
  host identity/version, the complete FR-004 governing-version tuple, transcript
  path and digest, and every operator-effort observation.
- Aggregate result that remains failed while any required scenario fails or lacks
  evidence.

## Behavior

- For each of the four supported agent hosts, the suite SHALL evaluate an existing
  repository with an applicable valid profile.
- For each supported host, the suite SHALL evaluate a repository for which no
  profile is justified.
- For each supported host, the suite SHALL evaluate both malformed and unavailable
  producer outcomes.
- For each supported host, the suite SHALL evaluate interruption followed by
  resume.
- For each supported host, the suite SHALL evaluate explicit human acceptance and
  explicit human rejection from equivalent decision-ready runs.
- Every transcript and result SHALL use fictional fixtures owned by this repository
  and SHALL pass the repository content-rights check before retention.
- If a required executable is unavailable, then the suite SHALL report the
  affected scenario as not executed and fail the release gate.
- The suite SHALL bind the exact snapshotted ix-flow executable into each agent
  environment and into post-run verification so PATH normalization cannot select
  a different implementation.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-006-AC-1 | The suite executes all five required scenario classes across each of the four supported hosts, including both malformed/unavailable producer variants and both terminal choices. | Test (TC-031) |
| FR-006-AC-2 | Every executed scenario records the immutable governing-version tuple, command and human-interaction counts, elapsed time, transcript path/digest, fixture revision, observed outcome, and terminal event when applicable. | Test (TC-032) |
| FR-006-AC-3 | A missing executable leaves the scenario not executed and the aggregate gate failed. | Test (TC-033) |
| FR-006-AC-4 | The aggregate gate passes only when all required scenarios pass and have complete observations. | Test (TC-034) |
| FR-006-AC-5 | Equivalent decision-ready fixtures retain one explicit acceptance and one explicit rejection per supported host, with no inferred terminal event. | Test (TC-049) |
| FR-006-AC-6 | Every agent and post-run verifier executes the same pinned ix-flow binary named by the governing snapshot. | Test (TC-050) |

## Dependencies

- **Upstream**: [FR-001](./FR-001-inventory-before-proposal.md),
  [FR-004](./FR-004-evidence-state-and-provenance.md), and
  [FR-005](./FR-005-resumable-human-decisions.md).
