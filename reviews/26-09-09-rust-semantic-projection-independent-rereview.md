---
id: SR-060
title: "Independent re-review — PGM-01 view validation and identity parity at a39b4f1"
type: SpecReview
analysis: code-review
scope: "PR #32 at a39b4f1 against the reviewed head bb7e062; SR-059 FND-001..005; src/semantics/pgm01.rs; tests/semantics_parity.rs; .github/workflows/ci.yml"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---

## Summary

Re-review of the five SR-059 findings at `a39b4f1`. The high finding is fixed
and the fix is the right one: `compatibility_identity` now reproduces CPython's
truthiness for strings, numbers, booleans and null, so an empty
`schemaVersion` or `recordId` yields the same placeholder Python emits instead
of an empty string the repository's own schema rejects. Five adverse identity
cases are compared directly against retained Python, and the
negative-capability audit is now a recursive, token-based scan with its own
mutation controls — I killed it with a file in a new subdirectory and with the
aliased-import evasion that defeated the old substring check.

Two things did not survive measurement. The 50-line `validate_contract` guard
that the PR body headlines can be deleted in full with the locked suite still
green, and two identity shapes still diverge from Python. Two SR-059 findings
were dispositioned rather than fixed; one of those dispositions cites a policy
that does not cover it.

## Verdict

**CONDITIONAL** — no high findings. SR-059's high is closed on the substance.
What remains is evidence reach: a new gate with no test, a differential that
stops short of the shapes that still disagree, and no automation that compiles
the library at all.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Deleting the entire `validate_contract` guard leaves the full locked suite green; the output-validation layer the PR body headlines has no test | src/semantics/pgm01.rs:86 | correct-requirement-no-evidence |
| FND-002 | medium | Six new refusal conditions were added under the single constant error code `invalid_semantic_contract`, and callers already discriminate refusals by substring-matching `message()` in nine places | src/semantics/mod.rs:64, tests/semantics_parity.rs:116 | wrong-requirement |
| FND-003 | medium | No CI job and no `make` target CI runs compiles or tests the Rust library; the cited local-only policy governs triggers, not job content, and the checkout has no `submodules:` key | .github/workflows/ci.yml:11 | missing-requirement |
| FND-004 | low | The new adverse differential covers five scalar shapes; JSON array and object identities still diverge, and there Python raises `TypeError` where Rust returns a view | tests/semantics_parity.rs:308 | correct-requirement-no-evidence |

## Finding detail

### FND-001 — the guard that closes the high finding is not itself gated

The PR body says:

> every returned PGM compatibility view validates its typed output invariants
> and exact source digest

`validate_contract` is 41 lines and six refusals: mapping version, empty source
identity, changed source digest, invalid field mapping, invalid unmapped field,
empty limitation. Every return path in `map_pgm01_bytes` now goes through it.

Measured — replacing the whole function body with `Ok(self)`, a verified
41-line deletion, then `cargo test --locked`:

```
 src/semantics/pgm01.rs | 42 +-----------------------------------------
 1 file changed, 1 insertion(+), 41 deletions(-)

test result: ok. 4 passed; 0 failed …
test result: ok. 7 passed; 0 failed …   (semantics integration)
… 25 passed, 0 failed across all suites
```

`grep -rn 'generated PGM' tests/ src/` outside `pgm01.rs` returns nothing. Not
one of the six refusals has a test.

This does not mean the defect is unfixed. The behavioural fix is
`compatibility_identity`, and the new differential does test that. But a
validation layer whose removal is invisible to the suite is not evidence, and
the PR body offers it as evidence. Either exercise the refusals — construct a
view that violates each invariant and assert the error — or describe it as
defence-in-depth rather than as verification.

There is a second reading worth stating plainly: because
`compatibility_identity` can no longer return an empty string, several of the
six branches may be unreachable by construction today. That is fine for a
safety net and fatal for a gate. Deciding which it is determines whether the
tests are worth writing.

### FND-002 — six new refusals, one error code

`SemanticError` carries a `code` field that is assigned a single constant:

```rust
fn new(message: impl Into<String>) -> Self {
    Self { code: "invalid_semantic_contract", message: message.into() }
}

/// Return the stable machine-readable error category.
pub const fn code(&self) -> &'static str { self.code }
```

`code()` is documented as the stable machine-readable category and returns the
same value for every failure in the module. The discriminant lives in the
message, and callers already read it that way. `tc_103_semantic_reference_failures_do_not_collapse`
— a test whose name asserts these failures do not collapse — separates six
refusals like this:

```rust
.message().contains("missing reference")
.message().contains("must target check_result")
.message().contains("authority must be quire")
.message().contains("complete producer tuple")
.message().contains("distinct")
.message().contains("premises differ")
```

`agent-skills/rust-review` §0b names this directly: the variant set is the API,
not the message; the grep symptom is callers that compare messages to decide
what happened.

This condition predates the PR. What the PR does is add six more refusals under
the same constant code, so a consumer distinguishing "the source digest
changed" from "a limitation is empty" has no option but substring matching, on
strings no test pins. This is the sole public error type of a library #60 is
about to consume through a narrow API handoff, which is when the shape is
cheapest to change.

### FND-003 — nothing compiles the Rust library

Unchanged since SR-059, and now dispositioned. The PR body says:

> The hosted-workflow observation does not change the approved local-only
> verification policy.

The policy the repository actually documents is about **triggers**:

