---
id: AP-001
title: Juniper release decision profile
type: AssuranceProfile
status: proposed
owner: juniper-release-owner
profile_version: 0.2
profile_kind: production-ai
scope: one candidate revision of the fictional Juniper service
impact_assessments:
  - id: impact-request-loss
    scenario: a valid request is accepted but its result is not retained
    severity: material
    verifiability:
      class: probabilistic
      stochastic_dependency: subject
    detect_before_harm:
      expected: true
      control_ref: ix://example/juniper/CAC-001
review_policy:
  mode: require
  operations: [code-review, gap-analysis]
relationships: []
---

# Juniper release decision profile

## Decision Boundary

This profile covers one identified candidate revision and its declared runtime
configuration. It does not authorize later revisions or different settings.

## Impact Scenarios

Each scenario names an observable adverse outcome, its severity, and the kind
of evidence capable of reducing uncertainty about it.

## Evidence Policy

Evidence must identify its producer, subject revision, collection conditions,
and limitations. A measurement is not a decision by itself.

## Exceptions

An exception records an owner, rationale, expiry, affected scenario, and the
decision that accepted it.
