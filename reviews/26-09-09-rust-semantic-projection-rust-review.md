---
id: SR-057
title: "Rust review of semantic validation, projections, and fixtures"
type: SpecReview
analysis: code-review
scope: "Semantic validation, projection, and fixture Rust slice; src/semantics/; tests/semantics_parity.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---

## Summary

The repository Rust rules and `agent-skills/rust-review/SKILL.md` were applied
to the additive semantic implementation. The final surface is an I/O-free Rust
library over typed, versioned semantic references and bounded projections. A
deeper re-review found that the first passing implementation had concentrated
four responsibilities and hand-walked PGM-01 JSON in one 1,678-line module;
that architectural finding is now remediated rather than waived. The library
does not persist evidence, discover a corpus checkout, execute generated
foreign-language fixtures, or expose a new CLI protocol. Four review findings
were corrected in the first pass, two architectural findings in the deeper
pass, and the independent-review mapping and audit findings in the final pass;
no open Rust finding remains.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-120 | high | Closed: an empty legacy schema or record identity could produce an `Ok` compatibility view that violated the committed output schema. Empty and scalar identities now retain Python-compatible normalization, and every return path validates the typed output invariants plus exact source digest before returning. Five adverse identity cases are differentially asserted. | `src/semantics/pgm01.rs`; `tests/semantics_parity.rs`; FR-015-AC-1; TC-100; TC-103 |
| FND-118 | high | Closed: the initial implementation put semantic validation, ownership checks, report rendering, PGM-01 decoding, and fixture generation in one 1,678-line module. Those responsibilities now have separate modules behind the unchanged `engineering_assurance::semantics` facade. | `src/semantics/mod.rs`; `src/semantics/report.rs`; `src/semantics/pgm01.rs`; `src/semantics/fixtures.rs` |
| FND-121 | medium | Closed: the negative-capability audit listed four source files and searched import-path substrings, so a fifth module or braced process import escaped. It now discovers every Rust source recursively, rejects the capability identifiers independent of import spelling, and proves three braced-import mutants are detected. | `tests/semantics_parity.rs`; FR-015-AC-4; TC-104 |
| FND-113 | medium | Closed: the first PGM-01 implementation converted retained byte counts through `u64`, so a valid non-negative Python integer above `u64::MAX` became `unreadable`. The mapper now validates the exact arbitrary-precision JSON integer token, preserves large values, and retains Python's `-0` → `0` behavior; both cases are asserted. | `src/semantics/pgm01.rs`; `tests/semantics_parity.rs`; FR-015-AC-1; TC-100; TC-103 |
| FND-114 | medium | Closed: the first generator embedded the pinned qa-corpus into the production crate at compile time. The pure API now requires exact corpus bytes from its caller, so it neither discovers workstation state nor hides qualification data in the product artifact. | `src/semantics/fixtures.rs`; `tests/semantics_parity.rs`; FR-015-AC-1; FR-015-CON-1; TC-100 |
| FND-115 | medium | Closed: the static inert-fixture audit treated an unreadable script/workflow as empty text, allowing an I/O failure to erase the audited population and pass. Every selected audit input now fails the test when unreadable. | `tests/semantics_parity.rs:389`; FR-015-AC-4; TC-104 |
| FND-119 | medium | Closed: PGM-01 v1/v2 were decoded by repeated string-key lookup and nested type branches. Typed per-version records, retained-stream structs, and closed state/disposition enums now perform the mapping. Generic JSON remains only for source-version classification, explicitly opaque legacy fields, arbitrary-precision integers, and the public preserved-value contract. | `src/semantics/pgm01.rs`; FR-015-AC-1; FR-015-AC-3; TC-100; TC-103 |
| FND-116 | low | Closed: report claim statuses and evidence relations were free strings guarded by later validation. Closed enums now make invalid states unconstructable through the typed API and `deny_unknown_fields` remains on every wire struct. | `src/semantics/report.rs`; FR-015-AC-1; TC-100 |
| FND-117 | low | No open Rust finding remains. Production code contains no panic, unchecked cast, unsafe block, lint suppression, filesystem/process/environment access, or unbounded external retry/async/lock lifecycle. The single semantic error boundary exposes a stable category and direct diagnostic accessor. | `src/semantics/mod.rs`; `src/semantics/pgm01.rs`; `src/semantics/report.rs`; `src/semantics/fixtures.rs` |

## Gate results

| Gate | Result |
| --- | --- |
| Exact Rust 1.98.1 toolchain | pass — `rustc 1.98.1 (48a229cea 2026-09-01)` |
| `cargo fmt --all -- --check` | pass; repository `rustfmt.toml` emits its pre-existing stable-channel warning for nightly-only import grouping |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | pass with two build jobs |
| `cargo check --workspace --all-targets --all-features --locked` | pass with two build jobs |
| `cargo test --workspace --all-targets --all-features --locked` | pass — 25 tests, including seven semantic integration tests |
| `RUSTDOCFLAGS=-D warnings cargo doc --workspace --all-features --no-deps --locked` | pass |
| `cargo deny check --disable-fetch` | pass — advisories, bans, licenses, and sources |
| Retained-Python differential | pass — PGM-01 v1/v2 mappings, five adverse identity cases, and JSON/Markdown report bytes agree |
| Generated fixture comparison | pass — all seven committed `.py`, `.ts`, and `.rs` inert projections agree |
| Ownership/adverse mutations | pass — authority confusion, missing/wrong links, source-version skew, aggregate verdict insertion, unsupported PGM version, tamper, malformed bytes, and arbitrary-precision integer boundaries are discriminating |
| `make lint` | pass |
| `make test` | pass — 186 tests and 2 location-dependent sibling-checkout skips from the linked worktree; they are not represented as optional-source evidence |
| `make package-audit` | pass |
| Local Quire validation | pass; inherited duplicate-provider diagnostics only |

## Review disposition

**PASS for the additive semantic Rust slice.** The aggregate port remains
explicitly incomplete, retained Python remains the
same-revision differential oracle, and independent implementation review is
still required before merge or downstream consumption.
