---
id: SR-078
title: "Rust review of parsed containment and trace source audit"
type: SpecReview
analysis: code-review
scope: "FR-014-AC-4, FR-014-CON-1, FR-014-CON-2, NFR-005-AC-2, TC-101, TC-117; src/source_audit.rs; tests/source_audit.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/NFR-005"
    type: reviews
---

## Summary

The repository rules and the exact `agent-skills/rust-review/SKILL.md`
checklist were applied to the pure parsed-source audit and its repository
qualification harness. The implementation accepts one bounded caller-supplied
Rust source document, parses it with `syn`, emits a closed ordered finding
taxonomy, resolves lexical aliases by module and block scope, and performs no
filesystem, environment, child-program, network, clock, or persistence access.
The repository tests separately supply every first-party library module and
every first-party Rust test source.

This review does not represent syntax inspection as compiler name resolution.
Cross-document resolution, dependency policy, executable-path inventory,
Quire reconciliation, cutover, and final legacy removal remain separate gates.
Four implementation findings and one specification-boundary finding were
corrected; no open Rust finding remains in this slice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-167 | high | Closed: the first implementation collected imports into one file-global alias map, so an alias declared in one inline module could incorrectly affect a sibling. Alias maps now follow lexical file, module, and block scopes; declarations shadow outer aliases; sibling isolation and alias-resolution mutation evidence pass. | `src/source_audit.rs`; `tests/source_audit.rs`; FR-014-AC-4; TC-101 |
| FND-168 | high | Closed: the first repository harness discovered only top-level conventional external modules and could omit an external module declared inside an inline module or through an unsupported explicit path. Discovery now recurses through inline module topology and fails closed if an explicit `#[path]` module appears without resolver support. | `tests/source_audit.rs`; FR-014-AC-4; TC-101 |
| FND-169 | medium | Closed: flattened imports made grouped syntax such as `use ix_trace_rs::{trace};` indistinguishable from the required canonical import. Exact import recognition now accepts only the unaliased, unqualified, inherited-visibility `use ix_trace_rs::trace` syntax; grouped, aliased, qualified-attribute, missing, and malformed cases fail. | `src/source_audit.rs`; `tests/source_audit.rs`; NFR-005-AC-2; TC-117 |
| FND-170 | high | Closed through `/specify`: a per-document AST cannot prove compiler name resolution across source documents. The specification and matrix now promise lexical alias resolution only and retain exact compilation, dependency policy, and the final executable-path gate for whole-program containment. | `spec/functional/FR-014-versioned-rust-boundary.md`; `spec/non-functional/NFR-005-rust-containment-and-traceability.md`; TC-101 |
| FND-171 | medium | Closed: deterministic ordering and the exact source-size boundary were not independently asserted. Tests now prove function/category/capability ordering, admit exactly 2,097,152 bytes, refuse the next byte, and assert stable codes for oversized, non-UTF-8, and invalid-syntax inputs. | `src/source_audit.rs`; `tests/source_audit.rs`; FR-014 Outputs and Error Conditions |
| FND-172 | low | No open Rust finding remains. Production code has no unsafe block, panic, lint suppression, unchecked numeric cast, I/O import, ambient state, async/lock surface, or unbounded input. Every new requirement test uses the canonical bare `ix_trace_rs::trace` marker. | `src/source_audit.rs`; `tests/source_audit.rs`; NFR-005 |

## Gate results

| Gate | Result |
| --- | --- |
| Exact Rust 1.98.1 formatting | pass; inherited vendored ix-trace-rs nightly-option warnings remain visible in direct formatting runs |
| Exact Rust 1.98.1 Clippy and compile | pass; all targets/all features/locked, `-D warnings`, two build jobs |
| Exact Rust 1.98.1 tests | pass; 83 tests, including eight new source-audit tests |
| Exact Rust 1.98.1 rustdoc | pass; workspace/all features/no dependencies/locked, `-D warnings` |
| `cargo deny --locked check` | pass; advisories, bans, licenses, and sources; existing duplicate transitive versions remain warnings under policy |
| `cargo audit` | pass; 140 locked dependencies scanned against 1,243 advisories |
| Adverse boundaries | pass; comments/literals, direct and lexical aliases, sibling scopes, exact and grouped trace imports, qualified and malformed attributes, absent TC/AC literals, stable ordering, exact/over ceiling, non-UTF-8, and invalid syntax |
| Mutation probes | pass; disabling exact trace-import recognition, disabling alias expansion, and rejecting the exact source ceiling each fail the owning test |
| `make lint` | pass |
| `make test` | pass; content rights, 189 Python tests, and retained module validation |
| `make package-audit` | pass |
| Local Quire validation | pass; inherited duplicate-provider diagnostics only |
| Slice traceability | pass; TC-101, TC-117, FR-014-AC-4, FR-014-CON-1, FR-014-CON-2, and NFR-005-AC-2 use canonical `ix_trace_rs::trace` markers |
| Whole-port integration traceability | expected incomplete; 225/248 and 112/121 test cases while host integration, cutover, final removal, and aggregate NFR-005 obligations remain pending |

## Mutation evidence

- Replacing exact `ix_trace_rs` import recognition with a non-existent root
  fails the repository TC-117 census on the first Rust source.
- Disabling lexical alias expansion removes the expected filesystem finding
  and fails the TC-101 alias test.
- Changing the source ceiling from `>` to `>=` rejects the exact 2,097,152-byte
  input and fails the boundary test.

## Review disposition

**PASS for the reviewed Rust implementation slice.** TC-101 and TC-117 are
implemented at their explicitly bounded source-audit layer. Whole-program
containment and final port completion remain governed by the still-pending
compiler, dependency, executable-path, reconciliation, cutover, and removal
gates.
