---
id: SR-061
title: "Independent re-review — semantic view invariants and the hosted Rust gate at 6611346"
type: SpecReview
analysis: code-review
scope: "PR #32 at 6611346 against the reviewed head a39b4f1; SR-060 FND-001..004; src/semantics/pgm01.rs tests; .github/workflows/ci.yml; tests/test_module.py; Makefile rust-* targets"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---

## Summary

Second re-review, at `6611346`. The two mediums that were about missing
evidence are closed, and both were verified by mutation. Gutting
`validate_contract` now fails the suite where at `a39b4f1` it passed silently,
and the hosted job now checks out submodules, installs exact Rust 1.98.1, and
runs `make rust-foundation-gate` — with a Python meta-test that fails if any of
those three is removed or floated.

The CI fix wired five of the six `rust-*` targets. `rust-deps` — `cargo deny
check`, with a committed `deny.toml` — is still invoked by nothing, and no
`cargo audit` target exists at all. The two findings about the shape of the API
and the reach of the differential are untouched; the new test entrenches the
first of them.

## Verdict

**CONDITIONAL** — no high findings. Three remain: one newly measured, two
carried forward.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Six new refusals still share the single constant code `invalid_semantic_contract`, and the new test now pins six exact message strings, making the message the tested contract | src/semantics/pgm01.rs:660, src/semantics/mod.rs:64 | wrong-requirement |
| FND-002 | low | `rust-deps` (`cargo deny check`) and its `deny.toml` are invoked by no target and no workflow; the CI fix wired five of six `rust-*` targets, and no `cargo audit` target exists | Makefile:44, Makefile:47 | missing-requirement |
| FND-003 | low | The adverse differential still covers five scalar shapes; JSON array and object identities diverge, and there the retained Python raises `TypeError` | tests/semantics_parity.rs:308 | correct-requirement-no-evidence |

## Finding detail

### FND-001 — the new test makes the message the contract

The six refusals are now exercised, which is the fix I asked for. What they are
distinguished *by* got worse:

```rust
fn assert_contract_error(view: Pgm01View, expected: &str) {
    let error = view.validate_contract(RAW).expect_err("…");
    assert_eq!(error.code(), "invalid_semantic_contract");
    assert_eq!(error.message(), expected);
}
```

`code()` is asserted six times and is the same constant every time — the test
records that the machine-readable category carries no information. The
distinguishing assertion is `assert_eq!` on the exact prose. So at `a39b4f1` the
six conditions were indistinguishable to a consumer; at `6611346` they are
distinguishable only by exact English string equality, and that equality is now
a tested contract, which makes it harder to change rather than easier.

`agent-skills/rust-review` §0b: the variant set is the API, not the message;
the grep symptom is callers comparing messages to decide what happened. Nine
call sites in `tests/semantics_parity.rs` already do this with `.contains(…)`,
and this commit adds six more with `assert_eq!`.

This is worth resolving before #60 takes the API handoff, because the fix is
cheap now and is a breaking change afterwards. A `SemanticErrorKind` enum —
`InvalidMappingVersion`, `EmptySourceIdentity`, `ChangedSourceDigest`,
`InvalidFieldMapping`, `InvalidUnmappedField`, `EmptyLimitation`, plus the
existing reference-validation conditions — returned by `code()` or by a `kind()`
accessor, lets the same test assert `kind()` and leaves the prose free to
change. The messages stay exactly as they are; only what the test pins moves.

### FND-002 — the sixth Rust target is still orphaned

`make rust-foundation-gate` is now genuinely wired into the manual job and works.
It expands to:

```make
rust-foundation-gate: rust-format rust-clippy rust-toolchain rust-tests rust-docs
```

`rust-deps` is defined four lines above it —

```make
rust-deps:
	$(CARGO) deny check
```

— and `grep -n 'rust-deps' Makefile .github/workflows/*` returns only its
`.PHONY` declaration and its own recipe. It is in no aggregate:
`rust-foundation-gate` omits it, and so do `integration-gate` and
`release-gate`. A `deny.toml` is committed and nothing reads it.
`grep -rn 'cargo audit\|cargo-audit' Makefile .github/` returns nothing at all.

Measured at this head, so the severity is honest:

| Gate | Result |
| --- | --- |
| `make rust-deps` | `advisories ok, bans ok, licenses ok, sources ok` |
| `cargo audit` | 28 crate dependencies scanned, **zero advisories** |

Nothing is wrong today, which is why this is low rather than high. It is the
same defect class the commit just fixed, one level down: the previous state was
six `rust-*` targets that no automation ran; the new state is five that it runs
and one that it does not, and the one left out is the supply-chain gate.

