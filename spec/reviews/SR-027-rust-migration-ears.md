---
id: SR-027
title: "EARS conformance review of the Rust-native Engineering Assurance migration"
type: SpecReview
analysis: ears-conformance
scope: "FR-014..FR-018, NFR-005"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/NFR-005"
    type: reviews
---

## Summary

Quire 0.31.0 reports the six target documents 6/6 grammar-clean with zero EARS
or quality findings. Human review still finds criteria whose many independent
outcomes are too broad to serve as one generated property without enumerated
subcases.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-062 | medium | Only 10/26 target criteria are property-extractable. FR-014-AC-3, FR-015-AC-2/AC-3, FR-016-AC-1/AC-3, and FR-017-AC-2 group independent triggers and outcomes into one criterion; the prose is grammatical, but the test oracle is not singular unless each named case is enumerated in the matrix. | FR-014-AC-3; FR-015-AC-2..AC-3; FR-016-AC-1,AC-3; FR-017-AC-2; Quire summary | missing-requirement |

## Pattern result

The requirements use named subjects and explicit SHALL obligations. The fix is
test decomposition and bounded case enumeration, not a cosmetic rewrite that
would weaken the closed failure vocabulary.

## Repeat-review disposition

| ID | Disposition |
| --- | --- |
| FND-062 | Fixed at the test-oracle layer: the Rust permutation, boundary, transition, integration, and edge matrices enumerate every grouped trigger while preserving the closed criterion vocabulary. The amended target is revalidated below; no cosmetic criterion split is required. |
