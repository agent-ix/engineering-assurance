---
id: SR-044
title: "Code review — Rust evidence classification remediation at bad4a55"
type: SpecReview
analysis: code-review
scope: "PR #27 at bad4a55; src/evidence.rs; engineering_assurance/evidence.py; tests/evidence_parity.rs; FR-015; spec/tests.md; plan/PLAN-003"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: "references"
---

## Summary

FND-084 is genuinely closed. An independent differential probe of 58 JSON
shapes not present in this branch's corpus produced byte-identical identity
digests against the retained Python canonicaliser in every comparable case, and
all Rust gates reproduce green on the pinned 1.98.1 toolchain. The blocker is
not the Rust slice: two gates the PR's verification list reports as "skipped"
are gates that stood down, and one of them fails on real content when it
actually runs.

## Verdict

**FAIL** — one high finding. The evidence claim, not the code, is what fails.

## Gates run at `bad4a55`

Rust 1.98.1 (48a229cea 2026-09-01), the toolchain `rust-toolchain.toml` selects;
isolated `CARGO_TARGET_DIR`; submodules initialised.

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | pass |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | pass |
| `cargo test --locked --all-features` | pass — 21 tests (evidence_parity 7, package_boundary 2, protocol_schema 1, plus unit and compatibility suites) |
| `make lint` (ruff) | pass |
| `make test` | **2 failed, 186 passed** — see FND-001 |
| `make integration-traceability` | 207/262 with the declared unbacked rows; matches the PR exactly |

`make integration-traceability` reproduces the withheld figure the PR states,
including the untracked-symbol noise (`TC-999` ×5, `TC-1`, `FR-001-INV-1` ×3,
`TC-1598` ×2, `FR-003-CON-1`) that FND-094 assigned to PLAN-003 TASK-023. That
noise originates in the `vendor/ix-trace-rs` and `corpus` submodules — other
repositories — so the scanner's population boundary, not this branch, owns it.

## Independent differential probe

58 raw JSON values were digested through `classify_producer` and compared with
`json.dumps(sort_keys=True, separators=(",",":"), ensure_ascii=False)` +
SHA-256, deliberately choosing shapes the branch corpus does not carry:

- float magnitude sweep `1e14, 1e15, 1e16, 1e17, 1e18, 1e21, 1e22, 1E+09`
- `1e-3 … 1e-7`, `5e-324`, `1e-323`, `2.220446049250313e-16`
- `0.1`, `0.3`, `0.3333333333333333`, `0.1234567890123456789012345`
- `1.0e2`, `1E2`, `-1e-0`, `123456789012345678.0`, `±1.7976931348623157e308`
- `9007199254740993` and `9007199254740993.0`
- `1.2345e{16..23}` and `1.2345e{-4..-8}`
- duplicate keys, unicode keys and values, `""`, `"😀"`, empty object, empty
  array, mixed array `[1,2.0,-0,-0.0,3e0]`
- `1e400`, `-1e400`, `1e-400`, and a 42-digit integer-valued float

**55 of 58 matched byte-for-byte.** The three that did not were the probe's own
symmetry, not divergence: `1e400`/`-1e400` are refused by Rust with
`producer-output-number-non-finite` and by the reference with a `ValueError`
under `allow_nan=False`, and one case carried a raw control character that both
JSON parsers reject. No new numeric divergence exists in the FND-084 class.

The `arbitrary_precision` fix behaves as intended across the whole sweep,
including the two places it could plausibly have broken: exponent-form
normalisation (`1e-9 → 1e-09`, `1e20 → 1e+20`) and the leading-zero rewrite
(`0.00001 → 1e-05`, `0.0001` left alone) both agree with CPython `repr`.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The two "skipped" repository tests are gates that stood down; run here, the migration-contract gate fails on 15 unaccounted script families | tests/test_migration_contract.py:73, tests/test_compatibility_corpus.py:304 | correct-requirement-no-evidence |
| FND-002 | medium | FR-015-AC-1..AC-3 were downgraded from Property to Test while spec/tests.md still lists TC-100/102/103 as Property | spec/functional/FR-015-semantic-and-identity-parity.md:139, spec/tests.md:271 | wrong-requirement |
| FND-003 | medium | The parity gate now compares Rust against a Python reference this PR also edits, so a shared misconception is undetectable | engineering_assurance/evidence.py:46, tests/evidence_parity.rs:184 | correct-requirement-no-evidence |
| FND-004 | medium | Refusing an ASCII space in a version token is implemented behaviour with no owning acceptance criterion | src/evidence.rs:129, engineering_assurance/evidence.py:48 | missing-requirement |
| FND-005 | low | `has_non_finite_numeric_identity` depends on an unstated invariant about exponent-free tokens | src/evidence.rs:291 | correct-requirement-no-evidence |

## Finding detail

### FND-001 — two gates skipped, and one of them fails

The PR reports "repository tests: 186 passed, 2 skipped". Here the same command
reports **186 passed, 2 failed**. Same pass count, different verdict, because
both tests decide whether to run by looking at repositories outside this one.

`tests/test_migration_contract.py:73` skips only when a campaign repository
*directory* is absent. With those repositories checked out it reads their
current contents, and reports 15 script families present in the campaign
repositories that `docs/migration-contract.md` does not name:

