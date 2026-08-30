---
id: TASK-007
title: "Close the onboarding integration gate"
type: Task
status: not_started
track: Gate
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-001"
    type: part_of
  - target: "ix://agent-ix/engineering-assurance/TASK-001"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/TASK-002"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/TASK-003"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/TASK-004"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/TASK-005"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/TASK-006"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/StR-001"
    type: references
  - target: "ix://agent-ix/engineering-assurance/NFR-001"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-001"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-038"
    type: verifies
---

# TASK-007: Close the onboarding integration gate

## Scope

Execute the complete onboarding slice through real install, discovery, validation,
evidence, workflow, resume, and decision boundaries; then close traceability for #3.

## TDD Work

- Implement TC-001..TC-003 and TC-038 as the top-level E2E/parity assertions.
- Execute IT-001..IT-004 from isolated local-source and repository-source installs.
- Compare canonical content digests across all four hosts and both sources.
- Run content-rights, Ruff, pytest, manifest validation, package audit, Quire
  validation, all required evals, and gap analysis.

## Exit Criteria

- Inventory precedes every proposal, no unjustified profile appears, and a named
  human owns every terminal outcome.
- Cross-agent canonical parity meets every declared threshold.
- Gap analysis reports TC-001..TC-049 backed and every plan task complete.
