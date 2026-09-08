---
id: SR-025
title: "Risk and complexity review of the Rust-native Engineering Assurance migration"
type: SpecReview
analysis: risk-complexity
scope: "ADR-002, FR-014..FR-018, NFR-005"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/ADR-002"
    type: reviews
---

## Summary

Additive parity and per-capability rollback reduce migration risk, but the
candidate has no resource/performance regression gate and compresses a large
capability inventory into broad tests that can mask which slice regressed.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-059 | medium | The Rust migration constrains MSRV, unsafe code, traceability, semantics, and byte identity but gives no latency, throughput, memory, or subprocess resource baseline. A semantically equal Rust path could make onboarding or the 28-cell evaluation gate operationally unusable. | NFR-005:25-54; ADR-002:127-136 | missing-requirement |
| FND-060 | medium | TC-100 and TC-111 each aggregate multiple independent capabilities and corpora. Without per-capability subcases and named populations, one passing aggregate can conceal an omitted classifier, serializer, audit, or refusal path. | ADR-002:57-72; spec/tests.md:265,276 | missing-requirement |

## Risk disposition

The migration order is sound. Planning must split implementation by the ADR
capability rows and retain old/new evidence at the same revision; no big-bang
removal is acceptable.

## Repeat-review dispositions

| ID | Disposition |
| --- | --- |
| FND-059 | Fixed: NFR-005-AC-5/TC-126 require 30-run same-runner p95 latency and peak-RSS comparisons for five named capability classes, with a 10% ceiling. |
| FND-060 | Fixed: TC-100 and TC-111 now require independently reported per-capability subcases, and the Rust Migration Permutation Matrix lists those populations. |
