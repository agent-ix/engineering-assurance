---
id: IT-001
title: "Local-source install exposes canonical agent discovery"
type: IT
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-002"
    type: "verifies"
  - target: "ix://agent-ix/engineering-assurance/FR-003"
    type: "verifies"
  - target: "ix://agent-ix/engineering-assurance/NFR-001"
    type: "verifies"
---

# IT-001: Local-source install exposes canonical agent discovery

## Objective

Verify that a real local-source installation exposes the same canonical
onboarding skill and workflow definitions through all four supported agent
discovery surfaces without copying behavioral content.

## Target Integration

The integration boundary is the packaged engineering-assurance repository tree
and the filesystem discovery conventions used by Claude Code, Codex, opencode,
and GitHub Copilot. Real files, links, and manifest parsers are used.

## Preconditions

The source checkout is clean, the package build has passed its rights audit, and
an empty temporary installation root is available. All four host discovery
adapters are available to the smoke-test harness.

## Inputs

- The repository path as the local installation source.
- The four supported host identifiers.
- Expected canonical skill name and four workflow names.

## Test Procedure

1. Install the repository into the empty temporary root using the documented
   local-source procedure.
   - IT-001-SC-01: installation exits successfully and creates one canonical
     onboarding skill tree.
2. Invoke each supported host discovery adapter against the installed root.
   - IT-001-SC-02: all four adapters resolve the canonical onboarding skill.
3. Resolve workflow contributions from each discovered skill.
   - IT-001-SC-03: every host returns the same four canonical workflow files.
4. Compare canonical paths, content digests, and host manifest bodies.
   - IT-001-SC-04: canonical digests match and host manifests contain no copied
     behavioral instructions.

## Expected Results

All four supported hosts resolve the same skill and workflow files. The installed
tree contains one behavioral implementation, and each host-specific surface is a
thin discovery adapter.

## Metadata

- Priority: P0
- Target Integration: supported-agent filesystem discovery
- Automation: Automated

## Dependencies

The package builder and all four host discovery adapters must be available. No
filesystem I/O or manifest parsing is mocked.

## Traceability

Verifies [FR-002](../functional/FR-002-canonical-discovery-bundle.md),
[FR-003](../functional/FR-003-package-installable-bundle.md), and
[NFR-001](../non-functional/NFR-001-cross-agent-parity.md).
