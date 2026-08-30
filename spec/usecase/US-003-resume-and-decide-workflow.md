---
id: US-003
title: "Resume interrupted assurance work and decide explicitly"
type: US
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-001"
    type: "traces_to"
  - target: "ix://agent-ix/engineering-assurance/FR-005"
    type: "traces_to"
---

# US-003: Resume interrupted assurance work and decide explicitly

## Story

**As a** human assurance decision owner
**I want** interrupted onboarding work to resume from its recorded phase and wait for my terminal choice
**So that** an agent restart neither repeats accepted evidence nor silently approves or rejects the work.

## Context

Assurance work often spans multiple sessions. ix-flow already persists run state
and supports human-gated terminal transitions; onboarding needs to preserve and
use that behavior rather than create a second lifecycle.

## Acceptance Examples (Illustrative)

### US-003-EX-1: Interrupted work resumes

- **Given** a run interrupted after a non-terminal phase
- **When** onboarding continues with the recorded run identifier
- **Then** completed phases remain complete and the next valid action is shown

### US-003-EX-2: Rejection remains explicit

- **Given** a run at a terminal decision gate
- **When** the named owner selects rejection
- **Then** the run records the rejection and no success outcome is synthesized

## Dependencies (Contextual)

The run identifier and state directory must remain available to ix-flow. A named
human owns each terminal transition.
