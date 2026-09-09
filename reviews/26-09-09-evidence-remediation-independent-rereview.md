---
id: SR-048
title: "Independent re-review of the evidence remediation at fba0326"
type: SpecReview
analysis: code-review
scope: "PR #27 at fba0326; qa-corpus PR #16 at 4b390c2; src/evidence.rs; tests/evidence_parity.rs; tests/test_evidence.py; engineering_assurance/fixtures/evidence-version-policy.json; docs/migration-contract.md; FR-015; spec/tests.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: "references"
---

## Summary

The high finding is closed and closed properly. `make test` now reports 188
passed with **no skips**: the migration-contract census reads the campaign
repositories and passes because `docs/migration-contract.md` classifies all
fifteen previously unaccounted families, and the corpus reproduction gate passes
because qa-corpus #16 makes `--check` read the revisions the corpus itself
records instead of a moving `origin/main`. Every medium and low finding from
SR-044 and SR-045 is addressed except the traceability population boundary,
which stays open by prior disposition. What remains is one measured weakness in
the new fixture and three low observations.

## Verdict

**CONDITIONAL** — no high findings. The blocking evidence defect is resolved and
verified; the remaining items are gate-strength and vocabulary, each closable
without a behaviour change.

## Gates run at `fba0326`

Rust 1.98.1, the toolchain `rust-toolchain.toml` selects; submodules initialised;
`corpus` at `4b390c2`.

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | pass (nightly-only grouping warnings, non-blocking) |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | pass |
| `cargo test --locked --all-features` | pass — 21 tests |
| `cargo deny check` | pass — advisories, bans, licenses, sources |
| `make lint` | pass |
| `make test` | **188 passed, 0 failed, 0 skipped** |
| `make integration-traceability` | 207/262, the declared withheld state; untracked-symbol noise unchanged |
| qa-corpus `compatibility_corpus_selftest.py` | pass |
| qa-corpus `build_compatibility_corpus.py --check` | pass — "corpus reproduces from source" |
| qa-corpus `make ci` | **fails at `verify`, 18 mismatches** — reproduced identically on parent `ea4ac7a`; see FND-004 |

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The version-policy fixture gates only the rows it happens to contain; deleting eight of nine cases leaves both suites green | engineering_assurance/fixtures/evidence-version-policy.json, tests/evidence_parity.rs:453 | correct-requirement-no-evidence |
| FND-002 | medium | No requirement states the traceability population, so submodule symbols still count against this repository | scripts/check_integration_evidence.py | missing-requirement |
| FND-003 | low | Corpus `--check` no longer verifies the recorded revision is published, and nothing else does | scripts/build_compatibility_corpus.py:672 | correct-requirement-no-evidence |
| FND-004 | low | qa-corpus `make ci` cannot pass in this environment before or after #16, so the fix rests on focused gates alone | corpus/Makefile:85 | correct-requirement-no-evidence |
| FND-005 | low | FR acceptance criteria say `Test` while `spec/tests.md` says `Integration`; the two documents use disjoint vocabularies repo-wide | spec/functional/FR-015-semantic-and-identity-parity.md:144, spec/tests.md:273 | wrong-requirement |

## Closed since SR-044 / SR-045

Verified closed, not assumed:

- **SR-044 FND-001 (high) — the two skipped gates.** Both now run. The
  migration-contract census reads all eight campaign repositories at
  `origin/main` and finds nothing unaccounted; the corpus reproduction gate runs
  the builder and it exits 0. `make test` reports 188 passed with no `s` markers
  in any file's progress line.
- **SR-044 FND-004 / SR-045 FND-001 — ungated whitespace refusal.** FR-015-AC-3
  now names "ASCII whitespace" and the gate is real. Mutating
  `src/evidence.rs:129` to admit `b' '` and `b'\n'` fails
  `tc_103_version_identity_rejects_actual_mutability_without_rejecting_metadata`
  at `tests/evidence_parity.rs:484` with `accepted version policy drifted for
  " 1.2.3"`. Source restored; tree clean.
- **SR-044 FND-003 / SR-045 FND-004 — the oracle moved with the implementation.**
  All five reclassified tokens, plus the four whitespace tokens, are now pinned
  in `evidence-version-policy.json` with both `prior_errors` and
  `accepted_errors`, and the fixture sits outside both implementations. Rust
  compares its own `identity()` to the fixture and Python compares its own to the
  same file, so the two languages agree through a third party rather than
  through each other. Both tests assert `prior_errors != accepted_errors`, so a
  row cannot degrade into a no-op.
