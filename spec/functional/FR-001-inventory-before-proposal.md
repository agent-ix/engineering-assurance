---
id: FR-001
title: "Inventory repository assurance context before proposing artifacts"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-001"
    type: "implements"
---

# FR-001: Inventory repository assurance context before proposing artifacts

## Description

When onboarding is invoked in an existing repository, the onboarding skill SHALL
complete a repository assurance inventory before proposing any AssuranceProfile,
MeasurementPlan, or governed workflow.

## Inputs

- Repository root selected by the operator.
- Operator-stated decision boundary and decision owner.
- Existing assurance artifacts, measurement definitions, test configuration,
  evidence references, and producer configuration visible in the repository.

## Outputs

- An inventory of found decisions, measurements, assurance artifacts, evidence
  producers, and unresolved inputs.
- A bounded recommendation that names existing artifacts to reuse, justified
  artifacts to author, a workflow to enter, or no applicable assurance work.

## Behavior

- The onboarding skill SHALL inspect existing decision and measurement material
  before it recommends new assurance artifacts.
- If an applicable valid artifact already exists, then the onboarding skill SHALL
  recommend reusing that artifact.
- If the selected decision boundary does not justify an AssuranceProfile, then
  the onboarding skill SHALL create no generic AssuranceProfile.
- If the selected decision boundary does not justify a MeasurementPlan, then the
  onboarding skill SHALL create no generic MeasurementPlan.
- When an artifact is justified, the onboarding skill SHALL delegate its
  validation to Quire.
- If the selected decision boundary is incomplete, then the onboarding skill SHALL
  request the missing human input without creating an artifact.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-001-AC-1 | Direct invocation in a repository with an applicable valid profile inventories and reuses that profile before proposing other work. | Test (TC-004) |
| FR-001-AC-2 | Direct invocation in a repository with no justified profile reports that result and creates no AssuranceProfile. | Test (TC-005) |
| FR-001-AC-3 | A justified new artifact is rendered from the installed module skeleton and accepted by Quire before it is reported as valid. | Test (TC-006) |
| FR-001-AC-4 | An incomplete decision boundary produces a request for the missing human input and no generated assurance artifact. | Test (TC-007) |
| FR-001-AC-5 | The inventory lists discovered decisions, measurements, artifacts, producer configurations, and unresolved inputs as separate collections. | Test (TC-008) |

## Dependencies

- **Upstream**: [US-001](../usecase/US-001-assess-existing-repository.md).
- **Downstream**: Quire validates any artifact justified by the inventory.
