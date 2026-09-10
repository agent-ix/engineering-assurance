---
id: SR-081
title: "Rust content-rights tree adapter and cutover base review"
type: SpecReview
analysis: base
scope: "FR-017-AC-3, FR-017-AC-4, FR-017-CON-2, FR-017-CON-3, FR-018-AC-2, FR-018-AC-3, TC-111, TC-112, TC-113, TC-114"
review_set: base
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-018"
    type: reviews
---

# SR-081: Rust content-rights tree adapter and cutover base review

## Summary

**PASS after repeated specification remediation.** This owner-selected base review
covers only the binary adapter that applies the already-reviewed pure Rust
content-rights classifier to the repository's retained Git-selected tree, the
direct `make test` cutover, and the prerequisites for later deletion of the
retained Python checker after same-revision correspondence and rollback
evidence. It does not change content
policy, create a repository scanner service, inspect another repository, or
advance unrelated archive, evaluation-host, or workflow-provider work.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | **Closed through `/specify`.** “Inspect the complete selected tree” did not identify the selector or require the supplied root to be the Git worktree top level, permitting a subdirectory or alternate traversal to silently reduce coverage. The requirement now fixes the exact `git ls-files` population and requires canonical top-level equality. | FR-017 Behavior; TC-111 | missing-requirement |
| FND-002 | high | **Closed through `/specify`.** The retained selector and filesystem reader had no process-output, duration, entry, path, individual-file, total-byte, or protected-token population ceilings. Exact and over-limit behavior plus typed refusal are now required. | FR-017 Behavior and Error Conditions; TC-111 | missing-requirement |
| FND-003 | high | **Closed through `/specify`.** Deleting the Python checker would also delete the only test of public-repository rights-metadata agreement. The same-revision gate now requires the complete governed positive/negative population, including repository metadata, before both the checker and its Python tests can be removed. | FR-017 Behavior; TC-111; FR-018 | missing-requirement |
| FND-004 | medium | **Closed through `/specify`.** A selected missing or special entry was previously skipped, and malformed/non-UTF-8 Git output had no stable disposition. These conditions now fail closed without reducing the inspected population. | FR-017 Behavior and Error Conditions; TC-111 | wrong-requirement |
| FND-005 | medium | **Closed through `/specify`.** The adapter result and exit-status contract were implicit. The requirement now names one versioned result, exact status meanings, deterministic finding order, and the no-source/no-token disclosure boundary. | FR-017 Inputs, Outputs, and Behavior; TC-111 | missing-requirement |
| FND-006 | high | **Closed through `/specify`.** The generic removal requirement did not say how the content-rights cutover proves same-revision parity or rollback before deletion. It now requires exact status/finding correspondence, a reversible direct-dispatch change while Python remains present, an independent Rust pass, and removal of the obsolete policy exceptions with the old files. | FR-018 Behavior; TC-113; TC-114 | missing-requirement |
| FND-007 | high | **Closed after implementation returned to `/specify`.** The initial deletion plan overlooked that `scripts/audit_packages.py` imports `text_findings` from the checker, and deleting it made Python test collection fail. The direct tree gate may cut over now, but the checker, parity tests, and temporary exemptions must remain until the package/archive Rust adapter removes that last consumer. | FR-017 Behavior; FR-018 Behavior; TC-111; TC-113 | missing-requirement |

## Base checklist result

- The explicit root, exact Git population, file kinds, environment input,
  result protocol, status meanings, and typed failures are defined.
- Happy, error, boundary, population, disclosure, deterministic-order,
  same-revision, cutover, rollback, and deletion cases are assigned to existing
  test cases without claiming their aggregate completion.
- The binary adapter owns filesystem, process, and environment access; the
  reusable classifier remains I/O-free.
- The direct host configuration remains declarative and no external repository
  or service changes are introduced.

## Decision

Implementation of this content-rights tree adapter and its governed local
cutover may proceed. The Python checker may not yet be deleted because the
package/archive audit consumes it. TC-111 through TC-115 remain incomplete for
their wider archive, installed-bundle, host-configuration, and final-removal
populations.
