---
id: SR-010
title: "Gap analysis — PLAN-001 assurance onboarding"
type: SpecReview
analysis: gap-analysis
scope: "plan/PLAN-001-assurance-onboarding/; spec/tests.md; engineering_assurance/; scripts/; tests/"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-001"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/TM-001"
    type: references
---

# SR-010: Gap analysis — PLAN-001 assurance onboarding

## Summary

Audited all seven PLAN-001 tasks, the complete Engineering Assurance onboarding
specification and Test Matrix, the production and gate surfaces, real package and
workflow integrations, and the retained four-host evaluation evidence. Every
task is done, Quire reconciles every target row to real evidence, and every added
behavior has an owning requirement or integration-gate task.

## Verdict

**PASS** — PLAN-001 has no incomplete task, all 101 target rows are backed,
TC-001..TC-049 are passing, no unowned implementation or stub was found, and the
complete-only real-agent aggregate is 28/28.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | low | No target gaps found | - |

## Plan Completion

| Task | Result | Owning evidence |
|------|--------|-----------------|
| TASK-001 | complete | Inventory-first onboarding, safe reuse/no-work behavior, staged Quire validation, and TC-004..TC-008/TC-044..TC-045. |
| TASK-002 | complete | One canonical skill/workflow tree, four thin host manifests, compatibility inventory, and TC-009..TC-013/TC-035..TC-037/TC-041..TC-043. |
| TASK-003 | complete | Exact wheel/npm allowlists, real offline archive installs, documentation separation, and TC-014..TC-019/TC-040. |
| TASK-004 | complete | Exclusive evidence states, immutable provenance, Quoin delegation, and TC-020..TC-025/TC-046. |
| TASK-005 | complete | Real ix-flow resume, binding, waiting, acceptance/rejection, and TC-026..TC-030/TC-047..TC-048. |
| TASK-006 | complete | Four real hosts by seven scenarios, strict retained envelopes, retry-preserving aggregation, and TC-031..TC-034/TC-039/TC-049. |
| TASK-007 | complete | One fail-closed integration target, cross-source parity, document/traceability gates, SR-009, and this review. |

All seven task documents are `status: done`. The plan bundle contains no stale
completion checkbox that contradicts those task states.

## Coverage

- Reconciliation: `quire coverage` with Quire 0.31.0, CLI revision `4f6ed024`,
  engine 0.46.0 revision `ca7362d4`.
- Tasks done: 7 / 7.
- Rows backed by a tagged evidence symbol: 101 / 101.
- Requirement criteria backed: 52 / 52 — forty FR criteria, nine NFR criteria,
  and three stakeholder validation criteria.
- Test Matrix rows backed: 49 / 49 (TC-001..TC-049).
- Unbacked rows: 0; status lies: 0; untracked symbols: 0.
- Retained real-agent cells: 28 / 28 across Claude Code, Codex, opencode, and
  GitHub Copilot.
- Untraced behaviors / stubs: 0.
- Semantic review: skipped; the optional intent↔test↔code expansion was not
  selected.

## Underspecified-Code Trace

- `onboarding.py` implements FR-001's inventory, reuse, no-applicable-work,
  human-selection, validation, confinement, and atomic-publication behavior.
- `discovery.py` and the canonical bundle implement FR-002 and FR-007's exact
  host/workflow sets, thin manifests, and pilot equivalence.
- `audit_packages.py`, package manifests, and installation documentation
  implement FR-003 and NFR-003's archive membership and real-install contract.
- `evidence.py` implements FR-004's availability and immutable-provenance states.
- `workflow.py`, canonical definitions, and invariants implement FR-005's ix-flow
  binding, resume, fail-closed gates, and attributed terminal choices.
- `evaluation.py`, `eval_reports.py`, the suite contract, and runner scripts
  implement FR-006 and NFR-002's 28-cell real-host evidence contract.
- `check_integration_evidence.py`, `Makefile`, and the integration tests implement
  TASK-007 and NFR-001's final cross-source, traceability, retained-byte, and
  governing-identity gate.

The public-function/class census and the full branch diff found no behavior
outside those owners. No source or test contains a TODO/FIXME/XXX marker,
concrete `pass` body, placeholder implementation, unowned protocol, skipped
test, or internal-logic mock.

## Retained Evidence

The evaluated implementation source is
`ea1ed8a8ad3de2f8d6b5b36fd8131947970857ac`. The verified aggregate
`evals/reports/aggregate-ea1ed8a.json` has SHA-256
`401e4353078820a58e689414b1f120302b55a599d91b620ef98f32adc8bd0b9e`.
Its verifier rereads all six report files and retained transcripts, recomputes
the 28 cells, preserves the opencode EA-005 and Copilot EA-006 failed attempts,
and confirms that current module, plugin, skill, workflow, schema, Quire, Quoin,
ix-flow, and cli-evals identities still match the evaluated inputs.

The final pre-review integration execution passed 120 pytest cases, Ruff,
content rights, module validation, real wheel/npm package auditing, Quire
validation, 101/101 traceability, and the retained 28/28 aggregate.

## Visible Diagnostics and Limits

- The installed TestMatrix structural schema requires `Coverage Status`, while
  its traceability declaration requests `Status`. Structural validation owns the
  authored header; the integration verifier compensates by requiring complete
  backing and rejecting every non-passing marker. The Quire diagnostic remains
  printed and is not suppressed.
- Quire reports duplicate providers for locally selected and installed module
  copies, plus optional absent `Inspections` and `SuiteRegistry` archetypes.
  These diagnostics do not change the selected first-wins definitions or the
  101-row target population and remain visible in every gate run.
- The semantic review expansion was not run. This PASS covers plan state,
  executable traceability, reverse ownership, stubs, real integration behavior,
  and retained agent evidence; it does not claim a separate LLM judgement for
  every intent↔test↔code triple.
