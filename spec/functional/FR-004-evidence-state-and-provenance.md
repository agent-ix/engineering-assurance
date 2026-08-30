---
id: FR-004
title: "Preserve evidence availability and producer provenance"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-004"
    type: "implements"
---

# FR-004: Preserve evidence availability and producer provenance

## Description

The onboarding skill SHALL record one explicit availability state and its
associated provenance or rationale for each evidence producer considered during
onboarding.

## Inputs

- Selected evidence producer identity and invocation result.
- Exact producer version when available.
- Operator-observed command, elapsed time, exit outcome, and diagnostic category.
- Applicability decision for the selected assurance boundary.

## Outputs

- Producer result classified as `observed`, `unavailable`, `not_computed`, or
  `not_applicable`.
- Provenance and operator observation fields appropriate to that state.
- Evidence references delegated to Quoin when evidence is persisted or reported.

## Behavior

- When a producer returns valid output, the onboarding skill SHALL record the
  state as `observed` with its exact version and operator observations.
- When a selected producer cannot be invoked, the onboarding skill SHALL record
  the state as `unavailable` with the observed failure category.
- When a selected producer has not been invoked, the onboarding skill SHALL
  record the state as `not_computed` with the next action or owner.
- When a producer is outside the selected boundary, the onboarding skill SHALL
  record the state as `not_applicable` with the boundary rationale.
- When producer output is malformed, the onboarding skill SHALL retain a
  validation failure without relabeling it as any successful evidence state.
- When evidence is retained or rendered, the onboarding skill SHALL delegate the
  evidence record and policy interpretation to Quoin.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-004-AC-1 | Valid producer output is `observed` and includes an exact producer version, command observation, elapsed time, and exit outcome. | Test (TC-020) |
| FR-004-AC-2 | An invocation failure is `unavailable` and retains the observed failure category. | Test (TC-021) |
| FR-004-AC-3 | A deferred producer is `not_computed` and names a next action or owner. | Test (TC-022) |
| FR-004-AC-4 | An excluded producer is `not_applicable` and includes the decision-boundary rationale. | Test (TC-023) |
| FR-004-AC-5 | Malformed producer output remains a validation failure and is not counted as observed evidence. | Test (TC-024) |
| FR-004-AC-6 | Persisted or reported evidence uses Quoin's evidence and policy surface rather than a module-local substitute. | Test (TC-025) |

## Dependencies

- **Upstream**: [US-004](../usecase/US-004-understand-evidence-availability.md).
- **Downstream**: Quoin owns evidence storage and policy-facing reports.
