---
id: SR-052
title: "Independent review of the evidence governance follow-up at dadce77"
type: SpecReview
analysis: code-review
scope: "PR #29 at dadce77; qa-corpus PR #18 at e91d0b1; evidence-version-policy fixture; NFR-005-AC-8; TC-129; spec/tests.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/NFR-005"
    type: "references"
---

## Summary

All three SR-048 findings this PR targets are closed, and the fixture fix is
better than the one the review asked for: a `class` field plus set equality in
both languages, with four unchanged-policy boundary classes added and the
`prior != accepted` assertion replaced by an explicit `policy_changed` marker
that must agree. qa-corpus #18 closes the fourth. What remains is that the two
repository gates SR-044 restored are still located by directory layout, so they
stand down in every worktree — which is where this campaign's work happens, and
is exactly what PR #30's evidence line reports.

## Verdict

**CONDITIONAL** — no high findings. One medium about where the gates can run,
two lows.

## Gates run at `dadce77`

Rust 1.98.1; submodules initialised.

| Gate | Result |
| --- | --- |
| `make lint` | pass |
| `make test`, run from the repository root | **188 passed, 0 skipped** |
| `make test`, run from a git worktree | **186 passed, 2 skipped** — see FND-001 |
| `cargo test --locked --all-features` | pass — 21 tests |
| qa-corpus #18 `compatibility_corpus_selftest.py` | pass |
| qa-corpus #18 `build_compatibility_corpus.py --check` | pass |
| qa-corpus #18 `ruff check` on both scripts | pass |

## Mutation experiments

Applied to `dadce77`'s fixture, reverted afterwards.

| # | Mutant | Rust `evidence_parity` | `pytest tests/test_evidence.py` |
| --- | --- | --- | --- |
| M4 | Delete the whole `ascii-space` class | **FAIL** — "version-policy fixture omitted or invented a governed input class" | (same class check) |
| M3 | Keep one case per class, delete the other five | ok | ok |

M4 is the defect SR-048 FND-001 named, and it now fails in both languages. M3 is
FND-002 below.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Both restored gates find their population at `REPO_ROOT.parent`, so they skip in any worktree and only gate from one directory | tests/test_migration_contract.py:66, tests/test_compatibility_corpus.py:301 | correct-requirement-no-evidence |
| FND-002 | low | Rows within a governed class are unanchored; deleting five duplicates leaves both suites green and drops the uppercase-`X` and interior-space checks | engineering_assurance/fixtures/evidence-version-policy.json | correct-requirement-no-evidence |
| FND-003 | low | SR-048, the independent review that gated PR #27, is not in the repository — #27 merged as `c47e176` without it | reviews/ | correct-requirement-no-evidence |

## Finding detail

### FND-001 — the gates work in one directory

`tests/test_migration_contract.py:66` reads `checkouts = REPO_ROOT.parent` and
skips when a campaign repository is not a sibling of the repository root.
`tests/test_compatibility_corpus.py:301` does the same through
`CORPUS_SUBMODULE.parent.parent`. Measured on this branch, same tree, same
command:

| Working copy | Result |
| --- | --- |
| the primary clone (campaign repos are siblings) | 188 passed |
| a git worktree under a scratch directory | 186 passed, **2 skipped** |

That is the whole difference. Both gates read real content in the first case and
stand down in the second, and neither is aware which happened beyond a skip
message.

This is not hypothetical drift. PR #30's verification line reads "make test
(186 passed, 2 skipped)", and PR #30 is authored in a
linked worktree whose parent directory holds other worktrees, not the campaign
repositories. Its evidence therefore records the two gates standing down, in a
PR whose own summary says nothing about it. Every agent worktree in this
campaign has the same layout.

SR-044 FND-001 recorded that a skip reported as coverage is the defect; the code
fix landed and the location dependence did not. The population needs to be
resolved from something stable — an explicit `ASSURANCE_SOURCE_ROOT` (the corpus
builder already accepts exactly that), a configured path, or a required
environment variable that makes absence a failure rather than a skip.

### FND-002 — classes are anchored, rows inside them are not

The class set is now pinned in both languages against nine names, and removing
one fails. Removing a *duplicate* within a class does not: reducing the fixture
from 14 cases to 9 by keeping the first of each class leaves both suites green.
What that silently drops is `1.2.3+X86` — the only case exercising ASCII-case
handling of the `x` marker inside build metadata — and `1.2.3 rc`, the only
interior-space case as distinct from leading and trailing.

