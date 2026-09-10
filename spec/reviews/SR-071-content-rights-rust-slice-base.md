---
id: SR-071
title: "Rust content-rights classifier slice base review"
type: SpecReview
analysis: base
scope: "FR-017-AC-5, FR-017-CON-3, TC-119"
review_set: base
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: "reviews"
  - target: "ix://agent-ix/engineering-assurance/TC-119"
    type: "reviews"
---

# SR-071: Rust content-rights classifier slice base review

## Review Configuration

- Review set: owner-approved `base` subset.
- Method: QUOIN `/spec-review` base checklist.
- Scope boundary: deterministic classification of caller-supplied path, kind,
  bytes, and protected tokens. Repository enumeration, filesystem/environment
  adapters, package selection, publication, host execution, cutover, and removal
  remain outside this slice.

## Summary

**PASS after specification remediation.** The amended requirement defines a
self-contained, testable pure Rust content-rights classification boundary and
does not claim the combined repository-qualification gate is complete.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | **Closed through `/specify`.** “Preserve the existing suffix populations” made executable Python the only source of the accepted and forbidden file types. FR-017 now enumerates every admitted, forbidden, and unreviewed suffix outcome. | FR-017 Behavior; FR-017-AC-5; TC-119 | missing-requirement |
| FND-002 | high | **Closed through `/specify`.** The retained self-exception list omitted the new Rust policy source and test, so scanning the candidate tree would reject the implementation merely for naming prohibited categories. The requirement now names the canonical Rust policy files and explicitly limits the old Python exceptions to additive parity. | FR-017 Behavior; FR-018 | missing-requirement |
| FND-003 | medium | **Closed through `/specify`.** FR-017-CON-3 prohibited I/O only for the evaluator/aggregator, leaving the new reusable classifier's purity ungoverned. The constraint now applies to the whole reusable qualification library and allocates tree enumeration to a later binary adapter. | FR-017-CON-3; FR-017-AC-5 | wrong-requirement |
| FND-004 | medium | **Closed through `/specify`.** Line numbers were observable findings, but the accepted set of line boundaries was implicit in Python `splitlines()`. The requirement now enumerates LF, CR, CRLF, vertical-tab, form-feed, file/group/record separators, NEL, and Unicode line/paragraph separators. | FR-017 Outputs, Behavior; TC-119 | missing-requirement |
| FND-005 | medium | **Closed through `/specify`.** A caller-supplied path could reach suffix and policy exceptions without a portable path grammar. Unsafe, non-normalized, backslash, control, dot, and parent paths now refuse before classification. | FR-017 Behavior; FR-017-AC-5; TC-119 | missing-requirement |
| FND-006 | high | **Closed after Rust boundary review through `/specify`.** The initial output wording allowed the rejected path to be echoed in a `PathInvalid` finding. An absolute path could therefore disclose the workstation location the classifier exists to suppress. Unsafe-path findings now carry no path value and no rejected bytes. | FR-017 Outputs; FR-017-AC-5; TC-119 | wrong-requirement |

## Base checklist result

- FR-017 remains linked to its existing stakeholder and prerequisite
  requirements; no new owner or product boundary is introduced.
- The input bytes, path/kind metadata, explicit token set, typed output, complete
  rule population, exceptions, line semantics, negative cases, and ordering are
  independently verifiable.
- FR-017-AC-5 maps to TC-119; the wider FR-017-AC-3 remains mapped to pending
  TC-111 and cannot become green through this slice.
- No option or state-transition matrix applies to a pure classifier. Size,
  suffix, path, encoding, line-boundary, exception, and disclosure boundaries
  are required test cases.
- No blocking identifier, link, terminology, security, or scope ambiguity
  remains.

## Decision

Implementation of the pure Rust content-rights classifier may proceed. The
specification cycle must reopen before adding tree enumeration, environment or
filesystem access, changing package formats, cutting over invocations, or
removing the retained Python classifier.
