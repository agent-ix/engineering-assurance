---
id: SR-075
title: "Rust module-manifest classifier slice base review"
type: SpecReview
analysis: base
scope: "FR-017-AC-7, FR-017-CON-3, TC-121"
review_set: base
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: "reviews"
  - target: "ix://agent-ix/engineering-assurance/TC-121"
    type: "reviews"
---

# SR-075: Rust module-manifest classifier slice base review

## Review Configuration

- Review set: owner-approved `base` subset.
- Method: QUOIN `/spec-review` base checklist.
- Scope boundary: pure validation of caller-supplied manifest, authoritative
  schemas, edge registry, and artifact schema/skeleton resources. Discovery,
  filesystem traversal, Quire invocation, distribution selection, cutover, and
  removal remain outside this slice.

## Summary

**PASS after specification remediation.** The requirement consumes the
authoritative schema offline rather than duplicating it in Rust, closes YAML
ambiguity and resource-exhaustion paths, and defines deterministic typed
findings without claiming the combined qualification gate complete.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | **Closed through `/specify`.** “Port manifest validation” did not allocate schema ownership and could be implemented as a second hand-written Filament manifest grammar. FR-017 now requires caller-supplied authoritative schemas, draft-aware offline validation, and forbids treating a Rust projection as a replacement for that validation. | FR-017 Behavior; FR-017-AC-7 | missing-requirement |
| FND-002 | high | **Closed through `/specify`.** The retained YAML loader can merge keys and did not state duplicate-key behavior, allowing document identity to depend on parser semantics. Both manifest and skeleton frontmatter now reject duplicate and merge keys. | FR-017 Behavior; TC-121 | missing-requirement |
| FND-003 | medium | **Closed through `/specify`.** No input ceiling bounded schema compilation or the artifact-resource population. Exact individual, count, and combined ceilings now refuse before schema compilation. | FR-017 Behavior; FR-017-AC-7; TC-121 | missing-requirement |
| FND-004 | medium | **Closed through `/specify`.** A schema reference could escape the selected module or be echoed after rejection. The requirement now defines the complete portable path grammar and prohibits unsafe-reference disclosure. | FR-017 Outputs; Behavior; TC-121 | missing-requirement |

## Base checklist result

- FR-017 remains linked to its accepted owner and prerequisites; Quire and the
  installed module schema retain their existing ownership.
- Inputs, outputs, failure categories, exact resource limits, path grammar,
  deterministic ordering, schema/frontmatter/heading behavior, and no-I/O
  constraints are independently verifiable.
- FR-017-AC-7 maps to TC-121; the wider FR-017-AC-3 remains pending TC-111.
- No state transition applies. TC-121 covers happy, error, security, size,
  population, permutation, schema-draft, resource, and heading boundaries.
- No blocking identifier, link, terminology, security, or scope ambiguity
  remains.

## Decision

Implementation of the pure Rust module-manifest classifier may proceed. The
specification cycle must reopen before schema discovery, filesystem traversal,
Quire invocation, distribution selection, invocation cutover, or retained-code
removal.
