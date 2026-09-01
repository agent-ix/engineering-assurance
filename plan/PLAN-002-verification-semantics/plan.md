---
id: PLAN-002
title: "Verification semantics and compatibility implementation"
type: Plan
status: active
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-002"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-008"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-009"
    type: references
  - target: "ix://agent-ix/engineering-assurance/FR-010"
    type: references
---

# PLAN-002: Verification semantics and compatibility implementation

## Outcome

Complete Engineering Assurance #5 with a reviewed semantic ownership contract,
package-visible schemas, deterministic cross-language fixtures, bounded report
projections, and a read-only PGM-01 compatibility mapping that reuses Quire,
Quoin, and ix-flow record families.

## Scope Boundaries

- Add no producer execution, subprocess runner, generic stdout verdict scraper,
  evidence persistence, audit replacement, or terminal decision logic.
- Treat Quire, Quoin, native producer, and ix-flow records as authoritative;
  local schemas validate references and projections rather than copying those
  records into a new envelope.
- Do not release an enforcing migration schema until the real-producer fixture
  gate in Engineering Assurance #9 is accepted.
- Preserve hosted workflows as manual-only.

## Dependency Graph

```text
TASK-008 ownership registry
  └── TASK-009 reference/report contracts
        └── TASK-010 read-only compatibility and projections
              └── TASK-011 fixtures, generated languages, tests, and review gate
```

## Test Strategy

- Schema and property tests validate unique ownership, distinct identities,
  required references, producer tuples, version refusal, report shape, and all
  non-success states.
- Mutation-style negative fixtures remove or alter each required premise.
- PGM-01 fixture tests hash every input before and after mapping.
- Rust, TypeScript, and Python fixtures are deterministic projections of one
  canonical JSON fixture and compare semantically.
- Static tests inspect the package for execution, persistence, generic scraping,
  duplicate record discriminators, and hosted workflow trigger drift.

## Task File Mapping

| Task | Scope | Owns |
| --- | --- | --- |
| TASK-008 | Ownership/type-fit registry and package inventory | FR-008; TC-053, TC-056..TC-059 |
| TASK-009 | Reference and bounded-report JSON schemas | FR-008; FR-009; FR-010; TC-054, TC-057..TC-063, TC-067..TC-068 |
| TASK-010 | Read-only PGM-01 mapping and JSON/Markdown renderers | FR-010; TC-055, TC-064..TC-067 |
| TASK-011 | Canonical, negative, legacy, and generated-language fixtures; final review | StR-002; IT-005; TC-052..TC-068 |

## Completion Gates

- Every TC-052..TC-068 row has a real tracking tag and passing evidence.
- The complete local lint, test, package-audit, document-validation, and
  integration-traceability gates pass.
- Code review and gap analysis are retained with no unresolved required fix.
- GitHub review feedback is read and resolved before merge.
- The issue remains non-enforcing until Engineering Assurance #9 accepts real
  producer compatibility fixtures.

