---
id: SR-037
title: "Code review — Rust package foundation"
type: SpecReview
analysis: code-review
scope: "Engineering Assurance Rust foundation; FR-014-AC-1, NFR-005-AC-1, TC-096, TC-116"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/NFR-005"
    type: reviews
---
# SR-037: Code review — Rust package foundation

## Summary

Reviewed the initial additive Cargo package using
`agent-skills/rust-review/SKILL.md` and the repository's `AGENTS.md`. The scope
is deliberately limited to the reusable library target, native CLI target,
exact Rust toolchain, dependency policy, canonical trace dependency, and the
two implemented package-boundary criteria. It does not claim any migrated
Engineering Assurance capability or legacy-path removal.

## Verdict

**PASS** — no unresolved Rust, test, integrity, boundary, or packaging finding
remains in this foundation slice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-075 | high | **Closed:** the preserved scaffold inherited Rust 1.75. The reviewed specification, package metadata, and exact toolchain now use Rust 1.98.1 and edition 2024; the real gates execute on that compiler. | `Cargo.toml`; `rust-toolchain.toml`; NFR-005-AC-1; TC-116 |
| FND-076 | medium | **Closed:** adding `Cargo.lock` to the complete content-rights scan initially rejected Cargo's required registry-source metadata. The exception now permits only the exact crates.io index URL and only in `Cargo.lock` and `deny.toml`; the same URL remains rejected elsewhere. | `scripts/check_content_rights.py`; `tests/test_content_rights.py` |
| FND-077 | low | **Closed:** TC-096 hard-coded package version `0.1.0`, so a legitimate version bump would fail an interface test unrelated to that criterion. It now compares CLI output with Cargo-provided package identity constants while retaining the required package name assertion. | `tests/package_boundary.rs` |

## Rust-review checklist

- The library is I/O-free and exports only documented package identity values;
  the CLI owns its process boundary.
- All first-party Rust targets forbid unsafe code through workspace lint policy;
  no unsafe block, panic path, unchecked cast, recursion, asynchronous task,
  lock, filesystem path, wire decoder, or unbounded collection exists in this
  slice.
- Both Rust requirement tests import `ix_trace_rs::trace` and use the bare
  canonical attribute form. TC-096 and TC-116 resolve in the test matrix and
  appear as backed in Quire's trace report.
- Test `expect` calls are confined to the integration test and name failures of
  the Cargo-provided executable/UTF-8 contract; no caller-controlled library
  input can panic.
- `ix-trace-rs` is an exact reviewed Git submodule and test-only path
  dependency. The root workspace excludes the proc-macro's independent package
  while the exact pinned source still compiles under Rust 1.98.1.
- The content-rights adjustment is temporary parity support for the retained
  Python gate; it adds no new non-Rust assurance authority.

## Gates executed

- `cargo fmt -- --check` — passed.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  — passed.
- `cargo check --workspace --all-targets --all-features --locked` — passed.
- `cargo test --workspace --all-targets --all-features --locked` — 2 passed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked`
  — passed using an isolated writable target directory.
- `cargo deny check` — advisories, bans, licenses, and sources passed; it reports
  one non-failing unmatched `Unicode-3.0` allowance.
- `make lint` — passed.
- `make test` — 182 passed, 2 skipped; content-rights and module validation
  passed.
- `make package-audit` — passed.
- `make validate-docs` — passed with the existing duplicate-provider warnings.
- `git diff --check` — passed.

`make integration-traceability` remains red because the staged migration has
31 unimplemented TC rows and the existing checker still expects the former
68-test population. It reports TC-096 and TC-116 as backed. This review does not
convert that expected incomplete migration state into a passing release claim.
