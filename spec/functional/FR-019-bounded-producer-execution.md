---
id: FR-019
title: "Execute declared producers through one bounded Rust boundary"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-005"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/ADR-002"
    type: "requires"
---

# FR-019: Execute declared producers through one bounded Rust boundary

## Description

Engineering Assurance SHALL expose one public Rust library boundary that
executes a caller-declared producer directly under explicit identity, input,
environment, concurrency, time, and output bounds and returns a typed result to
the caller-owned response adapter.

## Inputs

- One versioned producer-execution request containing a stable producer name,
  version and source revision; an executable path and expected SHA-256 digest;
  the direct procedure; an ordered argument vector; an explicit working root;
  an ordered environment; exact input-artifact or input-projection path/digest
  bindings; execution limits; and the expected response protocol binding.
- One caller-owned Rust response adapter whose declared binding equals the
  request and which converts a completed bounded process observation into the
  consumer's typed domain result.
- An optional cancellation handle supplied by the caller.

## Outputs

- A deterministic SHA-256 identity over every execution-request field.
- One typed `ProducerExecutionResult<T>` carrying that exact request identity,
  producer provenance, observed executable digest, and one closed execution
  state.
- A completed state carrying the caller adapter's typed observation and exact
  output/artifact references; no non-completed state carries an observation.

## Behavior

- When a request is submitted, the executor SHALL validate every request field,
  require an absolute canonical executable path without ambient `PATH`
  resolution, reject a linked or non-regular executable, and require its
  retained bytes to match the declared SHA-256 digest before launch.
- When a request is submitted, the executor SHALL validate each selected input as a bounded,
  non-linked regular file beneath the selected working root and require its
  bytes to match the declared path/digest binding before launch.
- When a valid slot is available, the executor SHALL launch the declared
  executable directly with the ordered arguments, no shell, an empty inherited
  environment, and only the request's validated environment entries.
- The executor SHALL place the child and its descendants in one invocation-owned
  process group and SHALL terminate and reap that group before returning from a
  timeout, cancellation, containment failure, or direct-child completion.
- If the host cannot establish and terminate an invocation-owned descendant
  process group, then the executor SHALL return `containment_failure` before
  producer launch.
- The executor SHALL enforce non-zero finite wall-clock, stdout, stderr, input-
  byte, and executor-wide concurrency limits at their exact admitted boundaries.
- The executor SHALL reject a request whose timeout exceeds 86,400 seconds,
  whose stdout or stderr limit exceeds 8,388,608 bytes, whose selected-input
  limit exceeds 1,073,741,824 bytes, whose argument population exceeds 4,096,
  whose environment population exceeds 512, or whose executor concurrency
  limit is outside one through 64.
- The executor SHALL return distinguishable `unavailable`, `refused`, `failed`,
  `timed_out`, `malformed_response`, `containment_failure`, `cancelled`, and
  `completed` states without deriving state from diagnostic prose.
- When bounded process execution completes, the executor SHALL give the exit status and
  captured output only to the request-bound Rust response adapter; an adapter
  rejection SHALL return `malformed_response` without a typed observation.
- The executor SHALL preserve the exact request identity and producer provenance
  in every result state.
- The executor SHALL release its concurrency slot and remove invocation-owned
  temporary state before returning on every path.
- A consumer SHALL remain the owner of its response schema, satisfaction,
  violation, stopping-rule, replay, oracle, and domain-failure semantics.
- Existing Engineering Assurance binary adapters SHALL reuse the same bounded
  process kernel rather than retaining a second process runner.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-019-CON-1 | The executor SHALL NOT invoke a shell or inherit the ambient process environment. | Security | Test (TC-123) |
| FR-019-CON-2 | The executor SHALL NOT persist evidence, infer evidence sufficiency, qualify a producer, or make a human disposition. | Responsibility | Test (TC-127) |
| FR-019-CON-3 | The implementation SHALL remain a module of the existing `engineering_assurance` crate unless a separately accepted isolation requirement justifies another crate. | Architecture | Inspection (TC-127) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-019-AC-1 | A valid synthetic producer request executes once through the public Rust library, and the typed completed result carries the exact request identity, executable digest, producer provenance, and adapter-produced observation. | Test (TC-122) |
| FR-019-AC-2 | Every request field is identity-significant, and an invalid version, procedure, path, digest, input, environment name, response binding, or zero/over-limit budget refuses before producer launch. | Property (TC-123) |
| FR-019-AC-3 | Unavailable, refused, failed, timed-out, malformed-response, containment-failure, cancelled, and completed remain distinguishable, and only completed carries a typed observation. | Test (TC-124) |
| FR-019-AC-4 | Exact wall-clock, stream, input-byte and concurrency boundaries are admitted while the next value is refused or terminated; descendants and invocation-owned state do not survive completion, timeout, cancellation, or failure. | Test (TC-125) |
| FR-019-AC-5 | A caller-owned typed response adapter receives bounded process output without requiring the consumer to parse CLI stdout, and Engineering Assurance contains no domain oracle, qualification verdict, evidence store, or Quoin record clone. | Test (TC-127) |
| FR-019-AC-6 | A real `quire-verification` synthetic consumer compiles against the accepted Engineering Assurance revision and distinguishes completed domain observations from every executor non-completion state without a local runner or stdout adapter. | Integration (TC-126) |

## Dependencies

- **Upstream**: accepted [ADR-002](../assets/adr/0002-rust-native-engineering-assurance.md), the exact Rust 1.98.1 boundary in [FR-014](./FR-014-versioned-rust-boundary.md), and [NFR-004](../non-functional/NFR-004-no-parallel-assurance-framework.md).
- **Downstream**: [IT-006](../integration/IT-006-producer-execution-consumer.md) and the consumer-owned `quire-verification` qualification cases.
