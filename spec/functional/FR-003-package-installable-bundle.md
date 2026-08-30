---
id: FR-003
title: "Package the canonical onboarding bundle"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-002"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-002"
    type: "depends_on"
---

# FR-003: Package the canonical onboarding bundle

## Description

When a Python wheel or npm archive is built, the package SHALL include the
canonical onboarding bundle and supported discovery manifests alongside the
existing engineering-assurance module root.

## Inputs

- Repository-owned Python and npm package definitions.
- Canonical skill, workflow, manifest, schema, skeleton, license, and install
  documentation files.

## Outputs

- Audited private Python wheel.
- Audited private npm archive.
- Separate local-source and repository-source installation instructions.

## Behavior

- The package audit SHALL compare every emitted member with an explicit
  repository-owned allowlist.
- A local-source installation SHALL preserve canonical discovery paths.
- A repository-source installation SHALL preserve canonical discovery paths.
- Installation documentation SHALL distinguish module installation from agent-
  plugin installation and SHALL give separate local-source and repository-source
  procedures for each applicable surface.
- The installed module root SHALL continue to expose `manifest.yaml`, `schemas/`,
  and `skeletons/` at the paths consumed by Quire.
- If either package contains an unallowlisted member, then the package audit SHALL
  fail.
- If a packaged manifest or link resolves outside its installed bundle, then the
  package audit SHALL fail.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-003-AC-1 | The wheel member set contains the existing module root plus the canonical skill, workflows, supported manifests, and installation documentation. | Test (TC-014) |
| FR-003-AC-2 | The npm archive member set contains the existing module root plus the canonical skill, workflows, supported manifests, and installation documentation. | Test (TC-015) |
| FR-003-AC-3 | Local-source installation resolves module and onboarding discovery from the installed tree. | Test (TC-016) |
| FR-003-AC-4 | Repository-source installation resolves module and onboarding discovery from the installed tree. | Test (TC-017) |
| FR-003-AC-5 | An unexpected or missing package member, or any installed manifest/link that escapes its bundle, fails the package audit. | Test (TC-018) |
| FR-003-AC-6 | Install documentation separates module installation from agent-plugin installation and presents local-source and repository-source procedures in distinct sections. | Test (TC-019) |

## Dependencies

- **Upstream**: [FR-002](./FR-002-canonical-discovery-bundle.md).
- **Downstream**: Quire consumes the installed module root without a new path
  convention.
