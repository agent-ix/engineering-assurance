---
id: SR-134
title: "Base review of the Python-port acceptance-criterion alignment"
type: SpecReview
analysis: base
scope: "FR-001-AC-9; FR-007-AC-4; FR-016-AC-5; FR-024-AC-11; FR-024-AC-12; NFR-005-AC-5; TM-001 rows TC-043, TC-115, TC-201..TC-205"
review_set: base
---

## Summary

This review covers the requirement edits made while aligning the ported
Python suite (`tests/python_port/`) to acceptance criteria. Five behaviours the
suite exercised had no owning criterion: the onboarding report's `--summary`
mode, the exact pilot workflow inventory, the per-binding fail-closed rules of
the canonical workflow invariants, the MeasurementPlan `ground_truth_kind`
and `preregistration` members, and the Test Matrix's own row integrity. Each
now has a criterion, a behavior clause where it adds behavior, and
one Test Matrix row backed by a traced test. Identifier formats are sequential
and unique (AC numbers continue each table; TC-201..TC-205 follow TC-200), every
new criterion maps to exactly one test case, and every new test case traces back
to its criterion. No existing criterion was removed or weakened.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Resolved: FR-001 Outputs and FR-024 Inputs and Error Conditions did not mention the summary report, `ground_truth_kind`, or `preregistration` that the new criteria test. Each section now names them, so the criteria do not introduce undeclared inputs, outputs, or error modes. | FR-001-AC-9; FR-024-AC-11; FR-024-AC-12 |
| FND-002 | medium | Resolved: the first FR-016 behavior clause for the fail-closed bindings was an agentless passive (`SHALL be owned`), which allocates the obligation to nobody. It now names the Rust evaluator as the subject; `quire validate` reports no grammar finding on any edited document. | FR-016-AC-5 |
| FND-003 | low | FR-007-AC-4 restates FR-007-CON-1 as a testable criterion so TC-043 can carry an acceptance id. The two are deliberately aligned; a later change to either must change both. | FR-007-AC-4; FR-007-CON-1; TC-043 |
| FND-004 | low | `ground_truth_kind` and `preregistration` sit in FR-024 because both are measurement-integrity declarations of the same kind as the protected apparatus (FR-021 explicitly disclaims `preregistration`). FR-024's title still names only the protected apparatus; renaming it is left to the requirement owner. | FR-024-AC-11; FR-024-AC-12 |
| FND-005 | low | FR-016-AC-5 checks the Rust evaluator against the retained JavaScript provider on every case. When FR-018 removes that provider, the parity half of TC-201 goes with it and the explicit expected codes remain the oracle. | FR-016-AC-5; TC-201; FR-018 |
| FND-006 | medium | Resolved: the matrix row-integrity test was traced to FR-021-AC-13 (decision-rule interval level), which it does not exercise, and it carried an allowlist of two "dangling" ids that were in fact present rows its own pattern missed because of cell padding. It now traces to the new NFR-005-AC-5, matches padded rows, has no allowlist, and proves it can fail on a synthetic dangling reference. | NFR-005-AC-5; TC-205; FR-021-AC-13 |
| FND-007 | low | Resolved: TC-115 is now bound by a real test of its Python slice, so its coverage and constraint rows state that only that slice is backed and the JavaScript providers keep the aggregate audit pending, rather than reading as either unbacked or complete. | TC-115; FR-018-AC-4; NFR-005-AC-4 |
