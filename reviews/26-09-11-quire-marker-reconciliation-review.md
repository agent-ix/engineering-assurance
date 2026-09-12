---
id: SR-120
title: "Review of the Quire Rust marker reconciliation test"
type: SpecReview
analysis: code-review
scope: "NFR-005-AC-3, TC-118; first-party Rust criterion marker reconciliation"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/NFR-005"
    type: reviews
---

## Summary

TC-118 was declared `🚧 pending implementation` and no test carried the tag, so
NFR-005-AC-3 had no evidence. It is now backed by an integration test that runs
the real Quire reconciliation over this repository.

The test distinguishes three failure classes the criterion names:

- **Orphaned** — a first-party Rust marker naming a target the matrix never
  minted, read from `untracked_symbols`.
- **Missing** — a tag Quire found but could not bind to a test symbol, read from
  `unmatched_tags`. This is the EC-016 case: a marker that compiles but carries
  no evidence.
- **Duplicate** — one symbol claiming the same trace id more than once. A trace
  id shared across *distinct* tests is legitimate evidence and is deliberately
  not treated as duplication; 52 shared ids touch first-party code and every one
  of them is a requirement backed by more than one test.

Markers inside a declared submodule are excluded on the same `.gitmodules` rule
SR-119 established for the traceability gate, because the qa-corpus detection
fixtures exist precisely to carry ids that bind to nothing.

The test refuses an empty population: if Quire binds no Rust markers at all,
every assertion above would pass vacuously, so a zero census fails explicitly.
It invokes the real `quire` rather than a fixture, because a marker form Quire
cannot parse is exactly the failure the criterion guards — a fixture would
assert against this repository's idea of a marker, not Quire's.

## Verdict

**PASS** — measured on this repository: 0 orphaned first-party Rust markers, 0
unbound first-party Rust tags, 0 duplicate symbol bindings, 321 Rust markers
bound. Injecting a deliberately orphaned marker fails the test, so the guard is
real rather than vacuous.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-224 | medium | Closed. NFR-005-AC-3 carried no evidence: TC-118 was pending and no Rust marker named it, so nothing verified that Quire could bind the markers the rest of the matrix depends on. | `tests/traceability_reconciliation.rs:95`; `spec/tests.md:297` | correct-requirement-no-evidence |

## Scope limits

Covers first-party Rust markers. Python and TypeScript markers in this
repository are not reconciled here, and no claim is made about the six test
cases that remain unbacked.

## Gate results

| Gate | Result |
| --- | --- |
| `cargo +1.98.1 test --test traceability_reconciliation` | pass |
| TC-118 with an injected orphaned marker | fails on the orphan (guard verified red) |
| `quire coverage` before | 247/270 backed, 21 unbacked rows |
| `quire coverage` after | 249/270 backed, 19 unbacked rows |