```
assurance_chain.py, check_checksum_manifest.py, check_default_dependencies.py,
check_kani_harnesses.py, check_kani_mutations.py, check_provenance.py,
check_shared_pins.py, check_upstream_pins.py, measure_footprint.py,
run_feature_matrix.py, run_fuzz_smoke.sh, run_kani_gate.py,
rust_test_census.py, test_corpus_gate.py, validate_corpus.py
```

That is the gate working. The decision table is genuinely behind the campaign
repositories, and the only reason the PR did not see it is that the repositories
were not on disk.

`tests/test_compatibility_corpus.py:304` skips when a source repository "is not
checked out", but the guard tests for the directory, not for the revision it
needs. With `quire-contract-ir` present but not carrying
`origin/main:evidence/pgm-01-02568b1/manifest.json`, `git show` exits 128 and
the test fails rather than skipping.

Neither file is in this PR's diff. This is pre-existing and its repair belongs
to the ticket that owns the migration contract, not to this branch — but the
PR's verification line should not read as coverage. Either state that both
stood down and why, or check the required revision in the guard so absence and
staleness are distinguishable.

### FND-002 — verification method downgraded on one side of the matrix

FR-015-AC-1, AC-2 and AC-3 changed from `Property (TC-100/102/103)` to
`Test (TC-100/102/103)`. `Test` is the honest label — `Cargo.toml` carries no
property-testing dev-dependency, only `ix-trace-rs`, and the new corpus is a
deterministic generator rather than a property. But the Test Cases table in
`spec/tests.md` still types TC-100, TC-102 and TC-103 as `Property`, so the
matrix now disagrees with the FR it traces to. Change the type column too, or
say in the AC that the generated corpus stands in for the property.

### FND-003 — the oracle moved with the implementation

`tests/evidence_parity.rs` executes `engineering_assurance/evidence.py` as the
retained reference, and this PR edits that file. Measured against `main`'s copy,
five version tokens changed classification:

| version | on `main` | at `bad4a55` |
| --- | --- | --- |
| `1.2.3+linux-x86_64` | `identity-version-mutable` | accepted |
| `1.2.3+x86_64` | `identity-version-mutable` | accepted |
| `2.0.0-beta+exp.sha.5114f85` | `identity-version-mutable` | accepted |
| `1.2.3+X86` | `identity-version-mutable` | accepted |
| `1.*` | accepted | `identity-version-mutable` |

All five are the declared FND-087/FND-088 correction, they are stated in FR-015
and exercised by TC-103, and the correction is right. The finding is about what
the gate can still see: for the version-policy and non-finite paths, TC-100 and
TC-103 no longer pin pre-migration behaviour, so a misconception shared by both
implementations passes. The PR body describes this slice as one where "no legacy
executable path is removed", which is true of code and not of behaviour. Pinning
the pre-change classifications for these five tokens as fixtures — separately
from the live Python reference — would restore the ratchet.

### FND-004 — a refusal with no criterion behind it

Four version tokens valid on `main` are now rejected in both implementations
with `identity-version-invalid-character`:

`" 1.2.3"`, `"1.2.3 "`, `"1.2.3\n"`, `"1.2.3 rc"`

The FR body does state it — "a non-empty token composed only of printable ASCII
characters without whitespace" — so the requirement exists. The acceptance
criterion does not: FR-015-AC-3 enumerates "non-printable or non-ASCII version
tokens", and U+0020 is printable ASCII. A whitespace refusal is therefore
implemented, spec'd in prose, and ungated.

It also has a migration consequence nothing covers: a governing version read
from a file with a trailing newline was accepted before this PR and is refused
after it. Either add the whitespace class to AC-3, or trim before validating
and keep the refusal for genuinely non-graphic characters.

### FND-005 — an implicit invariant worth naming

`has_non_finite_numeric_identity` only examines a number whose token contains
`.`, `e` or `E`. That is correct against the reference — CPython's `json.loads`
yields `int` for an exponent-free token and an `int` cannot be non-finite — and
the probe confirms it at every boundary, including a 42-digit integer and
`1e-400`. But the correctness depends entirely on that property of the retained
parser, and `arbitrary_precision` is exactly the feature that makes the token
shape load-bearing. One comment naming the invariant would keep the next change
to this path from removing the check's meaning without removing the check.

## What is correct

- The `arbitrary_precision` fix is real, and it is the right one of the two
  options SR-040 offered: integers beyond `i64`/`u64` are emitted verbatim
  rather than routed through `f64`.
- `legacy_canonical_json`'s string/number state machine is sound. Escape
  tracking, key ordering (serde_json's `BTreeMap` byte order equals CPython's
  `sorted()` code-point order for UTF-8), and `ensure_ascii=False` escape parity
  all hold across the 58-shape probe.
- The version-policy correction is symmetric between the two implementations —
  build metadata after `+` is excluded from the wildcard scan on both sides,
  `*` was added to the marker set on both, and ASCII-case handling matches
  because the printable-ASCII check runs first.
- Nothing in `src/` panics on caller input: `unwrap`/`expect` appear only in
  `#[cfg(test)]` code, `unsafe_code = "forbid"` and `missing_docs = "deny"` are
  in force, and no `#[allow(...)]` was added.
- PLAN-003 is a real plan bundle, not a placeholder: twelve tasks with an
  explicit serial order, named external gates, and a completion state.
- The withheld 207/262 traceability figure is accurate and the refusal to claim
  release completion on it is the right call.
