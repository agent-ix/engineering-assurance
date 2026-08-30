---
id: TASK-002
title: "Promote the canonical onboarding bundle"
type: Task
status: done
track: B
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-001"
    type: part_of
  - target: "ix://agent-ix/engineering-assurance/FR-002"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-007"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-009"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-043"
    type: verifies
---

# TASK-002: Promote the canonical onboarding bundle

## Scope

Create one repository-owned `assurance-onboarding` skill, promote the four pilot
workflow definitions to canonical paths, expose them through thin manifests for
all four supported hosts, and retain the exact pilot compatibility surface.

## TDD Work

- Write TC-009..TC-013, TC-035..TC-037, and TC-041..TC-043 first.
- Make canonical targets root-confined, existing, unique, and structurally audited.
- Keep host manifests to discovery metadata and one canonical target; reject copied
  behavioral text or workflow definitions.
- Route the four old pilot names to byte/semantic-equivalent canonical workflows.

## Exit Criteria

- Every supported host resolves the same skill and exactly four workflows.
- Missing, extra, duplicate, or escaping host/workflow entries fail closed.
- Compatibility invocations still load while documentation leads with canonical use.
