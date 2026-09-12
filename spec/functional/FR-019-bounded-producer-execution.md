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
environment, confinement, concurrency, time, and output bounds and returns a
typed result to the caller-owned response adapter.

## Inputs

- One closed `engineering-assurance.producer-execution-request/v1` request
  containing:
  - a stable producer name, exact version and source revision;
  - an absolute producer executable path and expected SHA-256 retained-byte
    digest;
  - an exact kind/version/revision/digest binding for the caller's domain
    request or context;
  - an exact kind/version/revision/digest binding for the response adapter;
  - a direct procedure, explicit capability root, ordered argument bindings,
    normalized environment map, ordered input bindings, closed stdin binding,
    ordered output-artifact bindings, cooperative-confinement contract, exact
    cancellation authority/event, execution limits, and response protocol;
  - an exit policy that is either every normal exit code or a normalized set of
    exact normal exit codes admitted to the response adapter.
- One caller-owned Rust response adapter whose exact implementation binding and
  response protocol equal the request and which converts a completed bounded
  process observation into the consumer's typed serializable domain result.
- One cancellation handle carrying the request's exact authority/event binding.

Argument, input and output collections are identity-significant ordered lists.
Environment keys and exact admitted exit codes are normalized sets represented
in deterministic key or numeric order. Input-role and output-role names are
unique within their respective request collections.

## Outputs

- A request identity represented as `sha256-jcs` of the RFC 8785 canonical
  JSON bytes of the complete closed request, including its protocol domain and
  version.
- RFC 8785 canonical JSON bytes and a `sha256-jcs` identity for the complete
  closed result artifact. Canonicalization includes every portable result
  field and the caller-owned serializable observation when present; it excludes
  only live operating-system handles whose role, path, length and byte digest
  are represented in the canonical artifact. An observation that cannot be
  represented canonically produces a typed encoding refusal and no result
  identity.
- Structurally invalid input produces a typed `InvalidExecutionRequest` and no
  request identity or execution result.
- A structurally valid request produces one
  `ProducerExecutionResult<T>` carrying the exact request identity, producer
  provenance, observed executable digest when available, monotonic timing,
  cancellation binding/event, optional bounded raw process evidence, exact
  output-artifact references/digests with immutable retained-byte readers, and
  one closed execution state.
- Every launched-process state may carry bounded raw process evidence and
  validated output-artifact references. Only `completed` carries the caller
  adapter's typed domain observation.

## Behavior

- The executor SHALL validate the closed request structure before minting its
  identity. Empty, over-limit, internally inconsistent, non-relative artifact,
  or unsupported protocol fields are structurally invalid.
- For a structurally valid request, the executor SHALL open the absolute
  capability root and executable without following symbolic links in any path
  component, copy the selected executable into a sealed retained descriptor,
  hash that exact snapshot, and execute the same sealed descriptor without
  ambient `PATH` resolution.
- The executor SHALL treat the declared ordered input population as the
  complete working projection. It SHALL open each input relative to the retained
  capability-root descriptor without following symbolic links, copy its exact
  observed bytes into a sealed retained descriptor, hash that snapshot before
  launch, and materialize only those retained input bytes plus declared empty
  output parents into an invocation-owned staged working tree. The executor SHALL
  prevent undeclared, changed-unbound, or post-preflight files beneath the source
  capability root from becoming visible through the producer working directory.
  Descriptor arguments and stdin SHALL expose the same sealed input snapshots.
- The executor SHALL validate every declared output path beneath the
  invocation-owned staged tree before launch, enforce output-artifact count and
  aggregate byte bounds, and after termination open each produced artifact
  without following links. It SHALL copy each exact observed output into a
  sealed retained descriptor and compute the returned role, declared relative
  path, byte length and SHA-256 digest from that same immutable snapshot. Each
  returned output artifact SHALL provide the response adapter and caller a
  reader for that retained snapshot without reopening the producer pathname.
- When a valid slot is available, the executor SHALL launch the retained
  executable directly with the ordered resolved arguments, no shell, the
  invocation-owned staged projection as working directory, an empty inherited
  environment, only the request's environment entries, and the exact null or
  retained-input stdin binding.
- The supported `process-group-v1` confinement profile SHALL require an exact
  producer confinement-contract binding that prohibits session/group escape,
  daemonization and untracked descendants. The executor SHALL establish an
  invocation-owned process group, terminate and reap that group before return,
  and return `containment_failure` if the host cannot establish the profile or
  if it observes a descendant leaving the group. This profile SHALL NOT claim
  adversarial full-tree containment; a consumer requiring that stronger
  property must use a separately accepted provider mechanism.
- The executor SHALL enforce non-zero finite wall-clock, stdout, stderr, input-
  byte, output-artifact-count, output-artifact-byte, and executor-wide
  concurrency limits at their exact admitted boundaries.
- The executor SHALL reject structurally a request whose timeout exceeds 86,400
  seconds, whose stdout or stderr limit exceeds 8,388,608 bytes, whose selected
  input or output-artifact byte limit exceeds 1,073,741,824 bytes, whose
  argument population exceeds 4,096, whose input or output population exceeds
  4,096, whose observable descendant population exceeds 4,096, whose
  environment population exceeds 512, or whose executor
  concurrency limit is outside one through 64.
- The response adapter SHALL receive every bounded normal terminal exit admitted
  by the request exit policy, including admitted non-zero exits. A rejected
  normal exit, signal termination, capture failure, or artifact failure is an
  executor `failed` result rather than a domain observation.
- The executor SHALL release its concurrency slot and invocation-owned process
  resources before returning on every path.
