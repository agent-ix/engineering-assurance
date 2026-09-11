---
id: SR-116
title: "Rust review of the compatibility observation absence refusal"
type: SpecReview
analysis: code-review
scope: "FR-012-AC-5, FR-012-AC-10, FR-015-AC-3, TC-083, TC-103, TC-130; compatibility observer, accepted-corpus refusals, fixture generator, hosted-CI pin agreement"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-012"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---

## Summary

Applied the repository conventions in `AGENTS.md` and the
`agent-skills/rust-review` checklist to the six commits resolving
agent-ix/engineering-assurance#68 against `bd6d2ce`. The change makes
`compatibility-observe` refuse a tree whose recorded artifact population it did
not verify, gives each distinguishable corpus refusal a typed discriminant,
makes the fixture generator apply the whole gate contract it names, and detects
hosted CI and the reviewed matrix pinning different Quire CLI versions. Every
non-obvious assertion was verified by mutation rather than by inspection.

## Verdict

**CONDITIONAL** — no high findings. Two low findings are recorded: one
deliberate source-inspection test, and one scope item whose premise does not
exist in this tree.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-212 | low | `tc_130_hosted_ci_installs_the_version_the_reviewed_matrix_pins` is a source-inspection test (`include_str!` over `.github/workflows/ci.yml`). Accepted under rust-review §2: the fact being asserted — which version hosted CI installs — exists only as workflow text, so no runtime path can reach it. Compile-time inclusion was chosen over a runtime read so the assertion cannot silently examine the wrong file. | tests/compatibility_cli.rs:127 |
| FND-213 | low | The issue's resource-bound scope item names `MAX_CORPUS_ENTRIES`, `MAX_CORPUS_DEPTH`, `compatibility_corpus_population_too_large`, `compatibility_corpus_tree_too_deep`, `compatibility_corpus_entry_unreadable` and a `walk` function. None exist anywhere in this tree at `bd6d2ce`. The two bounds that do exist, `MAX_INDEX_BYTES` and `MAX_RETAINED_PATH_BYTES`, were verified by mutation to fail their tests when removed or when the comparison is shifted. | src/compatibility_corpus.rs:48; src/compatibility_corpus.rs:45 |
| FND-214 | low | `compatibility-observe` is now a `make` target but is deliberately absent from `integration-gate` and hosted CI, because neither runs with `quire`, `quoin` and `ix-flow` all on PATH at their pinned versions. The Makefile records that reasoning rather than leaving it to be inferred. The drift this previously hid is now detected by TC-130 instead. | Makefile:35 |

## Checks applied

- **Idiomatic Rust.** The change removes three instances of the "variant set is
  the API, not the message" defect the skill names: a six-meaning
  `IncompleteRetention`, a nine-meaning `UnsafeRetainedPath`, and a two-meaning
  `InvalidObservation`. The catch-all `_` arm in `semantic_refusal`, which would
  have relabelled a future `CorpusError` variant with no compiler check, is
  replaced by an exhaustive match. No new `Box<dyn Error>`, `anyhow`, or
  stringly-typed error surface.
- **Visibility.** `acceptance_recorded_in` was `pub` with no caller outside its
  own module's tests and is now `#[cfg(test)]`, removing a test seam from the
  public surface.
- **Tracing.** Every new test carries a bare `ix_trace_rs::trace` attribute
  naming one TC and its criteria, matching the repository's existing form.
  `quire coverage --json` reports no `unbacked_rows` touching TC-083, TC-103,
  TC-130, FR-012-AC-5, FR-012-AC-10 or FR-015-AC-3.
- **Completeness (tests).** No assertion added here is tautological: each was
  verified by mutation. The four conjuncts of `population_verified` are driven
  individually, because asserting only that an empty tree withholds the gate
  would leave any one of them removable — which is the precise shape of the
  defect being fixed.
- **Panic surface.** No `unsafe`. The `expect` calls added are in test code or
  on `write!` into a `String`, which cannot fail.
- **Untrusted input.** No wire contract changed shape except the observation
  result, which gains fields; it is a `Serialize` struct, not a hand-built
  `serde_json::Value`.
- **Gates.** `fmt --check`, `clippy --workspace --all-targets --all-features
  -D warnings`, `test --workspace --all-targets --all-features`, `doc` with
  `-D warnings`, `make lint`, `make test`, `make package-audit` and `quire
  validate` were all run and pass. `make integration-traceability` fails on
  `main` already and was left alone.
- **CI diff.** The workflow change raises a pin to match the reviewed matrix and
  removes no lane, adds no `continue-on-error`, and drops no matrix entry.

## Mutations run

Fourteen mutations were applied and reverted; each failed at least one test.

| Mutation | Result |
| --- | --- |
| Restore the original skip of an absent artifact | 2 observer tests fail |
| Drop `hashed == recorded` from `population_verified` | fails |
| Drop `absences.is_empty()` from `population_verified` | fails |
| Drop `recorded > 0` from `population_verified` | fails |
| Drop `mismatches.is_empty()` from `population_verified` | fails |
| Swap the two `Retention::Referenced` branches | fails |
| Give two retention failures the same code | fails |
| Remove `require_all_kinds` from the fixture generator | fails |
| Revert hosted CI to the unpinned Quire version | fails |
| Shift the `MAX_INDEX_BYTES` comparison | fails |
| Remove the `MAX_RETAINED_PATH_BYTES` bound | fails |
| Report a parent traversal as a current-directory component | fails |
| Report a NUL path as a backslash path | fails |
| Swap the two blank-observation refusals | fails |
