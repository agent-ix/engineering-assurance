---
id: FR-002
title: "Expose one canonical onboarding and workflow bundle"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-002"
    type: "implements"
---

# FR-002: Expose one canonical onboarding and workflow bundle

## Description

The engineering-assurance repository SHALL own exactly one canonical
`assurance-onboarding` skill tree containing the promoted assurance workflow
definitions.

## Outputs

- Canonical `assurance-onboarding` skill instructions.
- Canonical workflow definitions for `assurance-intake`,
  `architecture-evaluation`, `measurement-promotion`, and `change-assurance`.
- Thin discovery manifests for Claude Code, Codex, opencode, and GitHub Copilot.

## Behavior

- Each supported agent manifest SHALL resolve the canonical skill tree without
  embedding an agent-specific copy of its behavioral instructions.
- The canonical skill SHALL expose all four named workflows.
- Generic repository-level scaffolding SHALL delegate to the canonical skill.
- If a manifest target is missing or escapes the repository bundle, then the
  discovery smoke test SHALL fail.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-002-CON-1 | Supported agent discovery SHALL cover exactly Claude Code, Codex, opencode, and GitHub Copilot for this release. | Compatibility | Static Test |
| FR-002-CON-2 | Host manifests SHALL contain discovery metadata and canonical references only. | Maintainability | Static Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-002-AC-1 | Repository inspection finds one canonical `assurance-onboarding` skill file. | Test (TC-009) |
| FR-002-AC-2 | Each of the four supported host surfaces resolves that same canonical skill. | Test (TC-010) |
| FR-002-AC-3 | Canonical workflow discovery returns exactly the four promoted workflow names. | Test (TC-011) |
| FR-002-AC-4 | Duplicated behavioral instructions in a host manifest fail the thin-manifest contract test. | Test (TC-012) |
| FR-002-AC-5 | A missing or out-of-bundle canonical target fails discovery. | Test (TC-013) |

## Dependencies

- **Upstream**: [US-002](../usecase/US-002-discover-onboarding-across-agents.md).
- **Downstream**: [FR-003](./FR-003-package-installable-bundle.md) packages the
  canonical tree and manifests.
