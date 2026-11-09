---
id: SR-16308
title: "gap-analysis of dead PGM-01 compatibility and bounded-report removal (engineering-assurance PR 196)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@e0cc38d1; FR-008, FR-009, FR-015, US-005, StR-002, NFR-004; spec/tests.md; tests/semantics_contract.rs; tests/semantics_parity.rs; src/semantics/mod.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-009
    type: references
---

# Gap analysis of the dead PGM-01 compatibility and bounded-report removal (engineering-assurance PR 196)

## Summary

Ticket: PLAT-945. Plan completion: not assessed. Computed with `quoin matrix --json` (quoin 0.29.1) at head e0cc38d1.

Removed trace tags: TC-054/US-005-AC-2, TC-055/US-005-AC-3, TC-064..TC-067/FR-010-AC-1..4, TC-103/FR-015-CON-1 (all with their criteria withdrawn), plus TC-062/FR-009-AC-3, TC-063/FR-009-AC-4, TC-100/FR-015-AC-1, TC-102/FR-015-AC-2 and TC-103/FR-015-AC-3, which keep other binders. No surviving criterion became untagged: the matrix's untagged and method-without-symbol set (FR-008-AC-4, FR-014-AC-3/6, FR-016-AC-4, FR-018, NFR metrics, and others) contains nothing this PR touched. FR-009-AC-1..4 and FR-015-AC-1..4, CON-2, CON-3 all stay tagged.

Tagged is not the same as backed, though. FR-009-AC-4 ("Projection values retain source record and field-path references") was behaviourally asserted by `tc_064_pgm01_mapping_is_traceable_lossy_and_leaves_its_source_unchanged`, which checked every mapped value carried an absolute source path. After the deletion its only binder is `tc_068_published_schemas_still_describe_the_values_this_library_produces`, a positive-only schema-conformance check of the committed canonical fixture. Reviewer mutation probe: replacing the native `validate_source` field-path check (`src/semantics/mod.rs:465`) with `if false` leaves all 13 semantics_contract and semantics_parity tests green. No test reaches `SemanticErrorKind::InvalidSourcePath`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-009-AC-4 / TC-063 lost its only behavioural backing. The remaining binder (tc_068) validates one positive fixture against the published schema and never runs the native reference validator on a missing or relative source path; disabling `validate_source`'s field-path check passes every test. The PR also rewrites the TC-063 row to "Native Rust reference validation", a claim no test exercises. Add an adverse case to the semantic-reference validation tests: a reference whose `source.field_path` is relative (and one with an empty `record_id`) refuses with `InvalidSourcePath` / its typed reason, tagged TC-063 / FR-009-AC-4. | spec/tests.md:321; tests/semantics_contract.rs:438; src/semantics/mod.rs:465 |

## Verdict

No criterion was left untagged and no live code lost a test. One medium gap: FR-009-AC-4's ✅ row is now backed only by a positive schema check, so a regression in native source-path validation would ship green. Not mergeable until FND-001 has an adverse test.
