---
id: FR-029
title: "Expose bounded producer execution as a versioned CLI command"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-028"
    type: "references"
---

# FR-029: Expose bounded producer execution as a versioned CLI command

## Description

The `engineering-assurance` binary SHALL provide a `producer-execute` command
that runs one FR-019 producer-execution request and writes the FR-019 result
protocol to stdout, so a consumer that cannot take a Cargo dependency on the
Rust library, such as a licence-separated Rust crate or a non-Rust tool, uses
the same bounded executor through a process boundary.

## Inputs

- `--request <JSON>`: one `engineering-assurance.producer-execution-request/v1`
  document as the argument value, or `-` to read the document from stdin up to
  the CLI's existing stdin limit.
- `--schema <request|result>`: select a published schema instead of running a
  request. Exactly one of `--request` and `--schema` is given.

## Outputs

- A run writes one line to stdout: the RFC 8785 canonical JSON bytes of the
  complete `engineering-assurance.producer-execution-result/v1` document,
  followed by one newline, and exits with status 0 for every closed execution
  state, including `refused`, `unavailable`, `timed_out`, `failed`,
  `malformed_response`, `containment_failure` and `cancelled`. The consumer
  reads the closed `state` field to decide the outcome.
- A request that yields no result writes the existing
  `engineering-assurance.error/v1` machine error for capability
  `producer-execute` to stdout and its message to stderr, and exits with
  status 2. The error codes are `producer_execute_request_unreadable`,
  `producer_execute_request_invalid`, `producer_execute_request_structure_invalid`
  and `producer_execute_result_encoding_failed`.
- `--schema request` and `--schema result` write the published JSON Schema
  (draft 2020-12) for the request and result documents.

## Behavior

- The command SHALL parse the request through the retained-request check of
  FR-019, so an unknown field, an elided default, a wrong type or a document
  that does not equal its typed re-serialization yields
  `producer_execute_request_invalid`, a document that is not JSON or exceeds
  the stdin limit yields `producer_execute_request_unreadable`, and a
  structurally invalid request, including a relative capability root, a relative
  or non-normalized artifact path, a zero or over-limit bound or an unsupported
  protocol, yields `producer_execute_request_structure_invalid`. No producer is
  launched on any of these.
- The command SHALL execute the request through one `ProducerExecutor` whose
  concurrency ceiling is the request's `budget.max_concurrency`, with a
  cancellation token carrying the request's own cancellation binding that the
  command never cancels.
- The command SHALL carry one built-in response adapter bound to the response
  protocol `engineering-assurance.process-evidence` version `1` and the adapter
  `engineering-assurance.cli-process-evidence-adapter` version `1`. The adapter
  accepts a normal exit admitted by the request exit policy and returns the
  observation `{"exitCode": <integer>}`; the consumer interprets the producer's
  stdout, stderr and retained output artifacts itself. A request naming any
  other response binding is refused by the executor with state `refused` and
  reason `adapter_binding`, before the producer is launched.
- The command SHALL NOT add a dependency, a Cargo feature or a library module
  beyond those the `full` feature already enables, and SHALL NOT interpret
  producer output, persist evidence or decide sufficiency (FR-019-CON-2).
- The published schemas SHALL be the files
  `src/producer_execute_schemas/producer-execution-request-v1.schema.json` and
  `src/producer_execute_schemas/producer-execution-result-v1.schema.json`,
  embedded in the binary so the installed command alone publishes them. A change to the closed request or result
  fields SHALL change the protocol version.
- On a host other than Linux the executor reports `containment_failure`, as
  FR-019 defines; the command reports that state unchanged.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-029-AC-1 | A valid request run through the real binary writes exactly one newline-terminated canonical result line with the result protocol, the producer provenance, timing, bounded process evidence and a `completed` state carrying the adapter observation (Linux), exits 0, and the line equals its own RFC 8785 canonicalization. | Test (TC-214) |
| FR-029-AC-2 | An unreadable, non-conforming, unknown-field or structurally invalid request yields the matching typed machine error, exit status 2 and no producer launch. | Test (TC-215) |
| FR-029-AC-3 | A request naming a response binding other than the built-in adapter yields a result with state `refused` and reason `adapter_binding`, and an unavailable or timed-out producer yields its own closed state, each with exit 0. | Test (TC-216) |
| FR-029-AC-4 | `--schema request` and `--schema result` write the published schemas; the published request schema accepts the real request and refuses a mutant for each required field, and the result schema accepts the real result line. | Test (TC-217) |
| FR-029-AC-5 | A consumer runs the installed binary from an unrelated directory with an empty environment, with no Cargo project or library linkage. | Test (TC-218) |

## Dependencies

- **Upstream**: [FR-019](./FR-019-bounded-producer-execution.md) owns the executor, request and result; [FR-028](./FR-028-shared-cli-output.md) owns stdout and machine-error encoding.
- **Downstream**: the TL stable-release producer consumer (TL-22) and any process-boundary consumer of the producer protocol.
