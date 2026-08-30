---
id: US-004
title: "Understand evidence availability and provenance"
type: US
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-001"
    type: "traces_to"
  - target: "ix://agent-ix/engineering-assurance/FR-004"
    type: "traces_to"
---

# US-004: Understand evidence availability and provenance

## Story

**As an** engineering operator reviewing an onboarding result
**I want** each evidence source to state whether it was unavailable, not computed, not applicable, or observed with exact provenance
**So that** I can distinguish a tooling failure, deferred work, a boundary decision, and actual evidence.

## Context

Missing output is ambiguous without a declared state. A report that merges all
absence into success or failure obscures both risk and the next action. Produced
evidence is likewise incomplete without the producer version and operator-visible
execution observations.

## Acceptance Examples (Illustrative)

### US-004-EX-1: Unavailable producer remains visible

- **Given** a selected producer that cannot be invoked
- **When** onboarding records its result
- **Then** the producer is marked `unavailable` with the observed failure

### US-004-EX-2: Produced evidence retains provenance

- **Given** a producer that returns valid evidence
- **When** onboarding records its result
- **Then** the record includes the exact producer version and operator observations

## Dependencies (Contextual)

Quoin owns persisted evidence records and policy-facing reporting. Quire owns
validation of artifact structure. Onboarding preserves their results.

## Priority and Risk (Informative)

- **Priority:** P0.
- **Primary risk:** missing provenance or collapsed availability states can turn a
  tooling failure or deferred observation into apparent evidence.
