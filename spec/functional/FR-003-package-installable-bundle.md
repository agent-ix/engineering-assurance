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
  contract. Each artifact type SHALL have exactly one schema file, a JSON
  Schema 2020-12 document, named by both its `frontmatter_schema_ref` and its
  `data_schema`, whose `$id` carries the manifest version and whose recorded
  digest is the SHA-256 of its raw bytes. No second copy of a schema SHALL
  exist.
- The exported schema of an artifact type describes that type's frontmatter
  only; body sections remain the type's `body_extraction`. The consumer applies
  a `data_schema` to the declaration record of an archetype named by a
  document's `object:` key, not to a `type:`-backed document, so the export
  pins the frontmatter contract for typed consumers and does not describe an
  extracted record.
- Every constraint of these schemas SHALL be a keyword a default-options
  2020-12 consumer asserts: a date-time field SHALL use one documented RFC 3339
  `pattern` (shape and field ranges, not calendar validity), never `format`.
  The digested schema files SHALL be excluded from line-ending conversion.
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
| FR-003-AC-7 | Every name in `semantic.exports` is a declared type with a `data_schema`; every recorded `data_schema` digest equals the SHA-256 of its file; every exported schema declares 2020-12 and an `$id` under the manifest version; each artifact type's `data_schema` is the same file as its `frontmatter_schema_ref`; the artifact schemas use no draft-07 form and no `format`; and the digested files contain no carriage return and are excluded from line-ending conversion. | Test (TC-194) |
| FR-003-AC-8 | Over every skeleton, every top-level single-field mutation of it, the decision-rule, `margin_mode` and interval-level cases, the retired-plan cases and nested `review_by` date-time cases, each artifact schema, built at default validator options, gives the verdict the original draft-07 schema gave, except that calendar validity of a date-time is not asserted. | Test (TC-195) |

## Dependencies

- **Upstream**: [FR-002](./FR-002-canonical-discovery-bundle.md).
- **Downstream**: Quire consumes the installed module root without a new path
  convention.