- A consumer SHALL remain the owner of its response schema, satisfaction,
  violation, stopping-rule, replay, oracle, and domain-failure semantics.
- Existing Engineering Assurance binary adapters SHALL reuse the same bounded
  process kernel rather than retaining a second process runner.
- The Cargo package SHALL expose a `producer-execution` feature usable with
  default features disabled. That feature SHALL compile the public producer
  API without activating any direct dependency used solely for package/archive,
  onboarding, CLI, YAML, regex, or source-audit behavior; dependencies shared
  with the producer API remain admissible. The default full Engineering
  Assurance package SHALL preserve its existing library and CLI surface.
- First-party executable fixtures for TC-122 through TC-125 SHALL be Rust
  binaries. Foreign-language/runtime producers remain supported inputs to the
  public protocol, but the tests SHALL NOT execute Python, JavaScript, or shell
  fixture programs as first-party evidence for this implementation.

## Closed state boundaries

| State | Exact boundary |
| --- | --- |
| `unavailable` | A structurally valid request selects an executable that cannot be opened or the retained executable cannot be launched. |
| `refused` | An identity-bearing request fails a pre-launch identity, capability, adapter, input, output-parent, cancellation-binding, or concurrency admission check. |
| `failed` | A launched process has a rejected normal exit, signal termination, stream overflow/read failure, process observation failure, or output-artifact failure. |
| `timed_out` | Monotonic elapsed execution time reaches the request deadline before terminal observation. |
| `malformed_response` | The exact request-bound caller adapter rejects an otherwise admitted bounded terminal response. |
| `containment_failure` | The selected confinement profile cannot be established or an escaping descendant/group breach is observed. |
| `cancelled` | The exact bound cancellation event is observed before completion and all observable invocation resources are terminated/reaped. |
| `completed` | The exact adapter accepts an admitted bounded terminal response and returns `T`; this is the only state carrying `T`. |

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-019-CON-1 | The executor SHALL NOT invoke a shell or inherit the ambient process environment. | Security | Test (TC-123) |
| FR-019-CON-2 | The executor SHALL NOT persist evidence, infer evidence sufficiency, qualify a producer, or make a human disposition. | Responsibility | Test (TC-127) |
| FR-019-CON-3 | The implementation SHALL remain a module of the existing `engineering_assurance` crate unless a separately accepted isolation requirement justifies another crate. | Architecture | Inspection (TC-127) |
| FR-019-CON-4 | The `process-group-v1` profile SHALL NOT be represented as adversarial full-descendant containment. | Safety | Test (TC-125) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-019-AC-1 | A valid Rust synthetic producer request executes once through the public Rust library, and the typed completed result carries the exact `sha256-jcs` request identity, executable digest, producer provenance, bounded raw evidence, monotonic timing, immutable retained output artifacts, and adapter-produced observation; every portable result-field/state mutation changes its canonical result bytes and identity. | Test (TC-122) |
| FR-019-AC-2 | Every request field is JCS-identity-significant, ordered and set collections have their declared semantics, structurally invalid input mints no identity, an identity-bearing digest, capability, input, adapter, or cancellation mismatch refuses before producer launch, and the invocation working directory exposes exactly the declared staged input/output projection despite extra, changed-unbound, or post-preflight source-root files. | Test (TC-123) |
| FR-019-AC-3 | Unavailable, refused, failed, timed-out, malformed-response, containment-failure, cancelled, and completed remain distinguishable and canonically serializable; launched failures retain bounded raw evidence where observable; and only completed carries `T`. | Test (TC-124) |
| FR-019-AC-4 | Exact wall-clock, stream, input-byte, output-count/byte and concurrency boundaries are admitted while the next value is structurally invalid or terminated; an admitted non-zero exit reaches the adapter; ordinary descendants are reaped and an escaping-descendant mutant produces `containment_failure`. | Test (TC-125) |
| FR-019-AC-5 | A caller-owned typed response adapter receives the exact bound terminal evidence without requiring the consumer to parse CLI stdout, and Engineering Assurance contains no domain oracle, qualification verdict, evidence store, or Quoin record clone. | Test (TC-127) |
| FR-019-AC-7 | A minimal consumer compiles the existing crate with default features disabled and only `producer-execution` enabled; Engineering Assurance activates no direct dependency used solely for package/archive, onboarding, CLI, YAML, regex, or source-audit behavior, while dependencies shared with the producer API remain admissible and the default full feature preserves the existing package library and CLI gates. | Test (TC-128) |

## Dependencies

- **Upstream**: accepted [ADR-002](../assets/adr/0002-rust-native-engineering-assurance.md), the exact Rust 1.98.1 boundary in [FR-014](./FR-014-versioned-rust-boundary.md), and [NFR-004](../non-functional/NFR-004-no-parallel-assurance-framework.md).
- **Downstream**: [IT-006](../integration/IT-006-producer-execution-consumer.md) and the consumer-owned `quire-verification` qualification cases.
  The retired FR-019-AC-6 asserted that the real `quire-verification` consumer
  had adopted this boundary. Every technical claim it made about Engineering
  Assurance is proven here already — the feature-gated compile and dependency
  census by FR-019-AC-7, the caller-owned adapter with no stdout parsing by
  FR-019-AC-5, and the distinguishable non-completion states by FR-019-AC-3.
  What remained was the consumer's identity, which is adoption by another
  repository rather than a guarantee this one can make or verify, so it is
  recorded here as a dependency instead of as an acceptance criterion that
  could never be discharged locally. Criterion numbering is left unchanged;
  renumbering would silently re-point every existing reference to AC-7.
