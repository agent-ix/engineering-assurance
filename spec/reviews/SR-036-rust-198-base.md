---
id: SR-036
title: "Base review of the Rust 1.98.1 qualification baseline"
type: SpecReview
analysis: base
scope: "NFR-005, ADR-002, TM-001 TC-116/TC-127/TC-128"
review_set: base
relationships:
  - target: ix://agent-ix/engineering-assurance/NFR-005
    type: reviews
---
# SR-036: Base review of the Rust 1.98.1 qualification baseline

## Summary

This review applies the owner-selected base checklist to the correction from an
inherited Rust 1.75 minimum to exact Rust 1.98.1 as the initial supported and
qualification compiler. It reviews requirement quality, identifiers, links,
and the applicable coverage rules. No optional analysis lens was selected.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
|---|---|---|---|---|
| FND-071 | high | **Closed:** Rust 1.75 was copied into the migration specification without evidence that it was suitable for a new package. NFR-005 now selects exact current stable 1.98.1 and requires the real target/tool matrix. | NFR-005 measurement table; NFR-005-AC-1; TC-116 | wrong-requirement |
| FND-072 | medium | **Closed:** the `ea-rust-msrv` status name encoded the discarded assumption that qualification was an old minimum-version exercise. It is renamed `ea-rust-toolchain` and remains bound to the current pull-request head. | NFR-005-AC-6; TC-127 | wrong-requirement |
| FND-073 | high | **Closed:** the first requirement had no rule for evaluating a newer stable compiler, allowing the stale pin to persist indefinitely. NFR-005-AC-7 and TC-128 add a seven-day compatibility run and bounded hold. | NFR-005-AC-7; TC-128 | missing-requirement |
| FND-074 | medium | **Closed:** compiler-caused formatting and repairable lint changes could have been mislabeled as tool incompatibility. The requirement now permits an older hold only for a reproduced failure in a required tool. | NFR-005 Rationale/Verification/AC-7; ADR-002 revisit trigger | wrong-requirement |

## Checklist Result

- NFR-005 retains one stakeholder relationship and adds one sequential
  acceptance criterion with a matching sequential test case.
- The exact version, required commands, acceptance/refusal conditions, release
  trigger, hold evidence, and maximum hold duration are measurable.
- TC-116 covers the exact compiler and every target; TC-127 covers current-head
  gate identity; TC-128 covers new-release, adoption, real incompatibility,
  formatting-only, lint-only, and expired-hold cases.
- No backward-compatibility promise, old compiler floor, formatting-stability
  promise, or unbounded exception remains.
- Existing containment, unsafe-code, traceability, performance, rights, and
  manual evaluation/release boundaries are unchanged.

## Intake and Evidence Boundary

- Selected review set: `base`.
- Optional analyses selected: none.
- `quoin write . --types NFR,SpecReview` supplied the live authoring contracts.
- This review establishes a specification baseline. It does not claim that Rust
  1.98.1, hosted statuses, or the Rust implementation have executed yet.

## Result

**PASS.** All base findings are closed. Implementation may pin and exercise
Rust 1.98.1; any incompatibility claim must be supported by TC-128 evidence.
