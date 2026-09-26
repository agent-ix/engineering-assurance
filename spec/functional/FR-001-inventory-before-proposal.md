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
- The onboarding report (`scripts/onboard.js` in the skill), which a reader
  runs once before authoring to see how the module relates to the
  repository and what each artifact type requires.
- With `--summary`, a compact report of the module version, each
  `spec/assurance/` artifact's Quire validation status, and the installed
  Quire and Quoin toolchain, whose exit status is the validation outcome.

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
- When a justified artifact is authored, the onboarding skill SHALL stage it
  within the repository, validate the staged artifact with Quire, and publish it
  by atomic same-directory rename only after validation succeeds.
- If existing applicable artifacts are malformed or conflict, then the onboarding
  skill SHALL preserve them unchanged, report each path and validation result, and
  request a human selection or correction rather than choosing or replacing one.
- The onboarding skill SHALL confine reads and writes to paths that resolve within
  the operator-selected repository root and SHALL refuse an absolute, parent-
  traversing, or symlink-escaping target.
- If the selected decision boundary is incomplete, then the onboarding skill SHALL
  request the missing human input without creating an artifact.
- When the onboarding report is invoked with `--help` or `-h`, the onboarding
  report SHALL print its usage to standard output and exit 0 without
  inventorying a repository.
- If the onboarding report is invoked with an unknown argument, a `--repo`
  without a path, or more than one `--repo`, then the onboarding report SHALL
  print the error and its usage to standard error and exit 2 without
  inventorying a repository.
- When the onboarding report is invoked with `--summary`, it SHALL print,
  instead of the full report, the module version in this checkout, each
  `spec/assurance/` artifact with its Quire validation status (`valid`,
  `invalid` with at most ten findings and a count of the rest, or
  `unavailable` when Quire did not run to completion), and the installed
  Quire and Quoin versions and whether `quoin measurement verify` exists,
  each `null` when it cannot be observed. It SHALL exit 1 when any artifact
  is invalid, 3 when validation was unavailable and none is invalid, and 0
  only when every artifact validated.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-001-AC-1 | Direct invocation in a repository with an applicable valid profile inventories and reuses that profile before proposing other work. | Test (TC-004) |
| FR-001-AC-2 | Direct invocation in a repository with no justified profile reports that result and creates no AssuranceProfile. | Test (TC-005) |
| FR-001-AC-3 | A justified new artifact is rendered from the installed module skeleton and accepted by Quire before it is reported as valid. | Test (TC-006) |
| FR-001-AC-4 | An incomplete decision boundary produces a request for the missing human input and no generated assurance artifact. | Test (TC-007) |
| FR-001-AC-5 | The inventory lists discovered decisions, measurements, artifacts, producer configurations, and unresolved inputs as separate collections. | Test (TC-008) |
| FR-001-AC-6 | Malformed or conflicting applicable artifacts remain byte-unchanged, every path and validation result is reported, and no replacement is selected without human input. | Test (TC-044) |
| FR-001-AC-7 | A justified artifact becomes visible only after staged Quire validation and atomic rename; validation failure or an escaping target leaves the intended path absent. | Test (TC-045) |
| FR-001-AC-8 | `--help` and `-h` print the onboarding report's usage to standard output and exit 0; an unknown argument, a `--repo` without a path, or a repeated `--repo` prints the error and the usage to standard error and exits 2; none of them prints a report. | Test (TC-174) |
| FR-001-AC-9 | With `--summary`, an artifact Quire accepts reports `valid` with no findings and one it refuses reports `invalid` with its findings, capped at ten with the remainder counted; the summary carries this checkout's module version and the observed Quire and Quoin versions, `null` for one not installed, and omits the full report's orientation sections; it exits 1 when any artifact is invalid, 3 with a stated reason when Quire is not installed, and 0 when every artifact validates. | Test (TC-202) |

## Dependencies

- **Upstream**: [US-001](../usecase/US-001-assess-existing-repository.md).
- **Downstream**: Quire validates any artifact justified by the inventory.
