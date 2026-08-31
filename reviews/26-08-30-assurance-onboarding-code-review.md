---
id: SR-009
title: "Code review — assurance onboarding integration gate"
type: SpecReview
analysis: code-review
scope: "PLAN-001 TASK-001..TASK-007; engineering_assurance/; scripts/; tests/; Makefile"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-001"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/TM-001"
    type: references
---

# SR-009: Code review — assurance onboarding integration gate

## Summary

Reviewed the complete Engineering Assurance #3 implementation and the TASK-007
gate added after evaluated source revision
`ea1ed8a8ad3de2f8d6b5b36fd8131947970857ac`. The review covered production and
gate code, all tests, package/install boundaries, retained evaluation loading,
workflow state, path confinement, specification traceability, and the full
working-tree diff. No unresolved high- or medium-severity implementation defect
was found.

The review found two fail-open paths and resolved both before this verdict. Tests
that require Quire or ix-flow now fail when the executable is absent instead of
skipping, and retained report paths are confined after symlink resolution before
their bytes are read.

## Verdict

**PASS** — the TASK-007 implementation has no unresolved blocking code-review
finding. The two low findings below are visible compatibility/style limitations;
neither weakens the completed integration, traceability, package, or evaluation
gate.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | low | Legacy test modules use the repository's compact top-level pytest convention instead of the class and expanded-docstring convention; new integration-evidence tests use the expanded convention, and no empty, skipped, mock-only, import-only, or assertion-free test was found. | `tests/` |
| FND-002 | low | The installed TestMatrix traceability declaration asks for `Status` while structural validation requires `Coverage Status`; the gate preserves the schema-owned header, requires 101/101 backing, and independently rejects every non-passing marker while leaving Quire's diagnostic visible. | `spec/tests.md`; `scripts/check_integration_evidence.py` |

## Resolved During Review

| Severity | Finding | Resolution |
|----------|---------|------------|
| medium | Required Quire and ix-flow tests could skip when executables were absent, allowing an incomplete environment to appear green. | The helpers now call `pytest.fail`; the 120-test gate requires the real tools. |
| medium | A repository-relative retained-report path could traverse a symlink outside the selected root before digest verification. | The verifier resolves and confines every report path before reading it and has a negative symlink-escape test. |

## Test and Mock Review

- The suite contains no `pytest.skip`, skip marker, `unittest.mock`, patch
  decorator, internal-logic mock, pass-through test, or database access.
- New verifier tests exercise complete and incomplete coverage, the known
  schema/traceability status mismatch, partial aggregate metadata, governing-file
  drift, and report-path symlink escape. The real gate then exercises report and
  transcript bytes plus current external executable identities without mocks.
- Weak-looking type and non-null assertions in legacy tests are followed by
  behavioral field assertions or protect dynamic import setup; none is the sole
  oracle for a claimed acceptance criterion.

## Completeness and Alignment

- No production or gate file contains a TODO, FIXME, XXX marker, concrete `pass`
  body, placeholder result, protocol-only implementation, or re-export-only
  implementation. The eight-line package `__init__.py` is the only small source
  file and is intentionally excluded by the stub rule.
- TASK-007 owns the integration-evidence script and Make targets. NFR-001 owns
  cross-source/host parity, NFR-002 owns non-inventing 28-cell outcomes, and
  NFR-003 owns package member stability. TC-001..TC-049 all bind to real evidence
  symbols.
- Current governing module, plugin, skill, workflow, result-contract, Quire,
  Quoin, ix-flow, and cli-evals identities match the retained reports. The
  complete-only aggregate remains 28/28 and preserves its two failed attempts
  alongside same-source passing retries.

## Executed Gate

`make integration-gate` reached 120/120 passing pytest cases, clean Ruff and
content-rights checks, module validation, real wheel/npm archive audit, Quire
document validation, 101/101 traceability backing including TC-001..TC-049, and
28/28 retained agent-evaluation cells. Quire's duplicate-provider and TestMatrix
status-column diagnostics remain printed rather than suppressed.

## Semantic Review

Not run here. The optional intent-to-test-to-code semantic expansion belongs to
the subsequent gap-analysis decision.
