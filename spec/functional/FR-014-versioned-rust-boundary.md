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
- Structured responses from supported external tools.

## Outputs

- Typed Rust library results.
- One versioned JSON result per machine-facing CLI invocation.
- Diagnostics on stderr and a documented non-zero status for refusal or error.

## Behavior

- Each machine-facing protocol SHALL carry an
  `engineering-assurance.<capability>/v1` discriminator.
- The CLI SHALL reject an unknown protocol version before a write or downstream
  action.
- The CLI SHALL write machine results only to stdout and diagnostics only to
  stderr.
- Reusable library validation and transformation behavior SHALL remain free of
  filesystem, subprocess, socket, and environment access.
- The CLI SHALL preserve the caller-selected repository root as an explicit
  boundary.

## Error Conditions

Malformed JSON, an unknown protocol version, an escaping path, an unavailable
required host, and a structurally invalid host response each return a
distinguishable non-success result and are never reported as successful.

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
| FR-014-AC-3 | Unknown versions, malformed inputs, escaping roots, unavailable hosts, and invalid host responses fail before a write or downstream action. | Property (TC-099) |
| FR-014-AC-4 | A call-surface audit finds no evidence persistence, arbitrary stdout verdict recovery, or reusable I/O behavior in the library. | Test (TC-101) |

## Dependencies

- **Upstream**: accepted [ADR-002](../assets/adr/0002-rust-native-engineering-assurance.md).
- **Downstream**: FR-015 through FR-018 use this boundary.