> Every workflow in every campaign repository stays manual-dispatch only. This
> playbook dispatches nothing and changes no trigger.
> — `docs/migration-contract.md:168`

That policy is satisfied by a Rust step; `on: workflow_dispatch:` stays
`workflow_dispatch:` either way. I searched `spec/`, `docs/`, `CLAUDE.md` and
`AGENTS.md` and found no statement that Rust verification is local-only.

At `a39b4f1`, measured:

- `grep -cin 'rust\|cargo' .github/workflows/ci.yml` → **0**
- `git log bb7e062..a39b4f1 -- .github/ Makefile` → **no commits**
- the manual `verify` job runs `make lint`, `make test`, `make package-audit`;
  none of the three invokes Cargo, and the `rust-*` targets in the Makefile are
  invoked by nothing
- the checkout step has no `submodules:` key, so `vendor/ix-trace-rs` would be
  absent and a Rust step could not build even if one were added today

The concrete consequence: the merged Rust library will be compiled only on the
machine of whoever last touched it. That is the owner's call to accept — but it
should be accepted as itself, not as an implication of the trigger policy. If
it is accepted, the honest form is one line in `docs/migration-contract.md`
saying Rust verification is local-only and why, and adding `submodules:
recursive` so the decision stays reversible.

### FND-004 — two identity shapes still disagree

`tc_100_pgm01_adverse_identity_views_match_retained_python` compares five cases:
empty string, empty string, integer, integer, `false`/`null`. All five agree.

The two shapes it does not cover are the two that still diverge. Measured, both
engines at `a39b4f1`:

| Input | Rust | Python |
| --- | --- | --- |
| `{"schemaVersion":"…v99","recordId":{"a":1}}` | `incompatible-source` | `{'a': 1}` |
| `{"schemaVersion":[1,2],…}` | `unknown` | raises `TypeError: unhashable type: 'list'` |
| `{"schemaVersion":[],"recordId":{}}` | `unknown` / `incompatible-source` | raises `TypeError: unhashable type: 'list'` |
| `{"schemaVersion":0,…}` | `unknown` | `unknown` |
| `{"schemaVersion":"…v99","recordId":1.5}` | `1.5` | `1.5` |
| `{"schemaVersion":true,…}` | `True` | `True` |

`compatibility_identity` matches `Value::String`, `Value::Number` and
`Value::Bool(true)` and sends everything else to the fallback. Python's
`str(x or default)` renders a non-empty list or dict.

Rust is the better behaviour in both rows — its output is schema-valid where
Python crashes on untrusted input — so this is not a defect in the port. It is
a scope claim. TC-100's criterion is that adverse cases agree with retained
Python, the test is named `…adverse_identity_views_match_retained_python`, and
for two of the seven JSON shapes they do not. Either add the two cases and
assert the intended divergence explicitly, or narrow the claim to scalar
identities.

Worth recording separately: `engineering_assurance/verification_semantics.py`
raises an unhandled `TypeError` on a JSON object or array in `schemaVersion`.
No test covers it. It is the reference implementation being retired, so the
value of fixing it is low, but it should not be discovered later and read as a
regression introduced by the port.

## Closed since SR-059

| SR-059 | Status at `a39b4f1` |
| --- | --- |
| FND-001 high — the view can violate its own committed schema | **fixed**; `compatibility_identity` reproduces CPython truthiness, and `validate_contract` covers every constraint in `pgm01-compatibility-view-v1.schema.json` including `additionalProperties: false` by construction |
| FND-002 medium — the capability audit is evadable | **fixed**; verified by two mutants |
| FND-005 low — the two skips are not optional-source | **fixed**; the gate table now reads "2 location-dependent sibling-checkout skips … not represented as optional-source evidence" |

The capability audit was measured, not assumed. `cargo test --test semantics_parity`
at `a39b4f1`:

| Mutation | Result |
| --- | --- |
| `use std::fs;` in a **new** file `src/semantics/sub/evil.rs` | **fail** — recursion works |
| `use std::fs as filesystem;` in `src/semantics/report.rs` — the alias evasion that defeated the old check | **fail** |
| baseline | pass |

The three in-test mutants (braced `fs`, `process::Command as Spawn`, `env as
ambient`) are controls on the detector rather than on the tree, which is the
right place for them.

## Verified and found sound

- **The committed JSON Schema is bound to the Rust view, transitively.**
  `validate_contract` is a hand-reimplementation of
  `pgm01-compatibility-view-v1.schema.json`, which normally drifts. It does not
  here: tightening `source_schema_version` to `minLength: 80` in the schema
  fails `tc_100_pgm01_views_match_retained_python_for_accepted_inputs`, because
  the retained Python validates against the file. The binding lasts exactly as
  long as the Python reference does — worth knowing before it is retired.
- **Gates green at this head**: `make lint`, `make test` (186 passed, 2 skipped),
  `make package-audit`, `cargo fmt --check`,
  `clippy --all-targets --all-features --locked -D warnings`, and
  `cargo test --locked` (25 passed, 0 failed) all pass.
- **The rustfmt warnings are not this repository's.** `cargo fmt` emits 8
  `imports_granularity`/`group_imports` nightly-only warnings; they come from
  `vendor/ix-trace-rs/rustfmt.toml` in the vendored submodule. This repository
  has no rustfmt.toml. Not a finding here.

## What this re-review does not do

It does not approve the pull request, dispatch hosted CI, decide the local-only
verification question, or reopen anything SR-059 closed.
