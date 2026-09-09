---
id: SR-036
title: "Base review of the Rust 1.98.1 qualification baseline"
type: SpecReview
analysis: base
scope: "NFR-005, ADR-002, TM-001 TC-116"
review_set: base
relationships:
  - target: ix://agent-ix/engineering-assurance/NFR-005
    type: reviews
---
# SR-036: Base review of the Rust 1.98.1 qualification baseline

## Summary

This review covers the correction from an inherited Rust 1.75 minimum to exact
Rust 1.98.1 as the supported and qualification compiler. It does not establish
a future-release SLA, hosted status policy, or ecosystem-wide compiler census.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
|---|---|---|---|---|
| FND-071 | high | **Closed:** Rust 1.75 was copied into the port specification without evidence that it was suitable for this new package. NFR-005 selects exact Rust 1.98.1 and requires the real local target/tool matrix. | NFR-005 measurement table; NFR-005-AC-1; TC-116 | wrong-requirement |
| FND-074 | medium | **Closed:** formatting and repairable lint changes are not tool incompatibilities. Any request to change the selected compiler must identify a reproduced failure in a required tool. | NFR-005 rationale; ADR-002 revisit trigger | wrong-requirement |

## Checklist Result

- The exact compiler and required local commands are measurable.
- TC-116 covers the exact compiler and every declared target.
- No backward-compatibility promise, old compiler floor, hosted gate, or
  ecosystem adoption process is introduced.
- Existing containment, unsafe-code, rights, and manual-operation boundaries
  remain intact.

## Result

**PASS.** Rust 1.98.1 is the exact local qualification compiler. A different
version requires evidence of actual required-tool incompatibility or a new
owner decision; existing files alone are not justification.
