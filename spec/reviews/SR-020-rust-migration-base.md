---
id: SR-020
title: "Base review of the Rust-native Engineering Assurance migration (#59)"
type: SpecReview
analysis: base
scope: "StR-003, ADR-002, FR-014..FR-018, NFR-005, TC-096..TC-127"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: reviews
---

## Summary

The candidate places the migration in the existing Engineering Assurance
repository, preserves external ownership, and maps every stated criterion to a
pending TC. It is not ready for acceptance: the machine-readable dependency
graph omits prerequisites stated in prose, and the six-rule test design covers
the older onboarding slice rather than the new Rust migration.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-050 | high | FR-015 through FR-018 state FR-014..FR-017 prerequisites in their Dependencies sections, but their frontmatter omits several corresponding `requires` edges. A graph consumer can therefore schedule FR-016/FR-017 before the shared types and FR-018 before onboarding parity. | FR-015:5-13,106-110; FR-016:5-11,69-72; FR-017:5-11,72-75; FR-018:5-11,80-84 | missing-requirement |
| FND-051 | high | TC-096..TC-120 give AC rows, but the option-permutation, constraint-boundary, state-transition, and integration-detail matrices do not enumerate the Rust protocol, registry, host, or migration variants. Broad rows such as TC-099, TC-105, TC-110, and TC-119 can pass while one named failure class is absent. | spec/tests.md:261-285,287-356,391-394 | missing-requirement |

## Coverage

- StR-003 has four validation criteria; FR-014..FR-018 and NFR-005 have 26
  acceptance criteria. Every criterion names at least one of TC-096..TC-120.
- Mapping completeness is present; permutation, boundary, error, and transition
  completeness are not (FND-051).
- The reviewed six requirement documents are 6/6 grammar-clean under Quire
  0.31.0; the repository is 55/55 grammar-clean.

## Repeat-review dispositions

| ID | Disposition |
| --- | --- |
| FND-050 | Fixed: FR-015..FR-018 now carry the missing typed `requires` edges, matching the stated ADR order. |
| FND-051 | Fixed: TC-121..TC-127 plus Rust-specific option, boundary, transition, edge, and integration rows enumerate the previously implicit cases. |
