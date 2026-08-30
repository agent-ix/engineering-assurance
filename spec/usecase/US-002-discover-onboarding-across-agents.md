---
id: US-002
title: "Discover onboarding across supported coding agents"
type: US
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-001"
    type: "traces_to"
  - target: "ix://agent-ix/engineering-assurance/FR-002"
    type: "traces_to"
  - target: "ix://agent-ix/engineering-assurance/FR-003"
    type: "traces_to"
---

# US-002: Discover onboarding across supported coding agents

## Story

**As a** developer using a supported coding agent
**I want** that agent to discover the module's onboarding capability after installation
**So that** I receive the same governed workflow regardless of which supported agent hosts the session.

## Context

Claude Code, Codex, opencode, and GitHub Copilot use different discovery files.
Those host-specific files should expose one module-owned skill and workflow tree,
not create four independently evolving implementations.

## Acceptance Examples (Illustrative)

### US-002-EX-1: Canonical skill discovery

- **Given** a local-source installation for any supported agent
- **When** the agent scans its discovery surface
- **Then** it resolves the canonical `assurance-onboarding` skill

### US-002-EX-2: Canonical workflow discovery

- **Given** the installed onboarding skill
- **When** the agent requests a named assurance workflow
- **Then** it resolves the module-owned definition for that workflow

## Dependencies (Contextual)

Discovery depends on the host's supported manifest convention and the installed
package retaining the canonical skill and workflow directories.
