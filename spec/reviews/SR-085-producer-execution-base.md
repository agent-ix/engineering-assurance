---
id: SR-085
title: "Bounded producer-execution base review"
type: SpecReview
analysis: base
scope: "ADR-002, FR-014-AC-4, FR-019, NFR-004-AC-2, IT-006, TC-122 through TC-127"
review_set: base
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/NFR-004"
    type: reviews
---

## Summary

The owner-selected base review examined the minimum specification needed for
Engineering Assurance #34: one public bounded Rust execution module, its
ownership exception, and one real Rust consumer integration. The reviewed scope
is implementable without a new repository, crate, runner, evidence store, or
domain-response contract. Seven findings were closed through `/specify`; no open
base-review finding remains.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-025 | high | Closed: ADR-002, the master scope, FR-014, and NFR-004 prohibited every library process boundary, making the requested public executor impossible while appearing to admit it. They now permit only the FR-019 module and keep all other library modules I/O-free. | ADR-002; FR-014-AC-4; NFR-004-AC-2 |
| FND-026 | high | Closed: placing the executable beneath the consumer working root would reject normal installed Rust tools. FR-019 now requires an explicit absolute canonical executable path and digest without ambient PATH resolution, while only selected input files must remain beneath the working root. | FR-019 Behavior; FR-019-AC-2 |
| FND-027 | high | Closed: returning captured stdout for downstream interpretation would recreate the forbidden consumer-local parser. A caller-owned typed Rust response adapter is bound in the request and receives bounded output inside the library call; only its typed observation may enter `completed`. | FR-019 Inputs; FR-019 Behavior; FR-019-AC-5 |
| FND-028 | high | Closed: a generic completed result could collapse fuzz, mutation, catalogue, and qualification semantics into Engineering Assurance. The consumer owns its observation type, response schema, satisfaction/violation, stopping rule, replay, oracle, and domain failures; the executor owns only host execution states and provenance. | FR-019 Behavior; FR-019-CON-2; IT-006 |
| FND-029 | medium | Closed: “bounded” was not measurable when every request could select an arbitrarily large limit. FR-019 now states hard maxima for wall time, each stream, selected inputs, argument and environment populations, and executor concurrency while requiring exact caller-selected ceilings beneath them. | FR-019 Behavior; FR-019-AC-4; TC-125 |
| FND-030 | high | Closed: adding a public runner alongside the existing private binary process adapter would create two containment implementations. FR-019 now requires existing binary adapters to reuse the same bounded kernel, and TC-127 admits process execution only in that module. | FR-019 Behavior; TC-127; NFR-004-AC-2 |
| FND-031 | medium | Closed: timeout coverage alone did not define caller cancellation or slot cleanup. Cancellation is a typed non-completion state; every completion and failure path must terminate descendants, remove invocation state, and release the executor-wide concurrency slot. | FR-019 Behavior; FR-019-AC-3; FR-019-AC-4 |
| FND-032 | low | No open base-review finding remains. IDs are sequential, every AC has a test, error/state/boundary paths are explicit, and the real consumer gate is separate from provider unit coverage. | FR-019; IT-006; TC-122 through TC-127 |

## Base checklist result

- FR-019 implements existing user value in US-005 rather than introducing a
  new product or repository.
- Inputs name every identity-bearing execution field; outputs distinguish every
  host state and make typed observations impossible outside `completed`.
- Error, cancellation, concurrency, exact-limit, descendant, environment,
  shell, malformed-response, and cleanup cases are mapped to TC-122 through
  TC-125.
- Ownership and negative-capability checks are mapped to TC-127.
- The real cross-repository seam is mapped to IT-006 and TC-126; provider tests
  alone cannot claim consumer acceptance.

## Decision

**PASS after remediation.** Implementation may proceed as one module in the
existing crate. The implementation may expose a typed library API and reuse its
process kernel from existing binary adapters; it may not add a generic CLI
runner, evidence persistence, domain oracle, new crate, or new repository.
