---
id: SR-076
title: "Rust review of pure module-manifest qualification"
type: SpecReview
analysis: code-review
scope: "FR-017-AC-7, FR-017-CON-3, TC-121; src/manifest.rs; tests/manifest_parity.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

## Summary

The repository rules and the exact `agent-skills/rust-review/SKILL.md`
checklist were applied to the additive pure Rust module-manifest classifier.
The implementation accepts caller-supplied bytes, validates the candidate
manifest and skeleton frontmatter through caller-supplied authoritative JSON
Schemas, rejects ambiguous YAML, resolves schemas offline, asserts known
formats, and emits a closed deterministic finding taxonomy. It performs no
schema discovery, filesystem or environment access, child-program execution,
network access, clock access, or independent redefinition of the authoritative
manifest grammar. Four review findings were corrected; no open Rust finding
remains in this slice.

This review does not claim schema discovery, module-tree traversal, Quire
artifact validation, archive/package adaptation, distribution selection,
invocation cutover, publication policy, combined TC-111 completion, or legacy
removal.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-162 | medium | Closed: the initial public finding fields allowed callers to construct category/payload combinations the classifier can never emit. Fields are now private and exposed through read-only accessors, preserving serialization while making construction an implementation invariant. | `src/manifest.rs`; FR-017 Outputs |
| FND-163 | medium | Closed: the first adverse corpus did not prove offline reference refusal or enabled known-format assertions. It now supplies an unavailable remote reference and a date-time format mutation; both withhold through typed schema findings. The `jsonschema` dependency has default features disabled, uses `.offline()`, and pulls no HTTP client. | `src/manifest.rs`; `tests/manifest_parity.rs`; FR-017 Behavior; TC-121 |
| FND-164 | medium | Closed: `cargo deny` rejected MIT-0 from the new schema engine and Zlib from the already-present YAML graph because neither compatible permissive license was explicit in repository policy. Both SPDX licenses are now expressly admitted; advisories, bans, licenses, and sources pass. | `deny.toml`; `Cargo.lock`; NFR-005-AC-1 |
| FND-165 | low | Closed: the first fixture replacement helper could silently leave a mutation unapplied, the exact-heading check lacked a near-match case, and the fallback schema path embedded the review machine's home directory at compile time. The helper now proves its target exists, the scenario is decomposed into reviewable tests, a heading with an added suffix is rejected, and the fallback reads `HOME` only at test runtime. | `tests/manifest_parity.rs`; TC-121 |
| FND-166 | low | No open Rust finding remains. Production code has no unsafe block, panic, unchecked numeric cast, lint suppression, I/O import, ambient state, async/lock surface, or unbounded resource population. Every new requirement test uses the canonical bare `ix_trace_rs::trace` marker. | `src/manifest.rs`; `tests/manifest_parity.rs`; FR-017-CON-3; NFR-005 |

## Gate results

| Gate | Result |
| --- | --- |
| Exact Rust 1.98.1 formatting | pass; inherited vendored ix-trace-rs nightly-option warnings remain visible in direct formatting runs |
| Exact Rust 1.98.1 Clippy and compile | pass; all targets/all features/locked, `-D warnings`, two build jobs |
| Exact Rust 1.98.1 tests | pass; 75 tests, including seven new TC-121 integration tests |
| Exact Rust 1.98.1 rustdoc | pass; workspace/all features/no dependencies/locked, `-D warnings` |
| Dependency feature review | pass; `jsonschema` 0.56.0 is exact-pinned, default features are disabled, offline compilation is explicit, and no `reqwest` package is present |
| `cargo deny --locked check` | pass; advisories, bans, licenses, and sources; duplicate transitive versions remain warnings under existing policy |
| `cargo audit` | pass; 140 locked dependencies scanned against 1,243 advisories |
| Retained implementation parity | pass; the current authoritative module bundle is accepted by both retained Python and Rust validation |
| Adverse boundary cases | pass; malformed/duplicate/merge YAML, invalid/mismatching/offline schemas, identity, registry, resource, reference, frontmatter, locator, heading, non-disclosure, exact/over ceilings, and input permutation |
| Mutation probes | pass; disabling known-format checks, weakening exact-heading comparison, and rejecting the exact document maximum each fail TC-121 |
| `make lint` | pass |
| `make test` | pass; content rights, 189 Python tests, and retained module validation |
| `make package-audit` | pass |
| Local Quire validation | pass; inherited duplicate-provider diagnostics only |
| Slice traceability | pass; TC-121, FR-017-AC-7, and FR-017-CON-3 use canonical `ix_trace_rs::trace` markers |
| Whole-port integration traceability | expected incomplete; 221/248 and 110/121 test cases while combined qualification, host evaluation, cutover, final removal, and aggregate NFR-005 obligations remain pending |

## Mutation evidence

- Changing known-format assertion from enabled to disabled accepts the invalid
  date-time title and fails the schema/frontmatter TC-121 test.
- Weakening the exact heading comparison to substring matching accepts
  `## Decision Boundary appendix` and fails the heading TC-121 test.
- Changing the manifest byte ceiling from `>` to `>=` rejects the exact
  1,048,576-byte boundary and fails the resource-limit TC-121 test.

## Review disposition

**PASS for the reviewed Rust implementation slice.** FR-017-AC-7's pure
module-manifest classifier is implemented. The retained Python discovery and
module gate remain active during additive parity; TC-111 and FR-017-AC-3 remain
pending.
