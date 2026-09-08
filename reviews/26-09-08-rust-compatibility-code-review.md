---
id: SR-038
title: "Code review — Rust compatibility classifier"
type: SpecReview
analysis: code-review
scope: "Engineering Assurance Rust compatibility classification slice; FR-012, FR-014-AC-2/3/5, FR-015-AC-1"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-012"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---
# SR-038: Code review — Rust compatibility classifier

## Summary

Reviewed the additive pure compatibility classifier, versioned CLI command,
protocol schemas, and parity/boundary tests using
`agent-skills/rust-review/SKILL.md` and the repository's `AGENTS.md`. This scope
does not include environment observation, consumer artifact re-hashing,
forbidden-registry scanning, or the broader assurance-chain host behavior.
Those remain explicit migration work and the retained Python path is not yet
eligible for removal.

## Verdict

**PASS** — no unresolved Rust, wire-contract, resource-bound, parity, test, or
dependency finding remains in this compatibility-classification slice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-078 | high | **Closed:** the first draft imposed a 1 MiB request ceiling even though FR-014 and TC-121 set the accepted boundary at 64 MiB. Request and result limits now use 64 MiB, both protocol schemas declare the same value, and the CLI test accepts exactly 64 MiB and refuses 64 MiB plus one byte. | `src/compatibility.rs`; compatibility request/result schemas; TC-121 |
| FND-079 | high | **Closed:** an unknown protocol could echo an input-sized discriminator into an unbounded machine error, and a classification result could duplicate a large observed version beyond the result ceiling. Diagnostics are now UTF-8-boundary-truncated before serialization and complete results are buffered and size-checked before any stdout write. | `src/main.rs`; `CompatibilityResult::to_json_line`; TC-121 |
| FND-080 | high | **Closed:** the draft's aggregate outcome depended only on component versions, so a fully pinned but unaccepted matrix could report `compatible`. `versions_compatible`, `human_acceptance_recorded`, and `gate_satisfied` remain separate facts; only the conjunction produces the compatible aggregate and exit 0. | `src/compatibility.rs`; TC-082 |
| FND-081 | medium | **Closed:** the draft shortened the retained Python classifier's unknown-version reason, violating the accepted byte-parity requirement. The wording now matches, and TC-100 compares canonical Rust classification bytes with the retained Python reference across compatible, incompatible, unknown, and absent cases. | `src/compatibility.rs`; `tests/compatibility_parity.rs` |
| FND-082 | low | **Closed:** the first CLI assertion manually counted newline bytes and failed the strict Clippy gate. It now directly proves one terminal newline and no earlier newline without adding a dependency or lint exception. | `tests/compatibility_cli.rs` |

## Rust-review checklist

- The public library operation is deterministic and performs no filesystem,
  environment, subprocess, socket, or persistence I/O. The CLI alone owns
  stdin/stdout/stderr.
- Every external request and embedded matrix struct rejects unknown fields;
  foreign protocol versions, foreign components, duplicate observations,
  blank values, malformed JSON, and oversize requests fail with stable codes.
- Input collection is capped at one byte above the declared ceiling. Machine
  results are fully serialized and bounded before stdout; diagnostic text is
  bounded without splitting UTF-8.
- No unsafe block, production panic/expect/unwrap, unchecked numeric cast,
  recursion, async task, lock, path traversal surface, or unbounded host loop
  exists in the slice.
- All new Rust tests use the canonical bare `ix_trace_rs::trace` attribute and
  resolve to TC-080/081/082/085/098/099/100/121 and their accepted criteria.
  Test-only `expect` calls name harness invariants, not caller-controlled
  production assumptions.
- The migration parity test intentionally executes the retained Python
  classifier only as the old side of the additive old/new gate. It does not
  execute an inert generated fixture and must be deleted with the reference
  path only after the full governed removal gate passes.
- Exact direct dependencies are current compatible releases and introduce no
  advisory, license, source, duplicate-version, or unsafe first-party policy
  exception.

## Gates executed

- `cargo fmt -- --check` — passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed.
- `cargo test --workspace --all-targets --all-features` — 14 passed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` — passed.
- `cargo deny check` — advisories, bans, licenses, and sources passed.
- `make lint` — passed.
- `make test` — 182 passed, 2 skipped; content-rights and module validation passed.
- `make package-audit` — passed.
- full scoped Quire validation — passed with only existing duplicate-provider warnings.
- `git diff --check` — passed.

`make integration-traceability` remains intentionally red for the still-staged
overall migration. This slice raises backing from 195/262 at the foundation
checkpoint to 203/262, but FR-014's complete root/host/deadline/signal surface
and the remaining FR-015 through FR-018 capabilities are not implemented. This
review does not misreport partial migration evidence as a complete release
gate.
