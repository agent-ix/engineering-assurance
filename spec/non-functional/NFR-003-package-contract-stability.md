---
id: NFR-003
title: "Package promotion preserves the module-root contract"
type: NFR
quality_attribute: maintainability
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-003"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-007"
    type: "constrains"
---

# NFR-003: Package promotion preserves the module-root contract

## Statement

When onboarding is promoted, each audited package SHALL retain every previously
allowlisted module-root member and add only explicitly allowlisted onboarding,
manifest, and installation members.

## Scope

- Applies to both the private Python wheel and private npm archive.
- Applies to existing module manifest, schema, skeleton, license, and rights
  members plus newly canonical onboarding files.

## Rationale

Quire consumes the installed module by its current root layout, while agents need
additional discovery content. Expanding package membership must not relocate or
silently remove the existing module contract.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Previously allowlisted module-root members missing from either package | 0 | 0 | contract-testing |
| Emitted members absent from the explicit package allowlist | 0 | 0 | contract-testing |
| Existing pilot workflows loadable after promotion | 4 of 4 | 4 of 4 | integration-testing |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-003-AC-1 | Neither audited package omits a previously allowlisted module-root member. | Test (TC-040) |
| NFR-003-AC-2 | Neither audited package emits a member outside its explicit allowlist. | Test (TC-040) |
| NFR-003-AC-3 | All four existing pilot workflows remain loadable after package installation. | Test (TC-035, TC-040) |

## Verification

Build both package formats into an isolated directory, enumerate every member,
run the content-rights audit, compare against explicit allowlists, install each
archive, and exercise both module-root and compatibility discovery.

## Dependencies

- **Upstream**: [FR-003](../functional/FR-003-package-installable-bundle.md) and
  [FR-007](../functional/FR-007-pilot-compatibility.md).
