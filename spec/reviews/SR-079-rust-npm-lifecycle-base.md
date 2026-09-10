---
id: SR-079
title: "Rust npm package-lifecycle slice base review"
type: SpecReview
analysis: base
scope: "FR-017-AC-3, FR-017-AC-4, FR-017-CON-2, FR-017-CON-3, TC-111, TC-112"
review_set: base
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

# SR-079: Rust npm package-lifecycle slice base review

## Summary

**PASS after specification remediation.** This owner-selected base review
covers only the existing npm staging, cleanup, and publication-refusal behavior
being ported from MJS to the repository-owned Rust CLI. The npm archive remains
a configuration and artifact bundle; this slice does not invent native-binary
distribution, public registry publication, or a new package format.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | **Closed through `/specify`.** The retained cleanup force-deletes fixed package-root paths without proving they are the copies created by staging. The requirement now mandates complete source/destination correspondence before any deletion and no deletion on any mismatch. | FR-017 Behavior; TC-111 | missing-requirement |
| FND-002 | high | **Closed through `/specify`.** Sequential retained copies can leave partial output after an error. The requirement now mandates complete preflight, invocation-owned rollback on returned errors, and post-copy byte correspondence. | FR-017 Behavior; TC-111 | missing-requirement |
| FND-003 | medium | **Closed through `/specify`.** Directory traversal had no link, special-file, portable-path, entry-count, individual-byte, or total-byte boundary. Closed ceilings and fail-before-write behavior are now explicit. | FR-017 Behavior and Error Conditions; TC-111 | missing-requirement |
| FND-004 | medium | **Closed through `/specify`.** “Publication refusal” did not state whether it could read the package or contact a registry. It is now an unconditional closed refusal with no package read or network action. | FR-017 Behavior; TC-111 | missing-requirement |
| FND-005 | medium | **Closed through `/specify`.** Owner acceptance of npm distribution could be misread as a requirement to embed or download native executables. The requirement preserves npm as the existing configuration/artifact bundle and makes no native-binary distribution claim. | FR-017 Behavior; FR-017-CON-2 | wrong-requirement |
| FND-006 | low | **Closed by scope clarification.** This slice can back npm lifecycle portions of TC-111 and TC-112 only. Archive membership adaptation, installed-bundle integration, complete host-configuration inspection, cutover, and final legacy removal remain pending. | FR-017-AC-3; FR-017-AC-4; FR-018 | correct-requirement-no-evidence |
| FND-007 | high | **Closed after integration probe and `/specify` revision.** Emitting the direct CLI result from an npm lifecycle hook contaminated `npm pack --json` with a second JSON document. The interface now distinguishes direct machine rendering from explicit npm-hook rendering, whose stdout is empty and whose only success signal is exit status. | FR-017 Behavior and Error Conditions; TC-111; TC-112 | wrong-requirement |

## Base checklist result

- The selected repository root, closed operations, fixed staged population,
  typed result, stdout/stderr boundary, and failure outcomes are explicit.
- Direct JSON rendering and stdout-silent npm-hook dispatch are distinct, so
  neither tool claims the other tool's machine-output channel.
- Destructive cleanup has a full-population correspondence precondition and
  idempotent all-absent state.
- Resource, file-kind, symlink, partial-write, rollback, and mutation states are
  specified before implementation.
- Package-manager configuration remains declarative and public publication
  remains separately controlled.
- No evaluation, evidence-store, package-format, native-distribution, or
  cross-repository responsibility is introduced.

## Decision

Implementation of the Rust npm stage, cleanup, and publication-refusal slice
may proceed. The remaining TC-111/TC-112 integration and static populations
stay open.
