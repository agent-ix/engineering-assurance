---
id: SR-021
title: "Failure-domain review of the Rust-native Engineering Assurance migration"
type: SpecReview
analysis: failure-domain
scope: "FR-014..FR-018, NFR-005, TC-096..TC-127"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: reviews
---

## Summary

The candidate fails closed for malformed inputs, invalid host responses, stale
revisions, and premature deletion. Two load-bearing failure domains remain
undefined: exhaustion/termination at the CLI-host boundary, and ambiguous or
conflicting consumer-registry identities.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-052 | high | The versioned CLI/host protocol has no declared input/output bound, downstream deadline, cancellation or signal rule. A host can hang, emit unbounded data, or be interrupted after a side effect while FR-014 only defines malformed/unavailable/invalid-response failures. | FR-014:20-49,63-66 | missing-requirement |
| FND-053 | high | The consumer registry has no wire discriminator, closed classification vocabulary, unique repository/path key, duplicate/conflict rule, or tamper-binding digest. FR-015 only specifies snapshot fields and one submodule deduplication case, so two owners can classify the same artifact differently without a defined refusal. | FR-015:31-34,58-74,80-85,103-104 | missing-requirement |

## Failure classes checked

- Trust boundaries: CLI input, repository root, subprocess host, registry, and
  retained reference implementations.
- Identity: protocol version, repository/commit/path/blob, module/schema digest,
  interface version, run binding, and historical canonicalization.
- Recovery: additive parity and rollback are specified; termination semantics
  and registry-conflict recovery are not.

## Repeat-review dispositions

| ID | Disposition |
| --- | --- |
| FND-052 | Fixed: FR-014 now sets 64 MiB request/result ceilings, a 30-minute maximum downstream deadline, buffered stdout, and fail-closed timeout/cancellation/signal behavior; TC-121 covers boundary and negative cases. |
| FND-053 | Fixed: FR-015 defines registry/v1, canonical repository identity, a closed classification vocabulary, uniqueness, exclusion attribution, digest binding, and mutation/conflict refusal; TC-122 covers them. |
