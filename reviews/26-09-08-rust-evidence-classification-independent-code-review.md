---
id: SR-040
title: "Code review — Rust evidence classification identity parity (independent)"
type: SpecReview
analysis: code-review
scope: "PR #27; src/evidence.rs; src/lib.rs; tests/evidence_parity.rs; Cargo.toml; engineering_assurance/evidence.py as the retained reference"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: "reviews"
---

# SR-040: Code review — Rust evidence classification identity parity (independent)

## Summary

Independent review of PR #27 against the retained Python classifier, run after
SR-039. All declared gates reproduce green (fmt, clippy `-D warnings`, 19 tests,
`cargo deny`), and the branch-set, error-ordering, and state-distinction parity
claims hold. A differential probe over 33 JSON output shapes outside the
committed 22-case fixture found that the Rust and Python classifiers assign
**different SHA-256 identities to the same producer output** whenever that
output contains an integer outside the `i64`/`u64` range.

## Verdict

**FAIL** — FND-084 is a silent identity divergence in the digest domain FR-015
forbids reinterpreting, and the parity gate cannot currently see it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-084 | high | Producer output containing an integer outside `i64`/`u64` gets a different evidence identity in Rust than in the retained reference. `serde_json` is pinned without `arbitrary_precision`, so `{"v":1234567890123456789012345}` parses to `f64` and the legacy encoder emits `{"v":1.2345678901234568e+24}` (digest `a933d40d…`), while Python digests the exact integer bytes (digest `afbf3261…`). Also reproduced for `18446744073709551616` and `123456789012345678901`. This is EC-013 realised and contradicts FR-015 "SHALL NOT use a different canonicalization algorithm to rewrite or silently reinterpret an existing identity". | `src/evidence.rs:151`; `Cargo.toml:22`; `engineering_assurance/evidence.py:141` |
| FND-085 | medium | Non-finite and out-of-`f64`-range output is digestible by the reference but unrepresentable in Rust. Python `json.dumps` defaults to `allow_nan=True` and digests `{"v":NaN}` / `{"v":Infinity}`; `serde_json` rejects `1e309` at parse time, so the attempt never reaches `classify_producer`. The parity claim states no boundary for this class and no test records it. | `src/evidence.rs:151`; `engineering_assurance/evidence.py:141` |
| FND-086 | medium | The identity gate's entire differential corpus is one hand-written 22-case list whose only integers are `3` and small array elements. FR-015-AC-1 requires byte-identity "over the accepted corpus and focused fictional cases"; the Rust TC-100 exercises fictional cases only, with no generated coverage of JSON value space, so FND-084 and FND-085 both sit outside the gate. | `tests/evidence_parity.rs:265` |
| FND-087 | low | `VersionIdentity::errors` normalises with `str::to_lowercase` where the reference uses `str.casefold`. The two disagree on case-folding inputs (`ß`, `ς`), so a version string can be judged mutable on one side and immutable on the other. | `src/evidence.rs:118`; `engineering_assurance/evidence.py:47` |
| FND-088 | low | The `'x'` mutability marker is substring-matched against the whole lowercased version, so an immutable build such as `1.2.3+linux-x86_64` is rejected as `identity-version-mutable`. Faithfully ported from the reference, and cemented by this PR; it belongs to the shared semantics, not to Rust. | `src/evidence.rs:123`; `engineering_assurance/evidence.py:48` |
| FND-089 | low | `canonical_json_digest_fixture_is_stable` carries a `TC-100` trace but is the only test in the crate without the repo's `tc_NNN_` name prefix, and two distinct tests share the `TC-102` tag. Neither breaks the scanner; both weaken by-name traceability. | `tests/evidence_parity.rs:293`; `tests/evidence_parity.rs:315` |

## What was verified as correct

- Branch order in `classify_producer` matches the reference exactly across
  producer-id, observation, applicability, invocation, outcome, output shape,
  output validity, governing provenance, and Quoin handoff, including the
  shared `producer-output-malformed` spelling for both output failures.
- `EvidenceEnvelope` retention on the invalid path (governing and
  `quoin_reference` kept; `next_action`, `owner`, `boundary_rationale` dropped)
  matches `_invalid`.
- Stable error strings and their emission order match, including the
  `{field}:{error}` governing prefix over the nine identities in reference
  order.
- The legacy number encoder is correct for the scientific-notation threshold,
  one-digit exponent padding, explicit `+` sign, negative zero, subnormals,
  and maximum finite `f64`; 28 of 33 probe shapes matched byte-for-byte.
- Key ordering is equivalent: `serde_json::Map` is a `BTreeMap` ordered by
  UTF-8 bytes, which is Python's `sort_keys` code-point order.
- The encoder's outside-string scanner never starts a number token on a
  literal (`true`/`null`), so no non-numeric byte is rewritten.
- Rust's type system makes the reference's `identity-missing` and
  `elapsed-invalid` type branches unreachable — strictly stronger, not a gap.
- `#![forbid(unsafe_code)]`, `missing_docs = "deny"`, no `unwrap`/`expect`/
  panic path in `src/`, no unchecked numeric cast, no recursion.
- `sha2 = "=0.11.0"` is exact-pinned with `default-features = false` and clears
  `cargo deny` advisories, bans, licenses, and sources.

## Gates executed

- `cargo +1.98.1 fmt --all -- --check` — pass (nightly-only import settings warn, as recorded).
- `cargo +1.98.1 clippy --all-targets --all-features -- -D warnings` — pass.
- `cargo +1.98.1 test --all-targets --all-features` — 19 passed.
- `make integration-traceability` — 207/262, as declared; FR-015-AC-1..AC-3 are backed.
- Differential probe, 33 output shapes against the retained reference — 5 mismatches (FND-084, FND-085).

## Recommended disposition

FND-084 is fixable inside this slice by enabling `serde_json/arbitrary_precision`
and rendering the retained integer token verbatim, or by refusing an output whose
numbers cannot round-trip rather than digesting a reinterpreted value. FND-085
needs a stated boundary in FR-015 either way: the reference can mint an identity
over bytes that are not RFC 8259 JSON, and Rust cannot. FND-086 should extend
TC-100 with generated cases before the next identity-bearing slice lands.
