---
id: IT-003
title: "Existing-repository onboarding preserves bounded evidence states"
type: IT
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-001"
    type: "verifies"
  - target: "ix://agent-ix/engineering-assurance/FR-004"
    type: "verifies"
  - target: "ix://agent-ix/engineering-assurance/FR-006"
    type: "verifies"
  - target: "ix://agent-ix/engineering-assurance/NFR-002"
    type: "verifies"
---

# IT-003: Existing-repository onboarding preserves bounded evidence states

## Objective

Verify direct onboarding against real fictional repositories with an applicable
profile, no justified profile, malformed producer output, and an unavailable
producer, while preserving inventory order and explicit evidence states.

## Target Integration

The integration boundary includes the supported agent harness, repository
filesystem, real Quire validation, real producer processes where available, and
the Quoin evidence/reporting command surface used by the onboarding result.

## Preconditions

Four fictional repository fixtures are checked out at fixed revisions. The
onboarding bundle, agent harness, Quire, Quoin, and required fixture producers are
installed with exact versions recorded by the test runner.

## Inputs

- Fixture with a valid applicable AssuranceProfile.
- Fixture whose selected decision does not justify a profile.
- Fixture whose producer returns malformed output.
- Fixture configured with a selected producer executable that is absent.

## Test Procedure

1. Invoke onboarding directly in the existing-profile fixture.
   - IT-003-SC-01: inventory precedes recommendation and reuses the validated
     profile.
2. Invoke onboarding directly in the no-profile fixture.
   - IT-003-SC-02: no profile is created and the bounded rationale is reported.
3. Invoke onboarding in the malformed-output fixture.
   - IT-003-SC-03: validation fails and no observed evidence is reported.
4. Invoke onboarding in the unavailable-producer fixture.
   - IT-003-SC-04: the result is `unavailable` with the process failure category.
5. Inspect all filesystem changes, Quoin records, transcripts, and observations.
   - IT-003-SC-05: all changes are fixture-authorized and every scenario records
     exact versions, commands, elapsed time, and outcomes.

## Expected Results

Existing material is reused, unjustified scaffolding is absent, malformed output
and producer unavailability remain distinct, and no unsupported artifact,
evidence claim, or policy conclusion is created.

## Metadata

- Priority: P0
- Target Integration: agent harness, Quire, Quoin, and repository filesystem
- Automation: Automated

## Dependencies

Real filesystem, subprocess, Quire validation, and Quoin evidence/reporting paths
are required. Only the fictional producer payload is fixture-controlled.

## Traceability

Verifies [FR-001](../functional/FR-001-inventory-before-proposal.md),
[FR-004](../functional/FR-004-evidence-state-and-provenance.md),
[FR-006](../functional/FR-006-agent-evaluation-suite.md), and
[NFR-002](../non-functional/NFR-002-non-inventing-onboarding.md).
