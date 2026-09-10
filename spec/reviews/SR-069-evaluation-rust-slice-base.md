---
id: SR-069
title: "Rust evaluation aggregation slice base review"
type: SpecReview
scope: "FR-017-AC-2, FR-017-CON-1, FR-017-CON-3, TC-110"
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: "reviews"
  - target: "ix://agent-ix/engineering-assurance/TC-110"
    type: "reviews"
---

# SR-069: Rust evaluation aggregation slice base review

## Review Configuration

- Review set: owner-approved `base` subset.
- Method: QUOIN `/spec-review` base checklist.
- Scope boundary: deterministic evaluation-envelope validation and complete-only
  aggregation. Agent execution, transcript-byte loading, current-HEAD
  observation, package qualification, release, invocation cutover, and legacy
  removal remain outside this slice.

## Summary

**PASS after specification remediation.** The reviewed slice is sufficiently
bounded and testable for implementation. It does not authorize host integration
or removal.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | **Closed through `/specify`.** “Missing, malformed, unavailable, stale-revision, changed-governing-file, and incomplete” did not define the 28-cell population, required fields, terminal-pair relationship, duplicate/unsupported behavior, or stable aggregate output. FR-017 now closes the host/scenario sets, request/result protocols, completeness rules, identity joins, terminal pairing, and deterministic failure classes. | FR-017 Inputs, Outputs, Behavior; FR-017-AC-2; TC-110 | missing-requirement |
| FND-002 | high | **Closed through `/specify`.** The prior requirement could be read to place report-file and transcript I/O in the reusable library, contradicting ADR-002's library/CLI boundary and inviting path-dependent validation. FR-017-CON-3 now keeps the reusable validator pure and allocates transcript loading, host execution, and current-HEAD comparison to explicit adapters. | ADR-002; FR-017-CON-3; TC-110 | wrong-requirement |
| FND-003 | medium | **Closed through `/specify`.** “Matches retained behavior” did not say whether unknown object fields and invalid scalar types were tolerated, enabling another permissive ad hoc JSON walker. The v1 request now uses closed typed Rust structures and refuses malformed or unknown structure before aggregation. | FR-017 Behavior; FR-017-AC-2; TC-110 | missing-requirement |
| FND-004 | medium | **Closed through `/specify`.** Aggregate diagnostics had no ordering contract, so input permutation could alter retained evidence and differential results. The requirement now fixes failure-class and host/scenario ordering and requires permutation invariance. | FR-017 Behavior; FR-017-AC-2; TC-110 | missing-requirement |
| FND-005 | medium | **Closed through `/specify`.** The slice boundary did not distinguish digest-shape validation from verifying transcript bytes. It now validates relative references and digest format only; transcript-byte loading and equality remain explicitly pending in the adapter phase. | FR-017 Behavior, Sequence; FR-017-CON-3 | missing-requirement |
| FND-006 | high | **Closed after Rust boundary review through `/specify`.** The byte API initially had no resource ceiling, allowing input far larger than the only meaningful 28-cell population to allocate during JSON decoding. It now refuses more than 8 MiB before decoding, matching the existing machine-input ceiling. | FR-017 Behavior; FR-017-AC-2; TC-110 | missing-requirement |
| FND-007 | medium | **Closed after Rust boundary review through `/specify`.** “Relative and non-traversing” depended on the build host's native path parser, so Windows separators or drive prefixes could pass on Linux and acquire different meaning later. Transcript references now use one normalized forward-slash protocol grammar on every host. | FR-017 Behavior; FR-017-AC-2; TC-110 | missing-requirement |

## Base checklist result

- Scope and ownership are explicit and consistent with ADR-002.
- Inputs, outputs, success behavior, adverse behavior, and deterministic ordering
  are testable without an external host.
- Every changed obligation maps to TC-110.
- No unresolved blocking ambiguity, contradiction, or missing acceptance oracle
  remains in the reviewed slice.
- The review makes no claim for FR-017-AC-1, FR-017-AC-3, FR-017-AC-4, or
  FR-018.

## Decision

Implementation of the pure additive Rust slice may proceed. The specification
cycle must be reopened if implementation requires filesystem access, a host
bridge, a changed persisted schema, or a different evaluation population.
