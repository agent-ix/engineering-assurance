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
- The module manifest SHALL export each of its five artifact types
  (`AssuranceProfile`, `MeasurementPlan`, `ArchitectureDescription`,
  `ComponentAssuranceContract`, `AssuranceArgument`) through its semantic
  contract, each with a `data_schema` naming a JSON Schema 2020-12 file whose
  `$id` carries the manifest version and whose digest is the SHA-256 of its
  bytes. Each such file SHALL be derived mechanically from the artifact type's
  draft-07 frontmatter schema, and SHALL accept and refuse the same documents.
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
| FR-003-AC-7 | Every name in `semantic.exports` is a declared type with a `data_schema`; every recorded `data_schema` digest equals the SHA-256 of its file; every exported schema declares 2020-12 and an `$id` under the manifest version; and each of the five artifact-type files equals the mechanical transformation of its draft-07 source. | Test (TC-194) |
| FR-003-AC-8 | Over every skeleton, every single-field mutation of it, the decision-rule and `margin_mode` cases and the retired-plan cases, each 2020-12 export accepts exactly the documents its draft-07 source accepts. | Test (TC-195) |

## Dependencies

- **Upstream**: [FR-002](./FR-002-canonical-discovery-bundle.md).
- **Downstream**: Quire consumes the installed module root without a new path
  convention.
