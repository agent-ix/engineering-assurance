---
id: SR-083
title: "Rust package-audit adapter and governed deletion base review"
type: SpecReview
analysis: base
scope: "FR-003-AC-1, FR-003-AC-2, FR-003-AC-4, FR-003-AC-5, FR-003-AC-6, NFR-003-AC-1, NFR-003-AC-2, FR-007-AC-3, FR-017-AC-3, FR-017-AC-4, FR-017-CON-2, FR-017-CON-3, FR-018-AC-2, FR-018-AC-3, FR-018-CON-1, FR-018-CON-2, TC-014, TC-015, TC-017, TC-018, TC-019, TC-037, TC-040, TC-111, TC-112, TC-113, TC-114"
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
| FND-016 | high | **Closed after Rust review returned to `/specify`.** A timeout on only the direct package-manager child can leave a descendant holding stdout or stderr open, making the adapter block while joining its capture threads after the declared deadline. FR-017 and TC-111 now require one invocation-owned process group and group termination before output joins on timeout or direct-child exit. | FR-017 Behavior; TC-111 |
| FND-017 | high | **Closed after Rust review returned to `/specify`.** Naming one temporary output root did not prove that pip/npm caches, tool temporary workspaces, or new repository-root entries stayed within it. The requirement now binds package-manager cache/temp paths and requires bounded before/after top-level entry correspondence around every invocation, after the admitted npm staging cleanup. | FR-017 Behavior; TC-111 |
| FND-018 | medium | **Closed after Rust review expanded the deletion census.** Deleting `tests/test_packages.py` would remove the only TC-019/TC-037 assertion that local/repository module and plugin installation instructions remain distinct and ordered. The deletion scope now retains those acceptance cases in a Rust test rather than silently turning their matrix rows green without an executable assertion. | FR-003-AC-6; FR-007-AC-3; TC-019; TC-037 |
| FND-019 | high | **Closed after Rust review returned to `/specify`.** Archive validation occurs after external builders read package sources, so rejecting a linked archive member cannot undo a builder following a linked module, plugin, or pilot source outside the selected tree. FR-017 and TC-111 now require complete bounded source preflight before either builder runs. | FR-017 Behavior; TC-111 |
| FND-020 | high | **Closed after Rust review returned to `/specify`.** Process-group termination was implemented only on Unix while the non-Unix branch silently degraded to direct-child termination, so a descendant retaining an output pipe could still make the declared deadline unbounded. The shared process adapter now fails before spawn when process-group containment is unavailable. | FR-017 Behavior; TC-111 |
| FND-021 | high | **Closed after Rust review returned to `/specify`.** `pyproject.toml` selects the wheel build backend but was absent from the fixed-source preflight, allowing a linked build configuration to be followed before archive inspection. It is now a required regular preflighted source. | FR-017 Behavior; TC-111 |
| FND-022 | high | **Closed after Rust review returned to `/specify`.** Default tar iteration preprocesses GNU/PAX extension headers and allocates their payloads before the adapter observes a member, bypassing the declared entry/kind/resource gate. npm archives now use raw iteration, refuse extension headers before preprocessing, count every raw header, and reject non-empty directory payloads before skipping them. | FR-017 Behavior; TC-111 |
| FND-023 | medium | **Closed through `/specify`.** The prior “no output outside temporary directory” sentence overclaimed an operating-system sandbox and contradicted the separately accepted tool-owned compiler cache. The requirement now names the invocation-owned output classes actually confined and expressly excludes both that compiler cache and an unimplemented general package-manager sandbox. | FR-017 Behavior; FR-017-CON-2 |
| FND-024 | high | **Closed after the governed deletion census was rerun.** Removing `tests/test_packages.py` also erased the only TC-015, TC-017, TC-018, and NFR-003 package-stability bindings even though the first review recorded only its documentation assertion. The Rust archive integration and explicit allowlist tests now carry npm and stability bindings, the membership gate carries the missing/extra refusal, and a bounded real repository-source install preserves TC-017 discovery before deletion. Aggregate traceability restores the deleted population. | FR-003-AC-1; FR-003-AC-2; FR-003-AC-4; FR-003-AC-5; NFR-003-AC-1; NFR-003-AC-2; TC-014; TC-015; TC-017; TC-018; TC-040 |
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
- Boundaries cover exact/over process output, archive bytes, total raw entry
  count, member bytes, aggregate expansion, path bytes, installed traversal,
  unavailable process-group containment, malformed reports, missing/multiple
  archives, every unsupported member kind and extension header, membership
  drift, rights refusal, installed discovery drift, and byte divergence.
- The reusable Rust policies remain I/O-free. Filesystem, environment, archive,
  child-process, and temporary-directory capabilities remain in explicit
  binary-only adapter modules.

## Decision

Implementation may proceed. Deletion remains last: first record same-revision
Python/Rust success, a direct Rust dispatch, independent Rust success, and a
reversible dispatch change. Historical review references are records, not
executable dependencies, and remain unchanged.
