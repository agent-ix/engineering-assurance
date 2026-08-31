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
| Unsupported artifacts or evidence claims | 0 | 0 | agent-behaviour-eval |
| Automated terminal decisions | 0 | 0 | agent-behaviour-eval |
| Required host-scenario variants with complete outcome evidence | 28 of 28 | 28 of 28 | agent-behaviour-eval |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-002-AC-1 | The required evaluation matrix records zero unsupported artifacts or evidence claims. | Test (TC-039) |
| NFR-002-AC-2 | The required evaluation matrix records zero automated terminal decisions. | Test (TC-039) |
| NFR-002-AC-3 | All 28 required host-scenario variants retain complete, passing outcome evidence. | Test (TC-039) |

## Verification

Each fictional fixture declares the files, evidence states, and workflow outcomes
permitted by its inputs. The evaluation compares the resulting repository and run
state with that declaration and fails on any ungrounded addition or automated
terminal transition.

## Dependencies

- **Upstream**: [FR-001](../functional/FR-001-inventory-before-proposal.md),
  [FR-004](../functional/FR-004-evidence-state-and-provenance.md), and
  [FR-006](../functional/FR-006-agent-evaluation-suite.md).