Small, and much narrower than the hole it replaced. The cheapest close is to
require a minimum count per class where the class has a stated internal
distinction, or to give those two cases their own class names
(`immutable-x-metadata-uppercase`, `ascii-space-interior`) so the existing
set-equality assertion covers them.

### FND-003 — the review that gated #27 is not in the record

SR-048 was written, `quire`-validated and committed locally at `e6aa6cf` on
`agent-c/issue-59-evidence-rust`. The push was refused by the repository's own
pre-push hook ("HEAD lacks a matching local audit marker"), and #27 then merged
as `c47e176`. `git ls-tree refs/heads/main -- reviews/` confirms no
`26-09-09-*` artifact exists.

So the branch history has SR-044, SR-045, SR-046 and SR-047 but not the review
whose findings this PR is closing, and #29's own summary refers to findings a
reader of `main` cannot look up. It is recorded here because it is a gap in the
evidence chain rather than in anyone's work, and it is closed by landing SR-048
and this artifact together.

## Closed since SR-048

- **FND-001 (medium) — the fixture had no anchor.** Closed and improved on. Each
  case now carries `class` and `policy_changed`; both suites assert
  `(prior != accepted) == policy_changed` and then compare the observed class set
  to nine named classes. Four unchanged-policy classes were added
  (`empty-token`, `immutable-exact`, `range-operator`, `mutable-alias`), which
  the old `prior != accepted` assertion could not have expressed at all — it
  required every row to be a policy change. M4 proves the class check fails when
  a governed class disappears.
- **FND-002 (medium) — the traceability population was unstated.** Closed.
  NFR-005 now defines the population as tracked regular-file Rust blobs in the
  candidate index belonging to first-party crate, test, benchmark, example or
  fuzz targets; excludes gitlinks *without traversing them*, vendored
  dependencies, generated fixtures, retained corpus records and build output;
  requires a recorded classification, owner and reason for every exclusion; and
  fails the census on a new source root rather than silently enlarging.
  NFR-005-AC-8, TC-129, EC-022, a measurement row and a decision-table pair all
  follow, and TASK-023 is staged against them. The digest-bound manifest is the
  part that turns this from a rule into a gate.
- **FND-005 (low) — the `Test`/`Integration` vocabulary split.** Closed by
  stating it: `spec/tests.md` note 9 now says acceptance criteria use broad
  verification-method categories while the matrix `Type` column names the
  concrete test level, and that an AC's `Test` may map to an `Integration` TC.
  That is the honest reading of a repository where `Test` appears 122 times in
  ACs and never once in the matrix.
- **FND-003 (low) — corpus `--check` did not verify publication.** Closed by
  qa-corpus #18, reviewed below.

## On qa-corpus #18

`require_published_source_refs` runs `git merge-base --is-ancestor <recorded>
origin/main` per source repository in check mode, and it is better than the
one-liner SR-048 suggested:

- It separates return code 1 ("not reachable from origin/main") from any other
  failure ("cannot verify … Git could not resolve the revision"), so an
  unreachable revision and an absent object produce different diagnostics.
- The self-test proves **both** branches with real objects: a `commit-tree`
  orphan carrying the recorded tree but no ref, and a 40-`f` SHA. It asserts the
  repository path, the offending revision and the expected diagnostic text
  appear in each message.
- It deliberately does not fetch, and the docstring says why — corpus
  verification stays offline and non-mutating. The consequence, worth stating in
  the PR body rather than as a finding, is that a stale local `origin/main` can
  refuse a genuinely published revision. That fails safe, which is the right
  direction.

Verified here: `--check` passes, the self-test passes, `ruff` passes, and all
three recorded revisions (`quire-contract-ir 8e0953e`, `quire-code-rs e1b7fc3`,
`quoin 9fb3aa2`) are ancestors of their published mains.

## Still open, external

SR-048 FND-004 — qa-corpus `make ci` cannot pass in this environment, before or
after #18: `verify` reports 18 external-producer drift mismatches
(`0.21.9-76-gd3ed56a` recorded, `0.23.1` installed), reproduced identically on
that repository's `main`. Owned by `agent-ix/quoin#224`/`#227`, not by either PR.
