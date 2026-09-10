---
id: SR-068
title: "Rust review of the ix-flow lifecycle host"
type: SpecReview
analysis: code-review
scope: "FR-016-AC-3, FR-016-CON-1, FR-016-CON-5..7, TC-107; src/workflow.rs; src/workflow_host.rs; workflow-host CLI"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-016"
    type: reviews
---

## Summary

The repository rules and `agent-skills/rust-review/SKILL.md` were applied to
the additive Rust ix-flow lifecycle adapter. The reviewed boundary delegates
run persistence, event-chain verification, transitions, and human gates to the
exact ix-flow candidate; Rust owns only strict binding, bounded invocation,
typed projection, retry reconciliation, and refusal behavior. Eight findings
were corrected and mutation-tested where applicable. No open Rust finding
remains in the implementation slice.

The implementation passes locally against public ix-flow 0.2.3, but this
review does not supply human acceptance of the revised compatibility matrix.
The slice therefore remains additive and cannot authorize invocation cutover,
JavaScript removal, consumer migration, or release enforcement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-138 | high | Closed: pristine interrupted-run recovery compared the definition name and phase but omitted the requested definition version. A stale-version pristine run could receive a new binding before the later mismatch was detected. Recovery now requires the exact requested version before mutation. Removing that conjunct makes the live TC-107 test mutate the state tree and fail its byte digest comparison. | `src/workflow_host.rs:956`; `tests/workflow_host_cli.rs:325`; FR-016-AC-3 |
| FND-139 | high | Closed: the first adapter parsed ix-flow history but never invoked ix-flow's own chain verifier. Rust now calls `ix-flow verify` before each existing-run mutation and before every successful result, validates the closed verification shape, and rejects a reported break without reading or recomputing state. Removing the final verification call makes the tampered-chain TC-107 case return exit 0 instead of 2. | `src/workflow_host.rs:651`; `tests/workflow_host_cli.rs:367`; FR-016-CON-1; FR-016-CON-6 |
| FND-140 | high | Closed: command-identity mismatch and missing structured error data were always classified as a read-response error, even after a mutating invocation could have committed. Validation now carries mutation possibility through command, envelope, and error handling so those cases return `ix_flow_outcome_indeterminate`. | `src/workflow_host.rs:834`; `src/workflow_host.rs:850`; FR-016-AC-3 |
| FND-141 | medium | Closed: early process-observation and pipe failures could return without reaping the child or joining both reader threads, and diagnostics named hard-coded production limits rather than the limits actually applied. Every exit path now attempts child cleanup, observes both readers, and reports the applied duration and byte ceiling. | `src/workflow_host.rs:701`; FR-016-AC-3 |
| FND-142 | medium | Closed: a decision owner beginning with `-` could be interpreted as an ix-flow option value boundary, while control characters remained accepted in binding and host-path fields. The pure request boundary now rejects both classes before process execution. | `src/workflow.rs:201`; `src/workflow.rs:207`; `src/workflow.rs:454`; FR-016-AC-3; FR-016-CON-5 |
| FND-143 | medium | Closed: malformed `gate.auto_acked` and `gate.acknowledged` payloads were silently discarded through optional lookup and `filter_map`, which could hide contradictory event evidence. Both event kinds are now parsed through closed typed payloads and malformed history refuses the decision. | `src/workflow_host.rs:995`; `src/workflow_host.rs:1067`; FR-016-CON-1 |
| FND-144 | medium | Closed: successful lifecycle responses were not consistently checked for agreement between envelope, summary, and run data before a later mutation. Instance id, definition identity/hash, phase, state version, gates, and summary now agree at one validation seam; the missing-run response also has a constrained no-run shape. | `src/workflow_host.rs:882`; FR-016-AC-3 |
| FND-145 | low | Closed: the outbound ix-flow `run_binding` item was assembled by inserting an `id` key into `serde_json::Value`. A private flattened `Serialize` type now makes the emitted field set compiler-visible. | `src/workflow_host.rs:1221`; FR-016-CON-1 |
| FND-146 | low | No open Rust finding remains. Production code contains no unsafe block, panic, unchecked numeric cast, lint suppression, shell invocation, ambient state lookup, asynchronous/lock surface, or direct ix-flow state-file access. Every new requirement test carries canonical bare `ix_trace_rs::trace` markers. | `src/workflow.rs`; `src/workflow_host.rs`; `tests/workflow_host_cli.rs`; NFR-005 |

## Gate results

| Gate | Result |
| --- | --- |
| Exact Rust 1.98.1 formatting | pass; inherited vendored ix-trace-rs nightly-option warnings remain visible |
| Exact Rust 1.98.1 Clippy | pass; workspace, all targets, all features, locked, `-D warnings`, two build jobs |
| Exact Rust 1.98.1 tests | pass; 56 tests, including eight live lifecycle tests and two host-adapter unit tests, run serially |
| Exact Rust 1.98.1 rustdoc | pass; workspace, all features, no dependencies, `-D warnings` |
| `cargo deny --locked check` | pass; advisories, bans, licenses, and sources; existing duplicate transitive versions remain warnings under policy |
| `cargo audit` | pass; 78 locked dependencies scanned against 1,243 advisories |
| Live ix-flow 0.2.3 lifecycle | pass; create/bind/resume, pristine recovery, accept/reject, idempotent retry, opposite-choice refusal, automatic-gate refusal, and intact-chain verification |
| Adverse host behavior | pass; absent/wrong-version host, malformed and oversized input, malformed/timed-out/oversized responses, wrong response command, and post-mutation indeterminate classification |
| Mutation probes | pass; removing stale-version recovery guard changes state bytes; removing post-resume chain verification admits tampered history |
| Compatibility acceptance | pending; candidate 0.2.3 provenance and behavior are verified, but the matrix deliberately records no attributed human acceptance |
| `make lint` | pass |
| `make test` | pass; content rights, 189 Python tests, and module validation |
| `make package-audit` | pass |
| Local Quire validation | pass; inherited duplicate-provider diagnostics only |
| Slice traceability | pass; TC-107 and FR-016-AC-3 are now backed by eleven canonical `ix_trace_rs::trace` markers |
| Whole-port integration traceability | expected incomplete; 213/242 while later FR-014 aggregate containment, FR-016 cutover, FR-017 qualification, FR-018 removal, and NFR-005 final-state obligations remain pending |

## Review disposition

**PASS for Rust implementation quality; CONDITIONAL for FR-016-AC-3.** The
only remaining condition is explicit human acceptance of the candidate matrix
that pins ix-flow 0.2.3. This review does not authorize FR-016-AC-4 cutover or
any retained JavaScript deletion.
