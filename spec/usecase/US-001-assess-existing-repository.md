---
id: US-001
title: "Assess an existing repository before proposing assurance work"
type: US
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-001"
    type: "traces_to"
  - target: "ix://agent-ix/engineering-assurance/FR-001"
    type: "traces_to"
---

# US-001: Assess an existing repository before proposing assurance work

## Story

**As an** engineering operator onboarding an existing repository
**I want** the agent to inspect its decisions, measurements, artifacts, and evidence producers first
**So that** any proposed assurance work is specific to the repository and its actual decision needs.

## Context

Repositories differ in both the artifacts they already contain and the decisions
they need to support. The useful starting point is therefore an inventory and a
bounded recommendation, not a fixed set of generated files.

## Acceptance Examples (Illustrative)

### US-001-EX-1: Existing artifacts are reused

- **Given** a repository with a valid applicable AssuranceProfile
- **When** onboarding assesses the repository
- **Then** the existing profile is inventoried and no replacement is proposed

### US-001-EX-2: No profile is justified

- **Given** a repository whose selected decision does not justify a profile
- **When** onboarding assesses the repository
- **Then** no generic profile is created and the reason is reported

## Dependencies (Contextual)

The assessment depends on repository-readable files and on the operator defining
the decision boundary. Quire may validate discovered artifacts after inventory.
