---
id: FR-014
title: "Expose a versioned Rust library and CLI boundary"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/ADR-002"
    type: "requires"
---

# FR-014: Expose a versioned Rust library and CLI boundary

## Description

Engineering Assurance SHALL expose its reusable production and qualification
behavior through the repository-owned `engineering_assurance` Rust library and
the `engineering-assurance` native CLI.

## Inputs

- Versioned Engineering Assurance request documents.
- Explicit file or repository roots selected by the caller.
- Structured responses from Quire, Quoin, ix-flow, and supported host tools.

## Outputs

- Typed Rust library results.
- One versioned JSON result per machine-facing CLI invocation.
- Diagnostics on stderr and a documented non-zero status for refusal or error.

## Behavior

- Each machine-facing protocol SHALL carry an
  `engineering-assurance.<capability>/v1` discriminator.
- The CLI SHALL reject an unknown protocol version before performing a write or
  invoking a downstream action.
- The CLI SHALL write machine results only to stdout.
- The CLI SHALL write human-readable diagnostics only to stderr.
- The CLI SHALL buffer the machine result and write no stdout bytes until one
  complete result is available; interruption cannot leave a partial JSON value.
- Each protocol schema SHALL declare a request and result limit no greater than
  64 MiB and a non-zero downstream deadline no greater than 30 minutes. A
  command that needs no downstream host still enforces the byte limits.
- When timeout, cancellation, or a termination signal occurs, the CLI SHALL
  terminate any child it started, perform no later write or downstream action,
  and return a distinguishable non-success status.
- The library SHALL keep reusable validation and transformation behavior free
  of filesystem, subprocess, socket, and environment access.
- The CLI SHALL preserve the selected repository root as an explicit boundary.

## Error Conditions

Malformed JSON, an unknown protocol version, an escaping path, an over-limit
request or result, an invalid or expired deadline, timeout, cancellation,
termination signal, an unavailable required host, and a structurally invalid host response each return a
distinguishable non-success result. No such result is reported as successful or
partially accepted.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-014-CON-1 | The Rust boundary SHALL NOT persist an authoritative evidence record. | Responsibility | Test |
| FR-014-CON-2 | The CLI SHALL NOT derive a verdict from arbitrary stdout or stderr. | Responsibility | Test |
| FR-014-CON-3 | The implementation SHALL remain in this repository. | Architecture | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-014-AC-1 | A root Cargo package produces the `engineering_assurance` library and `engineering-assurance` binary without requiring a new repository. | Test (TC-096) |
| FR-014-AC-2 | Every machine-facing command emits exactly one declared-version JSON result on stdout and sends diagnostics only to stderr. | Property (TC-098) |
| FR-014-AC-3 | Unknown versions, malformed inputs, escaping roots, unavailable hosts, and invalid host responses fail before any write or downstream action. | Property (TC-099) |
| FR-014-AC-4 | A complete call-surface audit finds no evidence persistence, arbitrary stdout verdict recovery, or reusable I/O behavior in the library (CON-1, CON-2). | Test (TC-101) |
| FR-014-AC-5 | Boundary tests prove the declared byte/deadline limits, buffer-before-stdout rule, child termination, and no-side-effect behavior for timeout, cancellation, and termination signals. | Property (TC-121) |

## Dependencies

- **Upstream**: accepted [ADR-002](../assets/adr/0002-rust-native-engineering-assurance.md).
- **Downstream**: FR-015 through FR-018 use this boundary.
