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
domain-response contract. Seventeen findings were closed through `/specify`; no open
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
| FND-033 | high | Closed: “SHA-256 over every field” did not define portable bytes. The request identity is now `sha256-jcs` over RFC 8785 canonical JSON of the closed request domain/version; list and normalized-set semantics are explicit. | FR-019 Inputs; FR-019 Outputs; FR-019-AC-2 |
| FND-034 | high | Closed: producer execution identity omitted the caller's domain request/context and the exact adapter implementation. Both are now exact kind/version/revision/digest bindings, and the runtime adapter must match its request binding. | FR-019 Inputs; FR-019-AC-1; IT-006 |
| FND-035 | high | Closed: non-zero exit was ambiguously both an executor failure and adapter input. The request now binds either all normal exits or a normalized exact set; admitted non-zero exits reach the adapter and rejected exits remain executor failures. | FR-019 Behavior; FR-019-AC-4 |
| FND-036 | high | Closed: stdin was implicitly null and artifacts were not bounded or available on non-completion. Stdin is now null or an exact retained input; output roles/paths and count/byte limits are declared before launch; launched states may retain bounded raw evidence and artifact references while only completed carries `T`. | FR-019 Inputs; FR-019 Outputs; FR-019 Behavior |
| FND-037 | high | Closed: canonicalize/hash followed by pathname execution admitted replacement races. The executor now retains no-follow descriptors for the capability root, executable and inputs; it executes or exposes those same retained descriptors and opens outputs relative to the retained root. | FR-019 Behavior; FR-019-AC-2 |
| FND-038 | high | Closed: process-group containment was represented as full descendant containment even though a child can call `setsid`. The only admitted profile is now explicitly cooperative and contract-bound; an observed escape returns `containment_failure`, and the interface makes no adversarial full-tree claim. | FR-019 Behavior; FR-019-CON-4; TC-125 |
| FND-039 | high | Closed: state boundaries, structural invalidity, cancellation identity and timing were underspecified. A closed state table now separates identity-less invalid requests from identity-bearing results and binds cancellation plus monotonic timings in the result. | FR-019 Outputs; FR-019 Closed state boundaries |
| FND-040 | high | Closed during implementation review: descendant population was described as bounded but absent from the request, while executor concurrency lived outside request identity. Both exact ceilings are now request fields with hard maxima; an executor whose ceiling differs from the request refuses before launch. | FR-019 Inputs; FR-019 Behavior; FR-019-AC-2; TC-123; TC-125 |
| FND-041 | high | Closed during implementation review: retaining an opened executable/input inode did not freeze bytes against in-place mutation after hashing. Selected bytes are now copied into a sealed anonymous descriptor; the exact snapshot is hashed and the same sealed descriptor is executed or exposed to the producer. | FR-019 Behavior; FR-019-AC-2; TC-123 |
| FND-042 | medium | Closed during implementation review: an exact exit-code set admitted negative and over-255 values that a normal Unix process cannot produce. Structural validation now limits every exact code to zero through 255 before identity is minted. | FR-019 Behavior; FR-019-AC-2; TC-123 |

## Base checklist result

- FR-019 implements existing user value in US-005 rather than introducing a
  new product or repository.
- Inputs name every identity-bearing execution field; outputs distinguish every
  host state and make typed observations impossible outside `completed`.
- Error, cancellation, concurrency, exact-limit, descendant, environment,
  shell, malformed-response, retained-I/O, JCS identity, adapter-binding and
  cleanup cases are mapped to TC-122 through TC-125.
- Ownership and negative-capability checks are mapped to TC-127.
- The real cross-repository seam is mapped to IT-006 and TC-126; provider tests
  alone cannot claim consumer acceptance.

## Decision

**PASS after remediation.** Implementation may proceed as one module in the
existing crate. The implementation may expose a typed library API and reuse its
process kernel from existing binary adapters; it may not add a generic CLI
runner, evidence persistence, domain oracle, new crate, or new repository.
