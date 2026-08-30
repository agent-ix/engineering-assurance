---
id: NFR-002
title: "Onboarding does not invent assurance posture"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-001"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-004"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-006"
    type: "constrains"
---

# NFR-002: Onboarding does not invent assurance posture

## Statement

Across the required agent-evaluation suite, onboarding SHALL introduce zero
unsupported artifacts, evidence claims, applicability decisions, or terminal
outcomes.

## Scope

- Applies to every required evaluation scenario and supported agent.
- Applies to generated files, reports, workflow items, and terminal states.

## Rationale

The central reliability property of onboarding is epistemic: absence, failure,
and human judgment must remain visible instead of being converted into a complete
assurance posture. A successful-looking artifact with no repository basis is a
failure, not a convenience.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Unsupported artifacts or evidence claims | 0 | 0 | Agent Evaluation |
| Automated terminal decisions | 0 | 0 | Agent Evaluation |
| Required scenario classes with complete outcome evidence | 5 of 5 | 5 of 5 | Agent Evaluation |

## Verification

Each fictional fixture declares the files, evidence states, and workflow outcomes
permitted by its inputs. The evaluation compares the resulting repository and run
state with that declaration and fails on any ungrounded addition or automated
terminal transition.

## Dependencies

- **Upstream**: [FR-001](../functional/FR-001-inventory-before-proposal.md),
  [FR-004](../functional/FR-004-evidence-state-and-provenance.md), and
  [FR-006](../functional/FR-006-agent-evaluation-suite.md).
