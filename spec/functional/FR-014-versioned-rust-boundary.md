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
- Caller-supplied UTF-8 Rust source bytes and a closed source-audit role.

## Outputs

- Typed Rust library results.
- One versioned JSON result per machine-facing CLI invocation.
- Diagnostics on stderr and a documented non-zero status for refusal or error.
- Typed, deterministically ordered Rust source-audit findings.

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
- Filesystem and Git access for a capability SHALL live in a host adapter
  outside the reusable library: in the binary where a production command needs
  it, and in the qualification tree where only qualification reads it.
- The pure Rust source audit SHALL parse Rust syntax rather than matching raw
  source substrings, so comments and string literals cannot create or satisfy a
  capability finding.
- For a caller-designated reusable-library source, the audit SHALL reject use
  or path access rooted in filesystem, child-program, environment, socket, or
  capability-filesystem APIs, including lexical import aliases declared in the
  same parsed source.
- The source audit SHALL remain a syntactic per-document boundary.
- The source audit SHALL NOT claim compiler name resolution across source
  documents.
- Complete library containment SHALL combine the audit of every library module
  with the pinned compiler, dependency-policy, and final executable-path gates.
- The reusable-library audit SHALL admit only the exact Cargo package-name and
  package-version `env!` expressions already used as compile-time package
  metadata; it SHALL reject other environment macros and output macros.
- For a caller-designated first-party Rust test source, the audit SHALL require
  every test function to have at least one bare `#[trace(...)]` attribute, an
  exact unaliased `use ix_trace_rs::trace` import in the parsed file, at least
  one `TC-XXX` literal, and at least one `*-AC-N` literal.
- The source audit SHALL reject path-qualified trace attributes, aliased trace
  imports, malformed trace arguments, invalid UTF-8, invalid Rust syntax, and
  any individual source larger than 2,097,152 bytes through typed findings or
  request errors.
- The source audit SHALL perform no filesystem, process, environment, network,
  clock, or persistence access and SHALL order findings by function name,
  closed category, and safe capability label.

## Error Conditions

Malformed JSON, an unknown protocol version, an escaping path, an unavailable
required host, and a structurally invalid host response each return a
distinguishable non-success result and are never reported as successful.
Invalid or oversized Rust source refuses source auditing. A forbidden library
capability, absent or non-canonical trace import, path-qualified or malformed
trace attribute, or missing test/acceptance identifier produces a typed
containment finding and cannot satisfy the corresponding static gate.

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
| FR-014-AC-4 | An AST-based audit of every reusable-library module finds no filesystem, environment, child-program, network, persistence, or arbitrary-stdout recovery capability named directly or through a lexical alias in that source; comments and literals cannot manufacture a finding, and the audit does not claim cross-document compiler name resolution. | Test (TC-101) |

## Dependencies

- **Upstream**: accepted [ADR-002](../assets/adr/0002-rust-native-engineering-assurance.md).
- **Downstream**: FR-015 through FR-018 use this boundary.