Worth weighing against the sibling repository: `quire-contract-ir` runs both
`cargo deny check` and `cargo audit` as hosted steps, and its SR-035 FND-1510
records that adding the real advisory gate is what surfaced vulnerable
`idna 0.4.0` and `time 0.3.36` there. A clean result today is not evidence that
the gate is unnecessary; it is evidence that running it is cheap.

Adding `rust-deps` to `rust-foundation-gate` is a one-line change. Adding a
`rust-audit` target is two more.

### FND-003 — unchanged from SR-060

`git diff a39b4f1 6611346 -- tests/semantics_parity.rs` is empty. The measured
divergences stand:

| Input | Rust | Python |
| --- | --- | --- |
| `recordId: {"a":1}` | `incompatible-source` | `{'a': 1}` |
| `schemaVersion: [1,2]` | `unknown` | raises `TypeError: unhashable type: 'list'` |

Rust is the better behaviour in both. The finding is the scope claim, not the
behaviour: the test is named `…adverse_identity_views_match_retained_python`
and for two of the seven JSON shapes they do not match. Add the two cases and
assert the intended divergence, or narrow the name and TC-100's criterion to
scalar identities.

## Closed since SR-060

### FND-001 medium — the guard now has a test

`tc_103_generated_pgm01_view_refuses_each_invalid_field_family` exercises all
six refusals against a valid baseline view. Verified by re-running the exact
mutation that passed at `a39b4f1` — replacing `validate_contract`'s body with
`Ok(self)`, a 41-line deletion:

```
 src/semantics/pgm01.rs | 42 +-----------------------------------------
 1 file changed, 1 insertion(+), 41 deletions(-)

thread 'semantics::pgm01::tests::tc_103_generated_pgm01_view_refuses_each_invalid_field_family'
  panicked at src/semantics/pgm01.rs:665:14
test result: FAILED. 4 passed; 1 failed
suite_rc=101
```

At `a39b4f1` the same deletion left the suite green. The gate is now a gate.

The unit test lives in `#[cfg(test)] mod tests` inside `pgm01.rs`, which is the
only way to reach a private method — correct placement, and it carries a
`#[trace("TC-103", "FR-015-AC-3")]` marker consistent with the repository's
convention.

### FND-003 medium — the hosted job compiles and tests the Rust library

```yaml
      - uses: actions/checkout@3d3c42e5…
        with:
          submodules: recursive
      …
      - uses: dtolnay/rust-toolchain@4360b525…
        with:
          toolchain: 1.98.1
          components: rustfmt, clippy
      …
      - run: make rust-foundation-gate
```

All three parts of the finding are addressed: the submodule gap that would have
made `vendor/ix-trace-rs` absent, the absent toolchain, and the absent step. The
trigger stays `workflow_dispatch:` only, so the manual-dispatch policy is
untouched — which was the point.

`test_manual_verification_workflow_runs_the_rust_foundation_gate` pins it.
Measured:

| Mutation to `ci.yml` | `pytest tests/test_module.py` |
| --- | --- |
| drop `- run: make rust-foundation-gate` | **fail** |
| drop `submodules: recursive` | **fail** |
| `toolchain: 1.98.1` → `toolchain: stable` | **fail** |
| baseline | 14 passed |

`make rust-foundation-gate` passes locally at this head, so the step is not
being wired green-by-assumption.

One gap in that meta-test, recorded as an observation rather than a finding
because its practical effect is near zero: it asserts the toolchain step
*exists* and that *some* step runs the gate, not that the install precedes the
gate. Moving the `dtolnay/rust-toolchain` block to after
`- run: make rust-foundation-gate` still passes all 14 tests. In practice
`rust-toolchain.toml` pins `channel = "1.98.1"`, so rustup would fetch the right
compiler when `cargo` first runs regardless of ordering.

## Verified and found sound

- **Gates green at this head**: `make lint` · `make test` (**187 passed**, 2
  skipped — one more than at `a39b4f1`) · `make package-audit` ·
  `make rust-foundation-gate` (fmt, clippy, check, tests, strict rustdoc) ·
  `make rust-deps` · `cargo audit`.
- **The CI toolchain declaration does not conflict with the repository pin.**
  `rust-toolchain.toml` is `channel = "1.98.1"` and `Cargo.toml` is
  `rust-version = "1.98.1"`; the workflow's `toolchain: 1.98.1` agrees with both.
- **The rustfmt warnings are still not this repository's.** They come from
  `vendor/ix-trace-rs/rustfmt.toml`. `quire-rs` removed the identical pair from
  its own copy at `8b636b7`; `ix-trace-rs` and `rust-lib-cookiecutter` still
  carry it. Tracked there, not here.

## What this re-review does not do

It does not approve the pull request, dispatch hosted CI, decide whether
`rust-deps` belongs in the foundation gate, or reopen anything SR-059 or SR-060
closed.