- **SR-045 FND-002 — structural equality versus identity.** FR-015 now states it
  ("Structural equality of parsed JSON values SHALL NOT define evidence
  identity; canonical output bytes and their digest define it") and
  `ProducerAttempt` no longer derives `PartialEq`, so the ambiguous comparison
  is gone rather than documented.
- **SR-044 FND-005 — the unstated exponent-free invariant.** Named in a comment
  at `src/evidence.rs:298`.
- **SR-044 FND-002 / SR-045 FND-005 — the dishonest `Property` label.** TC-100,
  TC-102 and TC-103 are now typed `Integration`, which is accurate: the crate
  carries no property-testing dependency and these tests drive a subprocess
  reference across a language boundary. The residue is vocabulary only — see
  FND-005.

## Finding detail

### FND-001 — the fixture is a ratchet with no anchor

Measured. Truncating `evidence-version-policy.json` to its first case and
re-running both suites:

```
cargo test --test evidence_parity   ->  7 passed; 0 failed
pytest tests/test_evidence.py       ->  17 passed
```

Eight cases were deleted — every whitespace case (`" 1.2.3"`, `"1.2.3 "`,
`"1.2.3\n"`, `"1.2.3 rc"`), the `"1.*"` wildcard case, and three of the four
build-metadata cases. Nothing failed. The fixture was restored afterwards.

The `prior_errors != accepted_errors` assertion is a good anti-noop guard, but
it only constrains rows that exist. FR-015-AC-3 now names ASCII whitespace, true
wildcard/range versions and `x`-bearing immutable metadata as distinct classes,
and nothing requires the fixture to carry a case for each. A future edit that
narrows the fixture narrows AC-3's coverage silently, which is the same failure
shape the fixture was added to fix.

Cheapest close: assert the fixture covers a named set of classes — one row
tagged `whitespace`, one `wildcard`, one `build-metadata` at minimum — rather
than asserting a property of whatever rows are present. A `class` field per case
plus a set-equality assertion is a few lines in each language.

### FND-002 — the traceability population is still unstated

`make integration-traceability` reports the same untracked symbols as at
`bad4a55`:

```
TC-999 ×5, TC-1, FR-001-INV-1 ×3, TC-1598 ×2, FR-003-CON-1
```

`TC-1598` is a quire-rs identifier; `TC-999` and `FR-001-INV-1` are
`ix-trace-rs` fixtures. Both arrive through submodules with their own matrices.

This is unchanged by design — FND-094 assigned the repair to PLAN-003 TASK-023 —
and it is correctly out of this PR's scope. It is recorded here only so it does
not fall out of the record between reviews: no requirement yet says which trees
the traceability census reads, so TASK-023 will have to invent that boundary at
the moment it enforces it, and any new submodule enlarges the population again
in the meantime.

### FND-003 — publication is no longer checked anywhere

The old `revision()` docstring named the property it protected: recording a
working-tree commit "would name a revision no reviewer can fetch". Write mode
still resolves `origin/main`, so a corpus is published at the moment it is
written. Check mode now resolves the recorded SHA directly, which is the right
fix for #15 — but `git rev-parse <sha>` succeeds on any object present locally,
so a recorded revision that later stops being reachable from `origin/main`
(a force-push, a dropped branch) keeps verifying against a local object
indefinitely.

Verified not currently the case: all three recorded revisions
(`quire-contract-ir 8e0953e`, `quire-code-rs e1b7fc3`, `quoin 9fb3aa2`) are
ancestors of `origin/main` today. One `git merge-base --is-ancestor <recorded>
origin/main` per repository in check mode restores the property the docstring
claims, and would have cost nothing to add alongside the rest of this change.

### FND-004 — the corpus repository's own CI does not pass

`make ci` in qa-corpus runs the new `compatibility-corpus-selftest` (which
passes) and then `verify`, which fails with 18 mismatches:

```
MISMATCH gate-that-gates-nothing-python (agent-ix/quoin#224): external producer
drift: expected '0.21.9-76-gd3ed56a' from d3ed56a9..., got '0.23.1'
```

Reproduced identically on the parent commit `ea4ac7a`, so #16 does not cause it:
the installed quoin is newer than the revision the corpus records. The finding is
that #16's evidence is therefore the four focused gates its body lists, and the
repository's own aggregate gate is not among them. Say so in the PR body, or the
next reader will assume `make ci` was green.

### FND-005 — two vocabularies that do not join

FR-015-AC-1/2/3 read `Test (TC-100)`, `Test (TC-102)`, `Test (TC-103)`.
`spec/tests.md` types those same three cases `Integration`. Across the
repository the split is total:

| Document | Values used |
| --- | --- |
| `spec/functional/*`, `spec/non-functional/*` | Test ×122, Property ×12, Integration ×8, Inspection ×4, Static ×3 |
| `spec/tests.md` | Integration ×39, Property ×31, Static ×27 |

`Test` appears 122 times in acceptance criteria and never once in the test
matrix. So the FR column and the matrix column are not the same taxonomy, and
nothing reconciles them — which is why the previous `Property`/`Test` pair read
as a contradiction and the current `Integration`/`Test` pair reads as one too.
The remediation fixed the substantive half (the `Property` label was a claim the
repository could not support). What is left is that the two columns need either
one shared vocabulary or one sentence saying they are deliberately different
axes.

## What is correct

- The migration-contract additions are decisions, not filler. Each of the eleven
  new rows names the repositories, a KEEP/REPLACE verdict, and a reason that
  distinguishes shared Engineering Assurance qualification from repository-owned
  domain checks — `check_kani_*` and `measure_footprint.py` stay with runtime,
  `assurance_chain.py` and `check_shared_pins.py` move, and the reasons say why.
- qa-corpus #16 is the right shape of fix. It repairs the cause (check mode read
  a moving ref) rather than the symptom (the EA test failed), keeps refresh mode
  on `origin/main`, refuses a corpus that attributes one repository to two
  revisions, and ships a self-test that advances a synthetic `origin/main` past a
  deleted path and proves the recorded revision still reads the original bytes.
  The self-test is wired into `make ci`, not left as a manual script.
- `setup.cfg` was updated so the new fixture ships with the wheel; the Rust side
  reaches it through `include_str!` from the package root, and `cargo package`
  is not a concern here because the crate is `publish = false`.
- No new `unsafe`, `allow`, `unwrap` or `expect` on a caller-input path;
  `unsafe_code = "forbid"` and `missing_docs = "deny"` remain in force and
  clippy passes with pedantic warnings denied.
