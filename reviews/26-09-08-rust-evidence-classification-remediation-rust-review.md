---
id: SR-043
title: "Rust review — evidence identity remediation"
type: SpecReview
analysis: code-review
scope: "PR #27 remediation; Cargo.toml; src/evidence.rs; engineering_assurance/evidence.py; tests/evidence_parity.rs; tests/test_evidence.py; FR-015; TC-100/102/103"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---

# SR-043: Rust review — evidence identity remediation

## Summary

The repaired slice passes the repository Rust conventions and the requested `/rust-review` checklist. Arbitrary-precision integers retain their value instead of crossing `f64`; decimal/exponent inputs are normalized through the finite retained Python float domain; invalid numeric inputs mint no digest; and governing versions now use one reviewed exact-token policy in Rust and the retained Python reference.

## Verdict

**PASS for the PR #27 implementation slice.** Merge remains gated on independent re-review of the current head. The repository-wide port remains incomplete.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-096 | low | No unresolved Rust code finding remains in the repaired diff: no production panic/unsafe/allow surface was added, public evidence helpers now have requirement ownership and direct inspection, numeric traversal is iterative and bounded by the already-materialized JSON input, and all changed wire structs retain `deny_unknown_fields`. | `src/evidence.rs`; `tests/evidence_parity.rs`; FR-015 |

## Independent finding dispositions

| Finding | Disposition |
| --- | --- |
| FND-084 | Fixed: `serde_json/arbitrary_precision` is enabled and accepted/generated tests prove integers beyond both native 64-bit ranges retain the Python identity digest. |
| FND-085 | Fixed: FR-015 defines finite RFC 8259 identity input; Rust and retained Python both refuse non-finite/overflow-to-non-finite values before digest creation. |
| FND-086 | Fixed: TC-100 now runs three accepted producer fixtures and a deterministic generated integer/value corpus in one differential process. |
| FND-087 | Fixed as a semantics correction rather than a case-fold imitation: both implementations reject non-printable/non-ASCII version tokens and use ASCII case-insensitive matching only after that boundary. |
| FND-088 | Fixed in both implementations: `x` is mutable only as a version-core wildcard component; immutable metadata such as `1.2.3+linux-x86_64` is accepted. |
| FND-089 | Fixed: the digest fixture has the `tc_100_` name and state-label/public-state coverage is one TC-102 test. Multiple TC-100/TC-103 functions are separately named subcases of their aggregate matrix rows. |
| FND-091 | Fixed: aggregate rows remain pending but now name the backed compatibility/evidence/CLI subcases explicitly. |
| FND-092 | Fixed: the evidence identity test consumes the three accepted `corpus/compatibility/producers` fixtures in addition to generated adverse values. |
| FND-093 | Fixed: FR-015 owns state spellings, exact-one untyped-label validation, result validity, and the error accessor; `EvidenceValidationError::message` is public. The typed classifier constructs one enum state directly and therefore does not route through an untyped-label parser. |

## Rust review checklist

- **Project conventions:** `AGENTS.md`, `Cargo.toml` workspace lints, and `deny.toml` were applied. No repository-specific Rust-style skill exists.
- **Idioms/API:** public types remain typed and documented; validation errors are structured at the crate boundary; no new stringly error return, unchecked cast, catch-all enum match, or unnecessary trait seam was added.
- **Panic/unsafe/integrity:** production `src/` adds no `unwrap`, `expect`, `panic!`, `unsafe`, `allow`, TODO, stub, or lint weakening. Test-only `expect` calls name fixture/process invariants.
- **Wire/numeric boundary:** external structs retain `#[serde(deny_unknown_fields)]`; arbitrary integer values never cross `f64`; non-finite numeric identity is rejected; version characters and wildcard semantics are explicit and differentially tested.
- **Resource/concurrency:** numeric inspection uses an iterative stack bounded by the parsed value; the differential test sends the accepted corpus over a pipe rather than argv and starts one retained-reference process; no async, lock, clock, network, or fixed-port behavior was added.
- **Test quality:** requirement tests use `use ix_trace_rs::trace` plus bare attributes, execute the real classifier and retained reference, and include accepted corpus, generated numeric boundaries, mutation/refusal cases, and direct public-API assertions.

## Gates executed

- `cargo +1.98.1 fmt --all -- --check` — pass; the repository's nightly-only stable-rustfmt settings emit warnings but do not alter the result.
- `cargo +1.98.1 clippy --locked --all-targets --all-features -j 2 -- -D warnings` — pass.
- `cargo +1.98.1 test --locked --all-targets --all-features -j 2` — 21 passed.
- `RUSTDOCFLAGS='-D warnings' cargo +1.98.1 doc --locked --workspace --all-features --no-deps -j 2` — pass.
- `cargo +1.98.1 deny check` — advisories, bans, licenses, and sources pass.
- `make lint` — pass.
- `make test` — content rights pass; 186 Python tests pass, 2 skip; manifest validation passes.
- `make package-audit` — pass.
- `make validate-docs` — pass with the recorded ambient duplicate-provider diagnostics.
- `make integration-traceability` — expected refusal for the incomplete repository-wide port; no completion claim.
