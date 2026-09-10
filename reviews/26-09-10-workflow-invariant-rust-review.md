---
id: SR-064
title: "Rust review of the workflow-invariant evaluator"
type: SpecReview
analysis: code-review
scope: "FR-016-AC-2 and FR-016-CON-3; src/workflow_invariants.rs; workflow-invariants CLI; TC-106"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-016"
    type: reviews
---

## Summary

The repository rules and `agent-skills/rust-review/SKILL.md` were applied to
the additive Rust port of the eleven existing workflow invariants. The final
surface is a deterministic, I/O-free typed evaluator plus a bounded CLI
transport. It does not implement ix-flow lifecycle or gate state, authorize a
foreign-language host bridge, remove the retained JavaScript provider, or
expand the invariant semantics. Six findings were corrected during review; no
open finding remains in this slice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-122 | high | Closed: the first wire structs camel-cased every intake, architecture, promotion, change, and artifact field even though only `interviewId` is camel-case in the canonical ix-flow records. Valid snapshots therefore refused `requested_artifacts`, `source_revision`, and related fields before evaluation. The closed structs now rename only `interviewId` and explicitly retain the canonical snake-case names. | `src/workflow_invariants.rs`; FR-016-AC-2; TC-106 |
| FND-123 | high | Closed: architecture and change evaluators could equate an absent binding subject with an absent evidence subject and return passed. Every subject lookup now requires a present non-empty request path or revision before evidence matching; adverse probes cover the prior false-pass shapes. | `src/workflow_invariants.rs`; `tests/workflow_invariants_parity.rs`; FR-016-AC-2; TC-106 |
| FND-124 | medium | Closed: workflow names were independently hand-matched during request validation and terminal-gate selection, with a catch-all empty transition set at the semantic boundary. A closed `WorkflowName` enum now validates once and selects terminal transitions exhaustively. | `src/workflow_invariants.rs`; FR-016-AC-2 |
| FND-125 | medium | Closed: the inherited CLI transport accumulated stdin without a ceiling, so an oversized request could exhaust memory before the typed evaluator refused it. Both CLI capabilities now stop after 8 MiB plus one byte and emit the capability's existing typed request error; TC-106 exercises the limit. | `src/main.rs`; `tests/workflow_invariants_cli.rs`; FR-016-AC-2 |
| FND-126 | medium | Closed: the first parity fixture combined every item family under one workflow, proving function parity but not parity over valid workflow-shaped projections. The test now derives four closed projections matching the assurance-intake, architecture-evaluation, measurement-promotion, and change-assurance populations and covers all eleven names. | `tests/fixtures/workflow-invariants/passing-projection.json`; `tests/workflow_invariants_parity.rs`; FR-016-AC-2; TC-106 |
| FND-127 | medium | Closed: exception-request selection used `Option::and_then`, so an earlier interview item with a missing `exceptions_expected` field fell through to a later request family; the retained provider selects the first interview item regardless of that field. The Rust selection now preserves the first-item rule, and the focused boundary is differentially asserted. | `src/workflow_invariants.rs`; `tests/workflow_invariants_parity.rs`; FR-016-AC-2; TC-106 |
| FND-128 | low | No open Rust finding remains. Production code contains no unsafe block, panic, unchecked cast, lint suppression, filesystem, environment, process, network, system-clock, async, lock, or retry surface. Request and result payloads are closed typed structures; output codes are closed enums; only the CLI owns bounded stream I/O. | `src/workflow_invariants.rs`; `src/main.rs`; FR-016-CON-3 |

## Gate results

| Gate | Result |
| --- | --- |
| Exact Rust 1.98.1 formatting | pass — `cargo +1.98.1 fmt -- --check` |
| Exact Rust 1.98.1 Clippy | pass — workspace, all targets, all features, locked, `-D warnings`, two build jobs |
| Exact Rust 1.98.1 check | pass — workspace, all targets, all features, locked, two build jobs |
| Exact Rust 1.98.1 tests | pass — 33 tests, including six TC-106 Rust tests |
| Exact Rust 1.98.1 rustdoc | pass — workspace, all features, no dependencies, `-D warnings` |
| `cargo deny --locked check` | pass — advisories, bans, licenses, and sources |
| `cargo audit` | pass — 34 locked crate dependencies scanned against 1,243 advisories |
| Retained-JavaScript differential | pass — all eleven names across four valid workflow projections, ordered batches, failure codes, expiry and snapshot time boundaries |
| Fail-closed probes | pass — malformed JSON, unknown fields, empty/duplicate/unknown names, unknown workflow, invalid instant, absent binding subjects, and oversized CLI input |
| `make lint` | pass |
| `make test` | pass — 189 tests |
| `make package-audit` | pass |
| Local Quire validation and TC-106 reconciliation | pass — inherited duplicate-provider diagnostics and intentionally adverse embedded corpus findings remain outside this slice |

## Review disposition

**PASS for FR-016-AC-2 and FR-016-CON-3.** The Rust evaluator is additive.
FR-016-AC-4 remains pending because ix-flow 0.2.3 exposes only its JavaScript
provider interface and the owner has not approved a foreign-language bridge.
