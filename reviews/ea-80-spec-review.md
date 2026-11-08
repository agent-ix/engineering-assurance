---
id: SR-16304
title: "spec-review of FR-030, FR-031, IT-007, US-006, TC-219..231 and PLAN-004 (engineering-assurance PR 193)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@ea-80-integrator-package; spec/functional/FR-030-integrator-package-contract.md; spec/functional/FR-031-integrator-package-publication.md; spec/integration/IT-007-integrator-package-consumer.md; spec/usecase/US-006-hand-off-source-facts-to-an-integrator.md; spec/tests.md TC-219..TC-231; plan/PLAN-004-integrator-package"
review_set: base
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-030
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-031
    type: references
---

# Spec review of the renumbered EA-80 bundle (engineering-assurance PR 193)

## Summary

Ticket: EA-80. After EA-79 merged and claimed FR-029, TC-214..218 and PLAN-003, the bundle was renumbered: contract FR-029 to FR-030, publication FR-030 to FR-031, TC-214..226 to TC-219..231, PLAN-003 to PLAN-004, TASK-012/013 to TASK-014/015. IT-007 and US-006 did not clash. No active AssuranceProfile applies to this repository change; the base set was chosen. Every reference in spec, plan, index, tests, Cargo feature comment and trace tags was updated. `quoin matrix` shows every FR-030 and FR-031 criterion tagged. No hash, digest or pin is introduced (FR-030 forbids a digest; PLAN-004 states it). Two findings were fixed.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-030-CON-1 was mapped to TC-220 in the coverage table but its trace tag was missing, so the matrix showed it untagged. Tag added. | tests/integrator_package.rs | correct-requirement-no-evidence |
| FND-002 | medium | FR-031 left the creating writer's outcome open against TL-19 (conflict, not a timing-selected winner). Added the reason a lock or claim step cannot fix it and the consumer rule: take the verdict from the strict read after all known writers return. | spec/functional/FR-031-integrator-package-publication.md | wrong-requirement |
| FND-003 | low | Two bullets tripped EARS warnings (compound subject line, agentless passive). Reworded. | spec/functional/FR-030-integrator-package-contract.md | wrong-requirement |

## Verdict

Mergeable on the spec side after the fixes.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Trace tag added to TC-220. |
| FND-002 | fixed | Documented in FR-031 and the store module docs; writer-side determinism is infeasible without a crash-prone lock. |
| FND-003 | fixed | Bullets reworded. |
