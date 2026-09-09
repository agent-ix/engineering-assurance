---
id: SR-053
title: "Rust review of version-policy fixture class coverage"
type: SpecReview
analysis: code-review
scope: "tests/evidence_parity.rs, tests/test_evidence.py, engineering_assurance/fixtures/evidence-version-policy.json"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---

## Summary

The review applied `agent-skills/rust-review/SKILL.md` to the focused TC-103
remediation. The Rust and retained Python gates now compare fixture classes
with the same requirement-owned exact set, while the fixture continues to
record the individual prior and accepted outcomes outside both implementations.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-099 | low | No open Rust finding remains. Reclassifying the only `wildcard-component` row as `immutable-x-metadata` fails both language gates with the missing class named; restoring it passes. The Rust test retains the canonical bare ix-trace-rs import and `TC-103`/`FR-015-AC-3` trace attribute. | `tests/evidence_parity.rs:6`; `tests/evidence_parity.rs:451`; `tests/evidence_parity.rs:465`; `tests/test_evidence.py:189`; `engineering_assurance/fixtures/evidence-version-policy.json:29` |

## Review checks

- The hard-coded expected class set is independent of the fixture under test;
  deleting or renaming the last row in a required class cannot move the oracle
  with the data.
- `BTreeSet` makes diagnostic ordering deterministic and adds no production
  allocation or public API surface.
- Test-only `expect` calls name malformed-fixture invariants; no caller input,
  panic, unsafe, allow, async, lock, wire, or persistence surface changed.
- Every changed behavior remains traced by the existing canonical
  `#[trace("TC-103", "FR-015-AC-3")]` attribute.

## Gate results

| Gate | Result |
| --- | --- |
| Exact toolchain | pass: `rustc 1.98.1 (48a229cea 2026-09-01)` and `cargo 1.98.1` |
| Missing-class mutation | pass: the deliberate mutation failed both Rust and Python focused gates; restored fixture passes |
| `cargo fmt --all --check` | pass; repository rustfmt emits its existing stable-channel warnings for nightly-only grouping options |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | pass with two build jobs |
| `cargo test --locked --all-features` | pass: 21 tests |
| `RUSTDOCFLAGS=-Dwarnings cargo doc --locked --no-deps --all-features` | pass |
| `cargo deny check` | pass after refreshing the writable advisory database: advisories, bans, licenses, and sources |
| `make lint` | pass |
| `make test` | pass: 188 tests, zero skipped, from a sibling checkout where every campaign repository is present |
| `make package-audit` | pass |
| `make validate-docs` | pass with inherited duplicate-provider diagnostics |
| `make integration-traceability` | expected withheld state: 207/262 until later PLAN-003 tasks; no completion claim |

## Review disposition

The focused Rust review passes with no open finding. This closes the final
re-review recommendation from PR #27 without claiming that the unfinished
repository-wide Rust migration or traceability aggregate is complete.
