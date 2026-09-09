---
id: SR-053
title: "Independent review of the Quoin chain orchestration requirement at b257398"
type: SpecReview
analysis: code-review
scope: "PR #30 at b257398; FR-019; FR-019-CON-1..CON-4; TC-130..TC-134; TASK-024; PLAN-003"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "references"
---

## Summary

FR-019 draws the boundary the campaign needs and draws it in the right place:
Quoin keeps every schema, store, receipt and outcome, and Engineering Assurance
gets a state machine over five named subcommands with no producer execution, no
stdout verdict recovery and no inferred human decision. The refusal semantics
are unusually honest — a completed Quoin write is reported rather than rolled
back or hidden, and EC-023 exists specifically to catch a wrapper that claims
otherwise. Three gaps: the differential baseline AC-4 depends on is neither
captured nor pinned, nothing governs two chains writing one store root, and the
verification evidence in the PR body records two gates that stood down.

## Verdict

**CONDITIONAL** — no high findings.

## Gates run at `b257398`

| Gate | Result |
| --- | --- |
| `make test`, run from a git worktree | 186 passed, **2 skipped** — matches the PR body |
| `make test`, same tree content, run from the repository root | **188 passed, 0 skipped** |
| `quire`-relevant spec structure | unchanged failures only (`spec/assets/*` frontmatter, AP-201/MP-20x schema), identical on `main` |

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | AC-4 requires matching eight consumers "at pinned revisions", but no revision is pinned and no observation is retained; the scripts are being deleted by the same campaign | spec/functional/FR-019-rust-quoin-chain-orchestration.md | missing-requirement |
| FND-002 | medium | Nothing governs two chain runs against one Quoin store root, in a requirement that otherwise enumerates every failure mode and explicitly disclaims rollback | spec/functional/FR-019-rust-quoin-chain-orchestration.md | missing-requirement |
| FND-003 | low | The PR's "186 passed, 2 skipped" is two gates standing down because of the worktree's directory layout, not a clean run | tests/test_migration_contract.py:66 | correct-requirement-no-evidence |
| FND-004 | low | TC-131 and TC-132 are typed `Property` while the crate carries no property-testing dependency, reopening the label #27 corrected for TC-100/102/103 | spec/tests.md:312, Cargo.toml:25 | wrong-requirement |

## Finding detail

### FND-001 — the baseline this depends on does not exist yet

FR-019-AC-4 reads: "The Rust capability matches the retained success and
adverse-case observations of all eight current `assurance_chain.py` consumers at
pinned revisions, preserving pass, fail, unavailable, not-computed, malformed,
stale, tampered, and incomplete without rewriting historical bytes."

Three things are missing behind that sentence:

1. **No revision is pinned anywhere.** `tests/test_migration_contract.py:23`
   names the eight repositories (`quire-contract-ir`,
   `quire-contract-runtime`, `quire-contract-codegen`, `quire-analyze`,
   `tl-syntax`, `tl-parse`, `tl-mltl`, `tl-rewrite`) and reads them at
   `origin/main` — a moving ref. FR-018 speaks of a "committed consumer registry
   and candidate-revision compatibility snapshot", but FR-019 does not require
   the eight `assurance_chain.py` revisions to be recorded in it.
2. **No observation is retained.** `engineering_assurance/fixtures/` holds
   `evidence-version-policy.json` and `verification-semantics/`; there is no
   consumer fixture set. TASK-024's deliverables list "Accepted/adverse fixtures
   plus same-revision differential evidence for all eight consumers", so capture
   is inside the implementation task rather than before it.
3. **The scripts are being removed by the same campaign.** `docs/migration-contract.md`
   classifies `assurance_chain.py` REPLACE across all eight, with consumer
   migration under `quire-research#59`/`#60`.

