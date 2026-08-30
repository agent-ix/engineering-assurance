---
id: TASK-003
title: "Package and install the onboarding bundle"
type: Task
status: todo
track: C
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-001"
    type: part_of
  - target: "ix://agent-ix/engineering-assurance/TASK-002"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/FR-003"
    type: references
  - target: "ix://agent-ix/engineering-assurance/NFR-003"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-014"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-040"
    type: verifies
---

# TASK-003: Package and install the onboarding bundle

## Scope

Extend Python-wheel and private-npm payloads, allowlist audits, and install docs so
module/plugin and local/repository paths preserve the canonical bundle and existing
module root.

## TDD Work

- Write TC-014..TC-019 and TC-040 before changing package manifests or audits.
- Reject unexpected, missing, binary, rights-violating, or root-escaping members.
- Repair manifest-schema discovery to use the configured/installed active module
  contract rather than assuming a stale adjacent source-tree layout.
- Exercise real local-source and repository-source installations in isolated roots.

## Exit Criteria

- Wheel and npm archives contain the exact declared payload and remain private.
- Quire and host discovery work after both installation sources.
- Module/plugin installation instructions are distinct from source-mode instructions.

