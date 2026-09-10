---
id: SR-074
title: "Rust review of pure package-membership classification"
type: SpecReview
analysis: code-review
scope: "FR-017-AC-6, FR-017-CON-3, TC-120; src/package_membership.rs; tests/package_membership_parity.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

## Summary

The repository rules and the exact `agent-skills/rust-review/SKILL.md`
checklist were applied to the additive pure Rust package-membership classifier.
The implementation validates one bounded exact allowlist, retains it in a
deterministically ordered policy type, and returns a closed ordered taxonomy for
invalid, duplicate, unexpected, and missing observations. It performs no
archive decoding, member-kind inference, package construction, installation,
filesystem, environment, child-program, network, clock, or distribution-format
selection. Three review findings were corrected; no open Rust finding remains
in this slice.

This review does not claim the archive/package adapter, combined package/rights/
manifest/integration gate, distribution format, invocation cutover, or legacy
removal.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-158 | medium | Closed: the first parity population did not exercise the specified case-sensitive comparison or uninterpreted archive escaping, so an implementation that folded case or decoded `%2F` could pass. Both mismatches now run through the retained Python differential and the typed Rust findings. | `tests/package_membership_parity.rs`; FR-017 Behavior; TC-120 |
| FND-159 | medium | Closed: the initial permutation check reversed only observed members even though the criterion governs both the expected allowlist and observed sequence. The test now rebuilds a policy from the reversed expected input and compares both reversed inputs with the canonical result. | `tests/package_membership_parity.rs`; FR-017-AC-6; TC-120 |
| FND-160 | low | Closed: the first unsafe-path non-disclosure assertion asked whether an error string contained the empty input path, which is necessarily true for every string. The empty-path case now relies on the exact path-free error variant while every non-empty rejected path is checked against rendered diagnostics. | `tests/package_membership_parity.rs`; FR-017-AC-6 |
| FND-161 | low | No open Rust finding remains. Production code has no unsafe block, panic, unchecked numeric cast, lint suppression, untyped JSON walking, I/O import, ambient state, async/lock surface, or unbounded comparison state. Every new requirement test uses the canonical bare `ix_trace_rs::trace` marker. | `src/package_membership.rs`; `tests/package_membership_parity.rs`; NFR-005 |

## Gate results

| Gate | Result |
| --- | --- |
| Exact Rust 1.98.1 formatting | pass; inherited vendored ix-trace-rs nightly-option warnings remain visible in direct formatting runs |
| Exact Rust 1.98.1 Clippy and compile | pass; workspace/all targets/all features/locked, `-D warnings`, two build jobs |
| Exact Rust 1.98.1 tests | pass; 68 tests, including four new TC-120 integration tests |
| Exact Rust 1.98.1 rustdoc | pass; workspace/all features/no dependencies/locked, `-D warnings` |
| `cargo deny --locked check` | pass; advisories, bans, licenses, and sources; existing duplicate transitive versions remain warnings under policy |
| `cargo audit` | pass; 83 locked dependencies scanned against 1,243 advisories |
| Retained implementation parity | pass; exact, empty, extra, missing, case-sensitive, and uninterpreted-escape member sets agree with Python `member_mismatch` |
| Adverse boundary cases | pass; duplicate policy/observation, full unsafe-path population, 4,096/4,097-byte paths, 65,536/65,537-member populations, deterministic ordering, and non-disclosure |
| Mutation probes | pass; rejecting the exact maximum and weakening duplicate detection both fail TC-120 |
| `make lint` | pass |
| `make test` | pass; content rights, 189 Python tests, and module validation |
| `make package-audit` | pass |
| Local Quire validation | pass; inherited duplicate-provider diagnostics only |
| Slice traceability | pass; TC-120, FR-017-AC-6, and FR-017-CON-3 use canonical `ix_trace_rs::trace` markers |
| Whole-port integration traceability | expected incomplete; 219/246 and 109/120 test cases while combined qualification, host evaluation, cutover, final removal, and aggregate NFR-005 obligations remain pending |

## Mutation evidence

- Changing the expected-population comparison from `>` to `>=` makes the exact
  65,536-member admission case fail with `ExpectedPopulationTooLarge`.
- Changing duplicate detection from a count greater than one to a count greater
  than two removes the required duplicate finding and fails the canonical
  finding assertion.

## Review disposition

**PASS for the reviewed Rust implementation slice.** FR-017-AC-6's pure
membership classifier is implemented. The retained Python archive/build path
remains active during additive parity; TC-111 and FR-017-AC-3 remain pending.
