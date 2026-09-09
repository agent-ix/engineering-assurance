---
id: SR-059
title: "Independent code review — Rust semantic projection slice at bb7e062"
type: SpecReview
analysis: code-review
scope: "PR #32 at bb7e062; src/semantics/mod.rs, pgm01.rs, report.rs, fixtures.rs; tests/semantics_parity.rs; spec/tests.md; .github/workflows/ci.yml; FR-014, FR-015, TC-096..TC-104"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---

## Summary

The Rust slice is careful work and its gates reproduce exactly: fmt, Clippy
`-D warnings`, 24 Rust tests, `cargo deny`, and the retained Python suite at
186 passed / 2 skipped all match the PR body. The typing is genuinely better
than the Python it mirrors — closed enums, `deny_unknown_fields` on every wire
struct, `#[serde(flatten)]` opaque-field capture, and exact arbitrary-precision
integer tokens. The matrix updates are honest; nothing was quietly promoted.
Three things do not hold up under measurement: the PGM-01 mapper can emit a
view that the repository's own committed schema rejects, the negative-capability
audit that FR-015-AC-4 rests on does not bind, and no CI or `make` path runs any
of the 2,287 new Rust lines.

## Verdict

**FAIL** — one high. The high is a measured contract violation with a
four-character reproduction, not a style judgement; the two mediums below it are
gates that do not gate.

## Gates run at `bb7e062`

