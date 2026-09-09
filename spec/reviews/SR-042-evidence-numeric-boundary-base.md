---
id: SR-042
title: "Base review of the evidence identity and public-API boundary"
type: SpecReview
analysis: base
scope: "FR-015, TC-100, TC-102, TC-103"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---

## Summary

The amended evidence-classification specification closes the parser-before-encoder gap identified by independent review: arbitrary-size integers remain exact, while non-finite or retained-domain-overflow numeric inputs fail without an identity digest. It also replaces the retained substring/case-fold accident with an explicit printable-ASCII exact-version policy that distinguishes true wildcard components from immutable metadata. The test matrix distinguishes this backed slice from the still-pending aggregate port and assigns the public Rust inspection surface to FR-015.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-095 | low | No blocking base-review issue remains in the amended scope: identifiers are unique, the happy/error/edge numeric, version-token, and exact-one state-label boundaries are measurable, and each changed criterion retains a TC mapping without claiming aggregate completion. | FR-015; TC-100; TC-102; TC-103 |

## Six-rule coverage

- **Coverage:** FR-015-AC-1 through AC-3 remain mapped to TC-100, TC-102, and TC-103.
- **Option/permutation:** accepted-corpus objects, deterministic generated JSON values, typed states, untyped label selections, immutable metadata, wildcard components, and invalid version characters define the affected variants.
- **Constraint boundary:** integers cross both `i64` and `u64`; non-finite literals and finite-number syntax that overflows the retained Python domain are refused before digest creation; `x` is refused only as a wildcard component rather than as incidental immutable metadata.
- **Error path:** malformed numeric input, missing provenance, invalid labels, and other existing validation failures return explicit invalid results or fail at the wire boundary.
- **State transition:** this pure classifier has no persisted transition; observed/unavailable/not-computed/not-applicable remain distinct values.
- **Edge case:** large signed/unsigned integers, Unicode/nesting/key order, float formatting thresholds, negative zero, subnormal values, maximum finite float, non-finite overflow, non-ASCII/control version tokens, and `+linux-x86_64` metadata are included in the affected TC scope.

## Review disposition

This is the owner-selected base subset for the amended specification, not an implementation acceptance. Independent code/gap findings SR-040 and SR-041 remain the implementation merge gate until the branch is repaired, qualified, and re-reviewed.
