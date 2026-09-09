---
id: SR-057
title: "Rust review of semantic validation, projections, and fixtures"
type: SpecReview
analysis: code-review
scope: "TASK-016 additive Rust slice; src/semantics.rs; tests/semantics_parity.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---

## Summary

The repository Rust rules and `agent-skills/rust-review/SKILL.md` were applied
to the additive TASK-016 implementation. The final surface is an I/O-free Rust
library over typed, versioned semantic references and bounded projections. It
does not persist evidence, discover a corpus checkout, execute generated
foreign-language fixtures, or expose a new CLI protocol. Four review findings
were corrected before publication; no open Rust finding remains.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-113 | medium | Closed: the first PGM-01 implementation converted retained byte counts through `u64`, so a valid non-negative Python integer above `u64::MAX` became `unreadable`. The mapper now validates the exact arbitrary-precision JSON integer token, preserves large values, and retains Python's `-0` → `0` behavior; both cases are asserted. | `src/semantics.rs:1091`; `src/semantics.rs:1384`; `tests/semantics_parity.rs:229`; FR-015-AC-1; TC-100; TC-103 |
| FND-114 | medium | Closed: the first generator embedded the pinned qa-corpus into the production crate at compile time. The pure API now requires exact corpus bytes from its caller, so it neither discovers workstation state nor hides qualification data in the product artifact. | `src/semantics.rs:1540`; `src/semantics.rs:1547`; `tests/semantics_parity.rs:343`; FR-015-AC-1; FR-015-CON-1; TC-100 |
| FND-115 | medium | Closed: the static inert-fixture audit treated an unreadable script/workflow as empty text, allowing an I/O failure to erase the audited population and pass. Every selected audit input now fails the test when unreadable. | `tests/semantics_parity.rs:389`; FR-015-AC-4; TC-104 |
| FND-116 | low | Closed: report claim statuses and evidence relations were free strings guarded by later validation. Closed enums now make invalid states unconstructable through the typed API and `deny_unknown_fields` remains on every wire struct. | `src/semantics.rs:698`; `src/semantics.rs:730`; `src/semantics.rs:765`; FR-015-AC-1; TC-100 |
| FND-117 | low | No open Rust finding remains. Production code contains no panic, unchecked cast, unsafe block, lint suppression, filesystem/process/environment access, or unbounded external retry/async/lock lifecycle. The single semantic error boundary exposes a stable category and direct diagnostic accessor. | `src/semantics.rs:4`; `src/semantics.rs:35`; `src/semantics.rs:51`; `src/semantics.rs:57` |

## Gate results

| Gate | Result |
| --- | --- |
| Exact Rust 1.98.1 toolchain | pass — `rustc 1.98.1 (48a229cea 2026-09-01)` |
| `cargo fmt --all -- --check` | pass; repository `rustfmt.toml` emits its pre-existing stable-channel warning for nightly-only import grouping |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | pass with two build jobs |
| `cargo check --workspace --all-targets --all-features --locked` | pass with two build jobs |
| `cargo test --workspace --all-targets --all-features --locked` | pass — 27 tests, including six TASK-016 integration tests |
| `RUSTDOCFLAGS=-D warnings cargo doc --workspace --all-features --no-deps --locked` | pass |
| `cargo deny check --disable-fetch` | pass — advisories, bans, licenses, and sources |
| Retained-Python differential | pass — PGM-01 v1/v2 mappings and JSON/Markdown report bytes agree |
| Generated fixture comparison | pass — all seven committed `.py`, `.ts`, and `.rs` inert projections agree |
| Ownership/adverse mutations | pass — authority confusion, missing/wrong links, source-version skew, aggregate verdict insertion, unsupported PGM version, tamper, malformed bytes, and arbitrary-precision integer boundaries are discriminating |
| `make lint` | pass |
| `make test` | pass — 188 tests, zero skips, from the linked worktree |
| `make package-audit` | pass |
| QUOIN/Quire validation | pass; inherited duplicate-provider diagnostics only |
| Aggregate `make integration-traceability` | expected non-pass — 209/264 while #59 is incomplete; it also reports the pre-existing fixed `68/68` census expectation against the expanded 129-case matrix. This result is not represented as a passing slice gate. |

## Review disposition

**PASS for the additive TASK-016 Rust slice.** The aggregate migration and
traceability gates remain explicitly incomplete, retained Python remains the
same-revision differential oracle, and independent implementation review is
still required before merge or downstream consumption.
