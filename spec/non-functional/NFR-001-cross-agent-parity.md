---
id: NFR-001
title: "Supported agents resolve identical canonical content"
type: NFR
quality_attribute: compatibility
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-002"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-003"
    type: "constrains"
---

# NFR-001: Supported agents resolve identical canonical content

## Statement

The installed onboarding bundle SHALL resolve byte-identical canonical skill and
workflow content from every supported agent discovery surface.

## Scope

- Applies to Claude Code, Codex, opencode, and GitHub Copilot discovery.
- Applies to local-source and repository-source installations.
- Applies to the onboarding skill and all four promoted workflows.

## Rationale

Host-specific behavioral copies can drift while appearing to offer the same
capability. One canonical target makes equivalence directly verifiable and keeps
maintenance in the owning module.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Supported discovery surfaces resolving the canonical skill | 4 of 4 | 4 of 4 | integration-testing |
| Promoted workflows with identical canonical digests across hosts | 4 of 4 | 4 of 4 | property-based-testing |
| Agent-specific behavioral copies | 0 | 0 | architecture-conformance |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-001-AC-1 | All four supported host surfaces resolve the canonical skill from both supported installation sources. | Test (TC-038) |
| NFR-001-AC-2 | All four promoted workflows have identical canonical bytes across hosts and installation sources. | Test (TC-038) |
| NFR-001-AC-3 | No host manifest contains an agent-specific copy of behavioral onboarding content. | Test (TC-038) |

## Verification

Install the bundle from each supported source, resolve each host surface, and
compare the resolved canonical file paths and content digests. Scan host manifests
for behavioral sections prohibited by the thin-manifest contract.

## Dependencies

- **Upstream**: [FR-002](../functional/FR-002-canonical-discovery-bundle.md) and
  [FR-003](../functional/FR-003-package-installable-bundle.md).
