---
id: SR-083
title: "Rust package-audit adapter and governed deletion base review"
type: SpecReview
analysis: base
scope: "FR-003-AC-1, FR-003-AC-2, FR-003-AC-5, FR-017-AC-3, FR-017-AC-4, FR-017-CON-2, FR-017-CON-3, FR-018-AC-2, FR-018-AC-3, FR-018-CON-1, FR-018-CON-2, TC-111, TC-112, TC-113, TC-114"
review_set: base
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-003"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-018"
    type: reviews
---

# SR-083: Rust package-audit adapter and governed deletion base review

## Review Configuration

- Review set: owner-approved `base` subset.
- Method: QUOIN `/spec-review` base checklist.
- Scope boundary: port the retained wheel/npm build, archive audit, offline
  install, discovery, and cross-format comparison into a binary Rust adapter;
  cut over the repository target; then remove only the superseded package-audit
  and content-rights Python paths and executable references. Package formats and
  their data remain supported distribution surfaces. Evaluation hosts, native
  Quire language design, and other Python capabilities remain outside the slice.

## Summary

**PASS after `/specify` remediation.** The amended requirements define one
bounded package-audit adapter that composes the existing pure Rust membership
and content-rights policies. External package tools retain construction and
installation ownership; first-party acceptance decisions live in Rust.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | **Closed through `/specify`.** The prior requirement named archive and installed-bundle integration but did not bound child processes, archives, expansion, member populations, or installed traversal. Exact commands, offline flags, timeout/output ceilings, archive/member/expanded-byte ceilings, and typed refusal behavior are now explicit. | FR-017 Behavior; FR-017 Error Conditions; TC-111 |
| FND-009 | high | **Closed through `/specify`.** Removing only the four Python source/test files would leave Rust tests importing or spawning the deleted implementations and would retain policy exceptions with no owner. FR-018 now requires removal of every executable import/subprocess reference and both temporary exceptions in the same final step while preserving inert historical reviews. | FR-018 Behavior; TC-113; TC-114 |
| FND-010 | medium | **Closed through `/specify`.** “Decode the archive” did not distinguish admitted directories from files or reject links, devices, FIFOs, sockets, duplicate names, unsafe paths, and decompression bombs. The member-kind, path, population, and expansion rules now fail closed before policy acceptance. | FR-017 Behavior; TC-111 |
| FND-011 | medium | **Closed through `/specify`.** “Installed bundle integration” did not identify the retained observations. The requirement now names exact host surfaces, canonical skill/workflow populations, canonical/pilot YAML equivalence, no-link traversal, fixed module roots, and cross-format canonical byte identity without claiming full IT-002 Quire/ix-flow execution. | FR-003; FR-017 Behavior; TC-111 |
| FND-013 | medium | **Closed after implementation design returned to `/specify`.** Bounding only regular-file members left directory-only archive populations unbounded. FR-017 and TC-111 now impose the 65,536 ceiling on every archive entry before a directory can be skipped. | FR-017 Behavior; TC-111 |
| FND-014 | high | **Closed after implementation design returned to `/specify`.** The initial temporary-output rule ignored the six fixed root paths staged by the reviewed npm lifecycle hook. The adapter now must prove those destinations absent before `npm pack`, admit no other root output, and perform exact lifecycle cleanup after both successful and failed pack attempts; an unverifiable population remains in place and fails the audit. | FR-017 Behavior; TC-111; FR-017-CON-3 |
| FND-015 | medium | **Closed after implementation design returned to `/specify`.** Calling Cargo from the reviewed npm hook uses a reusable compiler cache that is not a distribution artifact; forcing it beneath the invocation temp root would rebuild gigabytes per audit. The requirement now confines package-builder outputs, package-manager cache, archives, and installations while leaving the selected Cargo cache tool-owned. | FR-017 Behavior; FR-017-CON-2 |
| FND-012 | low | No open base-review finding remains. The npm package remains an approved distribution format; the adapter does not claim that it distributes a native executable or broaden the Rust-port finish line. | FR-017-CON-2; FR-017 Dependencies |

## Base checklist result

- FR-003 retains ownership of the wheel/npm distribution contract; FR-017 owns
  the native audit adapter; FR-018 owns cutover and deletion sequencing.
- Inputs, fixed child-command identities, one invocation-owned output root,
  typed success/error behavior, archive populations, installed observations,
  and cleanup boundaries are explicit.
- FR-017-AC-3 remains mapped to TC-111 and cannot turn green unless real wheel
  and npm artifacts both pass. TC-112 gates exact declarative Rust dispatch;
  TC-113 and TC-114 gate same-revision removal and rollback.
- Boundaries cover exact/over process output, archive bytes, total entry count,
  member bytes, aggregate expansion, path bytes, installed traversal, malformed
  reports, missing/multiple archives, every unsupported member kind, membership
  drift, rights refusal, installed discovery drift, and byte divergence.
- The reusable Rust policies remain I/O-free. Filesystem, environment, archive,
  child-process, and temporary-directory capabilities remain in explicit
  binary-only adapter modules.

## Decision

Implementation may proceed. Deletion remains last: first record same-revision
Python/Rust success, a direct Rust dispatch, independent Rust success, and a
reversible dispatch change. Historical review references are records, not
executable dependencies, and remain unchanged.
