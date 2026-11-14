---
id: SR-20261113192944
title: "Gap analysis of EA-109 review gate PR #252"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/engineering-assurance; PR #252 at 07a3d6b; FR-059/061/062/063/065; changed approval-gate implementation"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-059"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-061"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-062"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-063"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-065"
    type: reviews
---

# SR-20261113192944: Gap analysis of EA-109 review gate PR #252

## Summary

This planless, diff-scoped gap analysis used `quoin matrix --repo . --json`
(quoin 0.30.1; Quire 0.45.0 / engine 0.60.0). The first review correctly
found prose `Trace:` comments; this fix replaces them with `#[trace(...)]`
attributes. The matrix still reports 39 criteria untagged, 1 tagged
(FR-065-AC-6), and 3 inspection criteria as `method-without-symbol`. Plan
completion: not assessed. The optional semantic review was not run.

## Verdict

**FAIL** — the marker-syntax defect is fixed, but the matrix still cannot bind
the tests to the criteria without Test Case (`TC-...`) mappings. The residual
mapping decision is pending planner escalation e4.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new `Trace:` doc comments are not Quire trace markers; all in-scope test ACs still compute as untagged despite tests being present. | `src/approval_gate.rs:864-1453`; `tests/approval_gate_cli.rs:76-123`; computed matrix |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | Valid `#[trace("FR-...-AC-...")]` attributes still do not bind most tests in the computed matrix; the tests need Test Case IDs mapped to criteria. | `spec/tests.md:275-283`; `src/approval_gate.rs`; `tests/approval_gate_cli.rs`; computed matrix |

## Coverage

Examined the computed matrix and corresponding changed tests/source for
FR-059-AC-1..9, FR-061-AC-1..12, FR-062-AC-1..5, FR-063-AC-1..6, and
FR-065-AC-1..3 plus AC-6..10. FR-059-AC-10 and FR-065-AC-4/5 are Inspection
criteria and compute as `method-without-symbol`; no missing test tag was
claimed for those inspection-only criteria. Reverse-gap and stub checks over
the changed implementation found no separate unowned behavior or placeholder
implementation. The matrix invocation also emitted a Quire declaration-parser
diagnostic for the new shell script and existing duplicate-symbol warnings;
these do not change the matrix counts.

The 39 untagged rows comprise FR-059-AC-1..9, FR-061-AC-1..12,
FR-062-AC-1..5, FR-063-AC-1..6, FR-065-AC-1..3, and FR-065-AC-7..10.
FR-065-AC-6 is the one tagged row because its test includes TC-526, which is
mapped to that criterion in `spec/tests.md`; FR-065-AC-1..9 also have TC rows,
but the new tests do not bind those TC IDs. FR-059/061/062/063 have no Test
Case mapping rows in `spec/tests.md`.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The prose `Trace:` comments were replaced by Quire `#[trace(...)]` attributes, for example the test at `tests/approval_gate_cli.rs:184`. |
| FND-002 | still-open | The matrix still reports 39 untagged criteria. Only the FR-065-AC-6 test includes a mapped Test Case ID (TC-526); FR-065-AC-1..9 have other TC mappings but the new tests do not reference them, and FR-059/061/062/063 lack TC mapping rows. Planner escalation e4 is pending. |
| FND-001 | fixed | Round 2 confirms the test source still uses valid `#[trace(...)]` attributes; the matrix gap is tracked separately in FND-002. |
| FND-002 | deferred | Planner accepted this 39-criterion gap as a residual for PR #252, authorized merge with no fourth fix round, and directed a separate spec-first PR to add genuine Test Case rows mapped to the existing assertions. |

