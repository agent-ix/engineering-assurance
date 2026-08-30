---
id: IT-002
title: "Package archives preserve module and workflow discovery"
type: IT
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-003"
    type: "verifies"
  - target: "ix://agent-ix/engineering-assurance/FR-007"
    type: "verifies"
  - target: "ix://agent-ix/engineering-assurance/NFR-003"
    type: "verifies"
---

# IT-002: Package archives preserve module and workflow discovery

## Objective

Verify that real Python and npm package archives retain the existing Quire module
root, expose canonical onboarding, and preserve the documented pilot workflow
invocation.

## Target Integration

The integration boundaries are the Python wheel installer, npm archive installer,
Quire module loading, filesystem skill discovery, and ix-flow path loading. Each
boundary is exercised with real archives and real command-line tools.

## Preconditions

The package audit passes, an isolated installation root is empty, and Quire and
ix-flow are installed at recorded exact versions.

## Inputs

- One wheel and one npm archive built from the same source revision.
- A fictional valid module skeleton document.
- Each canonical and pilot workflow name.

## Test Procedure

1. Install each archive into its own isolated root.
   - IT-002-SC-01: both installers exit successfully with no network lookup.
2. Validate the fictional document through the installed module root using
   Quire.
   - IT-002-SC-02: both installed module roots validate the document.
3. Discover the canonical onboarding skill and its four workflows in each root.
   - IT-002-SC-03: both installations expose identical canonical content.
4. Run ix-flow discovery through both the canonical path and compatibility pilot
   path for every workflow.
   - IT-002-SC-04: all eight path-and-workflow combinations load and expose
     equivalent definitions.

## Expected Results

Both package formats install successfully, Quire consumes the unchanged module
root, canonical onboarding is discoverable, and all four pilot invocations remain
compatible with their canonical definitions.

## Metadata

- Priority: P0
- Target Integration: package installers, Quire, and ix-flow
- Automation: Automated

## Dependencies

Real package installers, Quire, ix-flow, and filesystem I/O are required and are
not mocked.

## Traceability

Verifies [FR-003](../functional/FR-003-package-installable-bundle.md),
[FR-007](../functional/FR-007-pilot-compatibility.md), and
[NFR-003](../non-functional/NFR-003-package-contract-stability.md).
