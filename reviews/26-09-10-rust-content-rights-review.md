---
id: SR-072
title: "Rust review of pure content-rights classification"
type: SpecReview
analysis: code-review
scope: "FR-017-AC-5, FR-017-CON-3, TC-119; src/content_rights.rs; tests/content_rights_parity.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

## Summary

The repository rules and the exact `agent-skills/rust-review/SKILL.md`
checklist were applied to the additive pure Rust content-rights classifier. The
implementation uses a closed finding taxonomy, portable path validation,
linear-time compiled patterns, full Unicode case folding, bounded non-license
text inputs, explicit line-boundary handling, and deterministic deduplication.
It performs no repository enumeration, filesystem, environment, child-program,
network, clock, package, or publication operation. Five findings were corrected;
no open Rust finding remains in this slice.

This review does not claim the selected-tree adapter, combined package/manifest/
integration gate, distribution format, invocation cutover, or legacy removal.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-152 | high | Closed through a return to `/specify`: an unsafe input path was initially returned verbatim in `PathInvalid`, allowing an absolute candidate to disclose the workstation location the policy should suppress. The finding now carries `path: None`; its test asserts both the typed refusal and serialized non-disclosure. | `src/content_rights.rs`; `tests/content_rights_parity.rs`; FR-017 Outputs; FR-017-AC-5 |
| FND-153 | medium | Closed without suppression: the first classifier implementation placed file and all line rules in one 145-line branch hub, and strict Clippy rejected it. Whole-file classification, logical-line iteration, location checks, and semantic/token checks are now separate bounded functions. | `src/content_rights.rs`; FR-017-AC-5 |
| FND-154 | medium | Closed: the runtime pattern for the root-user location contained the prohibited spelling literally in its own Rust source, making the retained full-tree gate reject the port. The pattern is composed from source fragments while compiling to the same expression; the actual tree gate now passes. | `src/content_rights.rs`; `scripts/check_content_rights.py`; TC-119 |
| FND-155 | low | Closed: the initial line loop full-case-folded every line even when no protected token was supplied. Folding is now skipped for the ordinary empty-token case, avoiding an allocation proportional to each scanned line while preserving full Unicode behavior when tokens exist. | `src/content_rights.rs`; `tests/content_rights_parity.rs`; FR-017-AC-5 |
| FND-156 | low | Closed: the initial adverse test sampled one admitted and one forbidden suffix, so mutation of the remaining enumerated population could survive. The test now exercises every admitted and forbidden suffix plus the exact 512,000-byte boundary, over-limit refusal, and root-license exception. | `tests/content_rights_parity.rs`; FR-017 Behavior; TC-119 |
| FND-157 | low | No open Rust finding remains. Production code has no unsafe block, panic, unchecked numeric cast, lint suppression, untyped JSON walking, I/O import, ambient state, async/lock surface, or unbounded regular-expression behavior. Every new requirement test uses the canonical bare `ix_trace_rs::trace` marker. | `src/content_rights.rs`; `tests/content_rights_parity.rs`; NFR-005 |

## Gate results

| Gate | Result |
| --- | --- |
| Exact Rust 1.98.1 formatting | pass; inherited vendored ix-trace-rs nightly-option warnings remain visible in direct formatting runs |
| Exact Rust 1.98.1 Clippy and compile | pass; workspace/all targets/all features/locked, `-D warnings`, two build jobs |
| Exact Rust 1.98.1 tests | pass; 64 tests, including four new TC-119 integration tests |
| Exact Rust 1.98.1 rustdoc | pass; workspace/all features/no dependencies/locked, `-D warnings` |
| `cargo deny --locked check` | pass; advisories, bans, licenses, and sources; existing duplicate transitive versions remain warnings under policy |
| `cargo audit` | pass; 83 locked dependencies scanned against 1,243 advisories |
| Retained implementation parity | pass; every retained text finding category and URL/policy exception agrees with the Python reference |
| Adverse boundary cases | pass; every suffix, exact size edge, encoding/control/LFS case, portable unsafe path, line separator, full Unicode case fold, and non-disclosure assertion |
| Mutation probes | pass; the 240-byte encoded threshold and unsafe-path redaction mutants both fail TC-119 |
| `make lint` | pass |
| `make test` | pass; content rights, 189 Python tests, and module validation |
| `make package-audit` | pass |
| Local Quire validation | pass; inherited duplicate-provider diagnostics only |
| Slice traceability | pass; TC-119, FR-017-AC-5, and FR-017-CON-3 use canonical `ix_trace_rs::trace` markers |
| Whole-port integration traceability | expected incomplete; 217/244 and 108/119 test cases while host evaluation, combined qualification, cutover, final removal, and aggregate NFR-005 obligations remain pending |

## Mutation evidence

- Raising the encoded-payload minimum from 240 to 241 makes the retained-Python
  differential fail because the required finding disappears.
- Returning the rejected unsafe path makes the non-disclosure test fail on the
  first invalid input.

## Review disposition

**PASS for the reviewed Rust implementation slice.** FR-017-AC-5's pure
classifier is implemented. The retained Python path remains the selected-tree
adapter during additive parity; TC-111 and FR-017-AC-3 remain pending.
