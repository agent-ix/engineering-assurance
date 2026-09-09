---
id: SR-039
title: "Code review — Rust evidence availability classification"
type: SpecReview
analysis: code-review
scope: "Engineering Assurance Rust evidence availability, provenance validation, and legacy output identity slice; FR-004, FR-015-AC-1..AC-3"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-004"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---
# SR-039: Code review — Rust evidence availability classification

## Summary

Reviewed the additive Rust evidence classifier, immutable governing identities,
closed availability states, retained canonical output digest, and Rust/Python
differential tests using `agent-skills/rust-review/SKILL.md`. The implementation
is a pure library capability: it performs no observation, producer execution,
filesystem access, persistence, Quoin write, or CLI protocol handling. The
retained Python module remains only as the old side of the port parity gate.

## Verdict

**PASS** — no unresolved semantic, identity, panic, unsafe, dependency,
ownership, or test finding remains in this evidence-classification slice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-083 | high | **Closed:** the first Rust digest path used Serde's ordinary JSON number rendering. Python's retained canonical bytes use a different scientific-notation threshold and pad one-digit exponents, so values such as `1e-5` and `1e-7` would receive different SHA-256 identities. The Rust encoder now preserves the legacy Python spelling without changing canonicalization version, and TC-100 compares exact result/digest bytes over threshold, exponent, negative-zero, subnormal, maximum-finite, Unicode, control-escape, key-order, and nested cases. | `src/evidence.rs`; `tests/evidence_parity.rs`; FR-015-AC-1; TC-100 |

## Rust-review checklist

- `classify_producer` is deterministic, side-effect free, and read-only. It
  accepts typed caller observations and returns a classification; it does not
  execute a producer or persist an evidence record.
- Observed evidence requires a nonblank producer, a valid operator observation,
  object output declared valid by its owning contract, all nine immutable
  governing identities, and a Quoin handoff reference. Invalid attempts cannot
  receive an availability state or output digest.
- Observed, unavailable, not-computed, and not-applicable remain distinct.
  Zero, duplicate, conflicting, and unknown labels are refused.
- Validation order and stable error strings match the retained implementation
  across missing commands, invalid elapsed time/outcomes/exit codes, absent
  diagnostic categories, missing rationales/owners/output/provenance, mutable
  versions, malformed digests, and invalid Quoin handoffs.
- The legacy canonical encoder is linear over Serde's complete JSON bytes,
  operates outside quoted strings only, preserves UTF-8 and JSON escapes, and
  introduces no recursion, unsafe block, unchecked numeric cast, or production
  panic/unwrap/expect path.
- Every requirement-verifying Rust test imports `ix_trace_rs::trace` and uses a
  bare `#[trace("TC-...", "...-AC-...")]` attribute. TC-100, TC-102, and
  TC-103 report this capability as independent subcases, not as completion of
  the larger aggregate port criteria.
- `sha2` 0.11.0 is exact-pinned, requires Rust 1.85, and passes the repository's
  advisory, ban, license, and source policy on the qualified Rust 1.98.1 toolchain.

## Gates executed

- `cargo +1.98.1 fmt --all -- --check` — passed, with existing stable-channel
  warnings for nightly-only rustfmt import grouping settings.
- `cargo +1.98.1 clippy --all-targets --all-features -- -D warnings` — passed.
- `cargo +1.98.1 test --all-targets --all-features` — 19 passed.
- `RUSTDOCFLAGS="-D warnings" cargo +1.98.1 doc --workspace --all-features --no-deps --locked` — passed.
- `cargo deny check` — advisories, bans, licenses, and sources passed.
- `make lint` — passed.
- `make test` — 182 passed, 2 skipped; content-rights and module validation passed.
- `make package-audit` — passed.
- `make validate-docs` — passed with the existing ambient duplicate-provider warnings.
- `git diff --check` — passed.
- `make integration-traceability` — intentionally withheld at 207/262 for the
  staged port, up from 203/262; the new evidence subcases are backed while
  the remaining FR-014..FR-018 and NFR-005 work stays visibly unimplemented.

This review does not claim FR-015, #59, or the staged language port is complete.
Semantic validation, bounded projection, fixture generation, shared
observation/digest checks, and assurance-chain integration remain separate
implementation slices.
