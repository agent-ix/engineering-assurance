---
id: FR-007
title: "Preserve existing pilot workflow invocation"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-002"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-002"
    type: "depends_on"
---

# FR-007: Preserve existing pilot workflow invocation

## Description

When canonical onboarding is promoted, the module SHALL preserve the documented
path-based invocation of each existing pilot workflow for this release's
compatibility period.

## Inputs

- Existing `pilots/assurance-workflows` path.
- The four documented pilot workflow names.
- Canonical promoted workflow definitions.

## Outputs

- Successful ix-flow discovery through both the pilot path and canonical path.
- Contract test showing that both paths expose equivalent workflow definitions.

## Behavior

- The pilot path SHALL remain loadable for each documented workflow name.
- The pilot path SHALL resolve content equivalent to the corresponding canonical
  workflow definition.
- New installation documentation SHALL identify the canonical path as the primary
  entry point.
- If pilot and canonical workflow definitions diverge, then the compatibility
  contract test SHALL fail.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-007-CON-1 | Compatibility SHALL cover the four pilot workflow names present before promotion. | Compatibility | Integration Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-007-AC-1 | Each of the four existing pilot workflow invocations remains loadable by ix-flow. | Test (TC-035) |
| FR-007-AC-2 | Pilot and canonical discovery yield equivalent workflow names, versions, phases, transitions, interviews, and item schemas. | Test (TC-036) |
| FR-007-AC-3 | Installation documentation presents the canonical invocation first and labels the pilot path as compatible. | Inspection (TC-037) |

## Dependencies

- **Upstream**: [FR-002](./FR-002-canonical-discovery-bundle.md).
- **Downstream**: ix-flow loads both paths during the compatibility period.
