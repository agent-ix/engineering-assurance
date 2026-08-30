---
id: PLAN-001
title: "Assurance onboarding implementation"
type: Plan
status: active
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-001"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-001"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-002"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-003"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-004"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-005"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-006"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-007"
    type: references
---

# PLAN-001: Assurance onboarding implementation

## Outcome

Deliver Engineering Assurance #3: one installable canonical onboarding bundle that
inventories an existing repository before proposing work, exposes the same content
to four coding-agent hosts, preserves explicit evidence provenance and availability,
delegates resumable human decisions to ix-flow, and proves the behavior with a
28-cell real-agent evaluation matrix.

## Scope Boundaries

- Engineering Assurance owns canonical onboarding content, thin discovery
  manifests, package membership, bounded inventory/proposal logic, workflow
  definitions, compatibility paths, and its evaluation harness.
- Quire owns artifact validation, Quoin owns evidence semantics/storage/reporting,
  and ix-flow owns run state and human gates; this repository integrates rather
  than reimplements those boundaries.
- No terminal decision, assurance policy, evidence result, or unjustified artifact
  may be invented by the onboarding bundle.
- Engineering Assurance #4 remains a later retrospective because its Filament
  baseline/pilot evidence is not yet complete.

## Dependency Graph

```text
TASK-001 bounded onboarding core ──┬── TASK-004 evidence/provenance
                                  └── TASK-005 resumable decisions
TASK-002 canonical bundle ──┬── TASK-003 packaging/install
                            └── TASK-005 resumable decisions
TASK-002 + TASK-004 + TASK-005
  └── TASK-006 four-host evaluation suite
TASK-001..TASK-006
  └── TASK-007 cross-source E2E and release gate
```

TASK-001 and TASK-002 may start in parallel. TASK-003 and TASK-004 may then proceed
in parallel; TASK-005 joins the onboarding and workflow contracts before eval work.

## Test Strategy

- Unit/property/static tests lead inventory separation, path confinement,
  manifest thinness, exact host/workflow sets, evidence-state exclusivity, terminal
  transitions, and package allowlists.
- Integration tests use real local/repository installs, package archives, Quire,
  Quoin, and ix-flow boundaries without redefining their contracts.
- Agent evaluations execute seven variants on Claude Code, Codex, opencode, and
  GitHub Copilot, retaining the full immutable version tuple, transcript digest,
  effort, commands, decisions, and outcomes.
- Existing package/content-rights gates remain mandatory; the manifest validator's
  stale sibling-schema lookup is repaired as part of the installation contract.

## Task File Mapping

| Task | Track | Scope | Owns |
| --- | --- | --- | --- |
| TASK-001 | A | Bounded inventory, proposal, and atomic artifact publication | FR-001; TC-004..TC-008; TC-044..TC-045 |
| TASK-002 | B | Canonical skill/workflows, thin manifests, pilot compatibility | FR-002; FR-007; TC-009..TC-013; TC-035..TC-037; TC-041..TC-043 |
| TASK-003 | C | Wheel/npm membership, source installs, documentation | FR-003; NFR-003; TC-014..TC-019; TC-040 |
| TASK-004 | D | Evidence availability, provenance, and Quoin delegation | FR-004; TC-020..TC-025; TC-046 |
| TASK-005 | E | ix-flow interruption, binding, acceptance/rejection gates | FR-005; TC-026..TC-030; TC-047..TC-048 |
| TASK-006 | F | Four-host agent-evaluation harness and aggregate gate | FR-006; NFR-002; TC-031..TC-034; TC-039; TC-049 |
| TASK-007 | Gate | Cross-source/host E2E and final traceability gate | StR-001; NFR-001; TC-001..TC-003; TC-038; IT-001..IT-004 |

## Completion Gates

- Every TC-001..TC-049 row carries a real tracking tag and passing evidence.
- All 28 host-scenario evaluation cells have complete, version-bound envelopes.
- `quire validate --scope . "spec/**/*.md" "plan/**/*.md"` passes.
- Content-rights, Ruff, pytest, manifest validation, package audit, and any declared
  evaluation commands pass from a clean installable tree.
- Gap analysis reports every task complete and no unowned requirement or code path.