Exact Rust 1.98.1 (`rustc 1.98.1 (48a229cea 2026-09-01)`), `-j 2`, isolated
target directory. Submodules initialised by hand — `git worktree add` does not
do it, and without `vendor/ix-trace-rs` the Rust build cannot resolve its
dev-dependency at all.

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass (with the repo's pre-existing nightly-only import-grouping warnings) |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | pass |
| `cargo test --locked` | pass — 24 tests, 0 failed, 0 ignored, including the 6 semantic integration tests |
| `cargo deny check` | advisories, bans, licenses, sources ok |
| `python3 -m pytest` | **186 passed, 2 skipped** — matches the PR body exactly |

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | `map_pgm01_bytes` returns a view that the repository's own `pgm01-compatibility-view-v1.schema.json` rejects; Python cannot produce it | src/semantics/pgm01.rs:583, engineering_assurance/schemas/pgm01-compatibility-view-v1.schema.json:11 | missing-requirement |
| FND-002 | medium | The FR-015-AC-4 negative-capability audit is a substring check that the repo's own dominant import style evades | tests/semantics_parity.rs:337 | correct-requirement-no-evidence |
| FND-003 | medium | No CI job and no `make` target that CI runs executes any Rust; CI could not run it if asked | .github/workflows/ci.yml:1, Makefile:18 | missing-requirement |
| FND-004 | medium | The retained-Python differential covers accepted inputs only, while TC-100's criterion claims adverse cases agree; 5 of 7 identity divergences measured | tests/semantics_parity.rs:159, spec/tests.md:270 | correct-requirement-no-evidence |
| FND-005 | low | The two pytest skips are reported as "optional-source" but are the location-dependent `REPO_ROOT.parent` defect carried from SR-052 | tests/test_migration_contract.py:73, tests/test_compatibility_corpus.py:304 | correct-requirement-no-evidence |

## Finding detail

### FND-001 — the Rust view can violate its own committed contract

Python coerces a falsy source identity to a placeholder and then validates the
view it is about to return:

```python
schema_version=str(schema_version or "unknown"),
record_id=str(record_id or "incompatible-source"),
...
_validate(view, "pgm01-compatibility-view-v1.schema.json")
```

Rust does neither. It reads the two identity fields with `Value::as_str` and
falls back only on `None`:

```rust
schema_version.as_deref().unwrap_or("unknown"),
record_id.as_deref().unwrap_or("incompatible-source"),
```

An **empty string** is `Some("")`, so it passes straight through. The committed
schema requires `"minLength": 1` on both fields, and Rust never runs the schema
at all — there is no equivalent of `_validate` on any Rust return path.

Measured. Feeding four inputs to the Rust mapper, serialising each returned
view, and validating it with the repository's own `_validate`:

```
empty-schemaversion: SCHEMA REJECTED -> source_schema_version: '' should be non-empty
empty-recordid:      SCHEMA REJECTED -> source_record_id: '' should be non-empty
both-empty:          SCHEMA REJECTED -> source_record_id: '' should be non-empty
control-good:        SCHEMA OK
```

The reproduction is `{"schemaVersion":"","recordId":"legacy-1"}`. Rust returns
`Ok` with `outcome: "incompatible"` and `source_schema_version: ""`; Python
returns `outcome: "incompatible"` with `source_schema_version: "unknown"`.

This matters more than an empty-string edge case, because this module's entire
job is reading arbitrary historical bytes and classifying them without
inventing semantics. A legacy record with an empty version string is exactly
the input it exists for, and the answer it gives is a record that fails the
governing schema.

Two changes close it, and the second closes the class rather than the instance:

1. Treat empty as absent — `.filter(|value| !value.is_empty())` after `as_str`
   — so the placeholder path is reached for the same inputs Python reaches it
   for.
2. Validate the constructed view before returning it. Rust has no JSON-schema
   dependency, but the invariants the schema states are three lines of typed
   checks against fields the code already owns, and running them on every
   return path is what makes the divergence impossible rather than fixed once.

### FND-002 — the negative-capability audit does not bind

FR-008-AC-4 and NFR-004-AC-2 are negative capability requirements, and the test
says so explicitly:

```rust
// FR-008-AC-4 / NFR-004-AC-2 are negative capability requirements. Static
// inspection is the direct gate: there is no runtime path to exercise for
// an execution or persistence capability that must not exist.
for forbidden in ["std::fs", "std::process", "std::env", "Command::new"] {
    assert!(!semantic_sources.contains(forbidden), ...);
}
```

Choosing source inspection here is right — there is no runtime path to test for
a capability that must not exist. The problem is that the check matches the
*path prefix* while the repository writes its imports braced. `src/semantics/mod.rs:9`
is already `use std::collections::{BTreeMap, BTreeSet};`, and the braced form of
a filesystem import is `use std::{collections::BTreeMap, fs};` — which contains
`std::{fs`, never `std::fs`.

Measured, both mutants applied to `bb7e062` and reverted:

| # | Mutant | Result |
| --- | --- | --- |
| M1 | `use std::{collections::BTreeMap, fs, path::Path};` plus a `pub fn` calling `fs::read` in `src/semantics/fixtures.rs` | **all 6 semantic tests pass** |
| M1b | `use std::{collections::BTreeMap, process::Command as Spawn};` plus a `pub fn` calling `Spawn::new("echo").output()` | **all 6 semantic tests pass** |

M1b is the one that matters. A function that spawns a process now lives inside
the module whose `//!` header says it "does not execute a producer", and the
gate written to prevent exactly that reports green. SR-057 FND-117's closing
statement — "Production code contains no ... filesystem/process/environment
access" — is true today, but it is not held true by anything.

There is a second hole in the same test. `semantic_sources` is a `concat()` of
four literal `include_str!` paths, so a fifth file added under `src/semantics/`
is not audited at all.

Both close together: read the directory rather than listing it, and match the
call form rather than the import path — `fs::`, `process::`, `env::`,
`Command::` — since those appear at the use site regardless of how the import
was spelled. The test already does `fs::read_dir` twice for its other audits, so
the machinery is present.

### FND-003 — nothing in CI runs the Rust

`.github/workflows/ci.yml` contains **zero** occurrences of `rust` or `cargo`.
Its three verification steps are `make lint`, `make test` and
`make package-audit`, and in the Makefile those are:

```make
lint:  $(PYTHON) -m ruff check .
test:  $(PYTHON) scripts/check_content_rights.py --tree
       $(PYTHON) -m pytest
       $(PYTHON) scripts/validate_manifest.py
```

The Rust gates exist as `rust-format`, `rust-clippy`, `rust-toolchain`,
`rust-tests`, `rust-docs`, `rust-deps` and the `rust-foundation-gate` aggregate
— and nothing invokes any of them. The repository's `pre-push` hook runs
`make lint`, `make test` and `make package-audit`, so it does not cover them
either. This PR adds 2,287 lines of Rust across four new source modules and a
412-line integration test, and no automated path executes a line of it.

It could not, as written. The job installs Python, Node and pnpm but no Rust
toolchain, and `actions/checkout` is called with no `submodules:` key — so
`vendor/ix-trace-rs`, which the `[dev-dependencies]` entry resolves by path,
would be absent. I hit exactly that: the first gate run in a fresh worktree
failed with `failed to read vendor/ix-trace-rs/Cargo.toml`, and
`corpus/compatibility/corpus.json`, which
`tc_100_rust_generator_matches_all_committed_inert_fixtures` reads, is in the
other submodule.

The `workflow_dispatch:`-only trigger is the standing repository policy and is
correct — this is not about when the job runs. It is that when someone does
dispatch it, the Rust is not in it, and three things must change before it can
be: a toolchain step, `submodules: recursive`, and a `make rust-foundation-gate`
step.

### FND-004 — the differential oracle covers the happy path only

Two tests compare Rust against retained Python. Both feed accepted inputs:
`pgm01-v1.json` and `pgm01-v2.json` for the mapper, `report-projection.json` for
the renderer. Adverse behaviour is asserted separately, in
`tc_103_pgm01_adverse_outcomes_preserve_source_identity`, against hand-written
Rust expectations — never against Python.

TC-100's criterion in `spec/tests.md` claims more than that: "Rust agrees
byte-for-byte with the accepted identity and report reference over the named
corpus **and focused adverse cases**". The row is marked backed for the
compatibility, semantic-validation and PGM-projection slices.

Measured. Running both implementations over the same adverse inputs and
comparing the returned views:

| Population | Cases | Agree | Diverge |
| --- | --- | --- | --- |
| Malformed / unreadable records | 8 | 2 | **6** |
| Non-string and empty identity fields | 7 | 2 | **5** |

Every one of the eight malformed cases agrees on `outcome`, `source_digest`,
`mappings` and `limitations` — the divergence there is the `reason` text, and
where it differs the Rust is more precise (`/commands/0/stdout/bytes` against
Python's `/bytes`). That half is a defensible, undisclosed improvement.

The identity population is the one that matters, and its divergences are field
values, not prose:

```
schemaversion-number  rust schema_version="unknown"  py "123"
schemaversion-empty   rust schema_version=""         py "unknown"
recordid-number       rust record_id="incompatible-source"  py "42"
recordid-empty        rust record_id=""              py "incompatible-source"
schemaversion-object  rust incompatible              py TypeError: unhashable type: 'dict'
```

Two of these are Rust discarding a source identity that Python preserves, in a
view whose stated purpose is preserving source identity; two are FND-001; and
the last is a latent Python crash that the Rust correctly survives.

None of this makes the port wrong — the classification agrees in every case I
could construct. It makes the *claim* wider than the evidence. Either narrow
TC-100's status to the accepted population, or extend the differential over the
adverse fixtures, which costs one loop over a directory since both harnesses
already exist in the same test.

### FND-005 — the two skips are not optional-source

The PR body and SR-057's gate table both record "186 tests and 2 stated
optional-source skips". Reproduced with `-rs`:

```
SKIPPED tests/test_compatibility_corpus.py:304: source repository quire-contract-ir is not checked out
SKIPPED tests/test_migration_contract.py:73: campaign repositories not checked out: quire-contract-ir, ...
```

Both resolve sibling checkouts through `REPO_ROOT.parent`, so they skip in any
worktree and pass in the primary clone. The count is a property of where the
suite was run, not of what is available. This is SR-052 FND-001 against #29,
unchanged and now inherited; it is recorded here so the "2 skips" figure is not
read as a fixed characteristic of the suite.

## Checked and found sound

Recorded so the next reviewer does not spend the time again.

- **The arbitrary-precision handling is correct and non-obvious.**
  `required_non_negative_integer` inspects the JSON number *token* rather than
  converting, so `184467440737095516160` survives above `u64::MAX` and `-0`
  normalises to `0` exactly as CPython's `json.loads` does. `1.0`, `1e5`, `-5`,
  `"12"` and `true` are all refused on both sides. This is SR-057 FND-113 closed
  properly, and both boundaries are asserted.
- **`deny_unknown_fields` is on every wire struct**, and the deliberately
  opaque legacy fields are captured in `#[serde(flatten)] _unmapped` maps rather
  than dropped — so an unrecognised historical field cannot be silently treated
  as absent.
- **The identity charset closes the Markdown-injection question.**
  `validate_identity` admits only alphanumerics and `. _ : / -`, so the
  `` `{}` ``-wrapped ids in `render_markdown` cannot carry a backtick, and
  free text goes through `markdown_text`, which folds newlines and escapes `|`.
- **No panic surface in the library.** No `unwrap`, `expect`, `panic!`,
  slicing, indexing, `as` cast, `unsafe`, or `#[allow]` in any of the four
  modules; `unsafe_code = "forbid"` and `missing_docs = "deny"` hold, and every
  fallible path returns `SemanticError`.
- **The matrix is honest.** Every FR-015 row stays `🚧` with a narrower
  description; only TC-096 and FR-014-CON-3 move to `✅`, and only for the
  package foundation, which is what the tests actually establish. Nothing was
  promoted to passing on the strength of this slice.
- **Failing rather than skipping when Python is absent is the right call.**
  The parity tests `.expect("retained Python reference must execute during
  additive parity")`. For a differential oracle that is correct — a skipped
  oracle is a green suite with no comparison in it. Worth knowing that this
  makes `cargo test` depend on an importable `engineering_assurance` and on
  `jsonschema` being installed, which no Cargo manifest states.

## One observation, not a finding

`SemanticError::code()` is documented as "the stable machine-readable error
category", and `SemanticError::new` sets it to the single constant
`"invalid_semantic_contract"` at every one of its call sites. The accessor
cannot return a second value, so `assert_eq!(error.code(), "invalid_semantic_contract")`
in `tc_100` can only fail if someone edits the constant. SR-057 FND-117 records
the single boundary deliberately, so this is disclosed rather than hidden — but
callers currently discriminate failures by substring-matching `message()`, which
is what a code catalogue exists to avoid, and the adverse tests do exactly that
in nine places.