The dependency ordering in FR-019 protects the *deletion* ("#60 may migrate the
eight consumer scripts only after this capability … pass"), but not the
*capture*. If any of the eight repositories edits or removes its
`assurance_chain.py` before TASK-024 runs, AC-4 becomes unverifiable against the
behaviour it was written to preserve, and the eight-way differential quietly
becomes a seven-way one.

`agent-ix/qa-corpus#15`/`#17` are the same lesson from three days ago: a
verification that reads a moving ref is not reproducible, and an object you did
not retain is one you may not be able to fetch. Add an AC — or a TASK-024
subtask ordered first — requiring the eight `(repository, revision, blob
digest)` triples and their observed outcomes to be recorded before any consumer
changes.

### FND-002 — one store root, no stated concurrency contract

FR-019 enumerates failure modes carefully: timeout, cancellation, signal,
malformed response, response bound to another candidate, changed input bytes,
changed historical byte. It restricts writes to an explicit store root. It
states plainly that it "SHALL NOT claim atomic rollback of a write Quoin
completed prior to the failure" — which is the right admission.

Those two facts together create the case the requirement does not address. A
chain that fails after `seal-record` leaves a real write in the store. A second
chain run — a retry, a parallel repository, a re-run after the operator fixes an
input — starts against a store that already contains a partial sequence from the
first. Nothing in the Operation Contract says whether `seal-record` is
idempotent against an existing record, whether a second run must refuse, or
whether concurrent runs against one store root are permitted at all. The
Operation Contract's "Exactly one" multiplicities are per-run, not per-store.

Quoin may well define this, in which case one sentence in Dependencies saying so
closes it. If it does not, the wrapper needs to state its own rule, because the
absence will be resolved by whatever the first implementation happens to do.

### FND-003 — two gates stood down in the reported evidence

`make test` on this tree reports 186 passed / 2 skipped from a worktree and 188
passed / 0 skipped from the primary repository checkout. The two tests locate
the campaign repositories at `REPO_ROOT.parent`, and the linked-worktree parent
directory has no such siblings.

Nothing here is wrong with #30 — the tree passes when the gates run. The finding
is that "186 passed, 2 skipped" in the PR body reads as a clean run and is
actually the migration-contract census and the corpus reproduction gate
declining to execute. That is the same class SR-044 FND-001 raised against #27
and the location dependence is the residue; it is filed in full against #29 as
its FND-001. Either state in the body that both stood down and why, or run the
suite from a checkout where they cannot.

### FND-004 — a `Property` label without a property tool

TC-131 and TC-132 are typed `Property` in the matrix. `Cargo.toml`'s
dev-dependencies are `ix-trace-rs` alone; there is no `proptest`, `quickcheck`
or equivalent. That is the exact label PR #27 corrected for TC-100, TC-102 and
TC-103, retyping them `Integration` on the grounds that a deterministic
generated corpus is not a property.

Both new cases describe enumerated adverse inputs — "unknown protocols,
escaping/aliased paths, dirty or mismatched revisions, absent/changed inputs…" —
which is a table-driven integration or unit case, not a generated one. #29's new
`spec/tests.md` note 9 draws the AC-versus-matrix distinction, but the matrix
column is the *concrete* level, so `Property` there is a claim about how the
test is built. Retype both, or state in the AC that a bounded adverse-input table
stands in for the property.

## What is correct

- **The boundary is drawn where it belongs and stated four ways.** CON-1 through
  CON-4 forbid defining or reinterpreting a Quoin schema, executing a producer
  or recovering a verdict from arbitrary output, creating a repository-local
  evidence store or canonical form, and inferring or renaming a human decision
  or Quoin outcome — and TC-134 is a static audit over all four rather than a
  behavioural test that could pass while the surface widened.
- **The five-subcommand allowlist plus the Operation Contract table.** Required
  prior state and multiplicity are given per operation, and the requirement
  explicitly rejects a skipped prerequisite, an undeclared proof or selection, an
  extra operation, and repetition outside the per-proof rows. EC-024 catches the
  way this normally erodes: an arbitrary executable or reordered graph
  re-entering through local configuration.
- **Digests are re-verified immediately before the consuming operation**, not
  only at preflight, and EC-025 covers an output that changes in between.
- **Failure semantics are honest.** No later operation runs, every completed
  operation is reported, the child is terminated, and rollback is explicitly not
  claimed. EC-023's failure mode is written as "a wrapper claims atomic rollback
  or hides the write that actually occurred" — which is the defect this kind of
  orchestrator usually ships with.
- **Implementation is deliberately absent** and TASK-024 depends on TASK-017,
  with consumer replacement held in `quire-research#60`. Specifying before
  building, and saying which serial prerequisites gate it, is the right shape.
- SR-051 records that the base review closed two high missing-boundary findings
  and one medium failure-semantics finding at the specification boundary — the
  boundary text and the rollback disclaimer both read like the product of that.
