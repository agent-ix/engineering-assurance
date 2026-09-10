---
id: SR-062
title: "Base review of the semantic refusal interface clarification"
type: SpecReview
analysis: base
scope: "FR-015-AC-1, FR-015-AC-3, TC-100, and TC-103"
review_set: base
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---

# SR-062: Base review of the semantic refusal interface clarification

## Summary

This corrective base review covers the narrow FR-015 clarification authored
after implementation was drafted and before merge. The reviewed requirement
preserves the existing top-level error category, requires an exhaustive typed
refusal reason instead of diagnostic-prose matching, limits byte-parity claims
to the shared supported input domain, and records the deliberate behavior for
structured historical identities.

The identifier, relationship, terminology, and acceptance-criterion shapes are
valid. FR-015-AC-1 and FR-015-AC-3 remain mapped to TC-100 and TC-103. Options
and state transitions do not apply to this additive read-only interface slice;
the relevant happy, error, and edge cases are the valid reference/view
baseline, distinct semantic and generated-view refusal families, scalar parity,
and JSON array/object identity divergences.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No blocking base-review finding remains: the shared parity domain, deliberate structured-identity divergences, stable top-level category, typed refusal reasons, and diagnostic-only prose boundary are explicit and mapped to traced tests. | FR-015-AC-1; FR-015-AC-3; TC-100; TC-103 |
