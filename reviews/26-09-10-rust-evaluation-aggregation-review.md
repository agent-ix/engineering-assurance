---
id: SR-070
title: "Rust review of pure evaluation aggregation"
type: SpecReview
analysis: code-review
scope: "FR-017-AC-2, FR-017-CON-1, FR-017-CON-3, TC-110; src/evaluation.rs; tests/evaluation_parity.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

## Summary

The repository rules and the exact `agent-skills/rust-review/SKILL.md`
checklist were applied to
the additive pure Rust evaluation validator and aggregator. The implementation
uses closed Serde inputs, exhaustive host/scenario/failure enums, a fixed-size
cell index, deterministic failure ordering, and a bounded byte entry point. It
performs no filesystem, environment, process, network, clock, transcript, or
release operation. Four review findings were corrected; no open Rust finding
remains in this slice.

This review does not claim the cli-agent-evals host interface, transcript-byte
verification, current-HEAD comparison, package qualification, invocation
cutover, release evidence, or legacy removal.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-147 | high | Closed through a return to `/specify`: the initial byte API decoded an unbounded document even though only 28 cells can contribute. It now refuses more than 8 MiB before JSON decoding; the typed aggregation index retains at most one reference and a saturating count per closed cell rather than one pointer per duplicate. | `src/evaluation.rs:26`; `src/evaluation.rs:583`; `src/evaluation.rs:600`; `tests/evaluation_parity.rs:419`; FR-017-AC-2 |
| FND-148 | medium | Closed: aggregate and envelope failure kinds were initially constructed as free-form strings, making the discriminant prose and scattering the wire catalog. They are now exhaustive `EnvelopeFailure` and `EvaluationFailure` enums; one `Display`/`Serialize` boundary preserves the retained string format. | `src/evaluation.rs:144`; `src/evaluation.rs:221`; `src/evaluation.rs:499`; FR-017-AC-2 |
| FND-149 | medium | Closed through a return to `/specify`: native `Path` parsing gave a protocol path different traversal meaning on Linux and Windows. The validator now admits only normalized forward-slash relative components and rejects POSIX roots, drive prefixes, backslashes, NUL, empty components, `.` and `..`; focused cases cover both separator families and a drive prefix. | `src/evaluation.rs:794`; `tests/evaluation_parity.rs:244`; FR-017-AC-2; FR-017-CON-3 |
| FND-150 | medium | Closed: the first immutable-revision predicate preserved the old evaluator's five mutable labels but still admitted ranges, wildcards, `nightly`, and `x` components. The predicate now rejects all of those before a cell can be complete, with focused host/source cases. | `src/evaluation.rs:767`; `tests/evaluation_parity.rs:253`; FR-017-AC-2 |
| FND-151 | low | No open Rust finding remains. Production code has no unsafe block, panic, unchecked numeric cast, lint suppression, untyped JSON walking, filesystem or process call, ambient state lookup, async/lock surface, or unbounded per-cell accumulator. Every new requirement test uses the canonical bare `ix_trace_rs::trace` marker. | `src/evaluation.rs`; `tests/evaluation_parity.rs`; NFR-005 |

## Gate results

| Gate | Result |
| --- | --- |
| Exact Rust 1.98.1 formatting | pass; inherited vendored ix-trace-rs nightly-option warnings remain visible |
| Exact Rust 1.98.1 Clippy | pass; all targets, all features, locked, `-D warnings`, two build jobs |
| Exact Rust 1.98.1 tests | pass; 60 tests, including four new TC-110 integration tests, run serially |
| Exact Rust 1.98.1 rustdoc | pass; workspace, all features, no dependencies, `-D warnings` |
| `cargo deny --locked check` | pass; advisories, bans, licenses, and sources; existing duplicate transitive versions remain warnings under policy |
| `cargo audit` | pass; 78 locked dependencies scanned against 1,243 advisories |
| Retained implementation parity | pass; complete, missing, duplicate, failed, drifted, and unavailable matrices agree with retained Python output |
| Adverse request/semantic cases | pass; closed fields and enums, malformed scalars, unknown protocol/host/scenario, 8 MiB ceiling, revisions, paths, counts, outcomes, decisions, matrix identities, and permutation invariance |
| Mutation probes | pass; removing missing-cell detection fails the order oracle, and removing terminal-pair validation admits an invalid paired decision |
| `make lint` | pass |
| `make test` | pass; content rights, 189 Python tests, and module validation |
| `make package-audit` | pass |
| Local Quire validation | pass; inherited duplicate-provider diagnostics only |
| Slice traceability | pass; TC-110, FR-017-AC-2, FR-017-CON-1, and FR-017-CON-3 use canonical `ix_trace_rs::trace` markers |
| Whole-port integration traceability | expected incomplete; 215/242 while host evaluation, qualification, cutover, final removal, and aggregate NFR-005 obligations remain pending |

## Review disposition

**PASS for the reviewed Rust implementation slice.** FR-017-AC-2's pure
validation and aggregation boundary is implemented. The retained Python/MJS
paths remain until the separately reviewed adapter, invocation cutover, and
FR-018 removal gates pass.
