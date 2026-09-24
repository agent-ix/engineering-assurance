---
name: assurance-onboarding
description: Inventory an existing repository's assurance context and route only justified, human-owned assurance work.
license: AGPL-3.0-or-later
contributes:
  workflows: ./workflows
---

# Assurance onboarding

Use this skill when an operator asks what engineering-assurance work applies to
an existing repository or wants to enter one of the governed assurance
workflows.

## First run in a new repository: `onboard`

Run this once, before authoring anything, the first time this skill is used in
a repository:

```bash
node <skill-dir>/scripts/onboard.js --repo <repo_root>
```

(`<skill-dir>` is the directory holding this `SKILL.md`. For a module installed
with `quoin module install` it is
`~/.ix/filament/modules/engineering-assurance/skills/assurance-onboarding`; in a
checkout of this repository it is
`engineering_assurance/skills/assurance-onboarding`. Add `--json` for a
machine-readable report, or `--help` for usage.)

It prints, in one place:

1. **How the two homes relate.** This module (`engineering-assurance`) is the
   schema/skeleton **source** — `manifest.yaml`, `schemas/*.schema.json`, and
   `skeletons/*.md`. It holds no project's actual decisions. The target repo's
   own `spec/assurance/` is the **instance home** — every `AssuranceProfile`
   (`AP-*.md`), `MeasurementPlan` (`MP-*.md`), `ArchitectureDescription`
   (`AD-*.md`), `ComponentAssuranceContract` (`CAC-*.md`), and
   `AssuranceArgument` (`AA-*.md`) the project actually authors lives there,
   validated against the schemas this module supplies. The report also
   inventories what the target repo's `spec/assurance/` already has.
2. **Working examples**, so nobody has to read `quoin-measurement`'s Rust
   source (`validate/mod.rs`, `validate/stack.rs`, `types/collection.rs`,
   `types/observation.rs`) to learn the measurement-record format: the real,
   accepted measurement-collection JSON files under `corpus/spec/evidence/measurements/`
   (local path when this module's `corpus` submodule is checked out, otherwise
   a GitHub link to `agent-ix/qa-corpus`), plus a link to `agent-ix/quoin`'s own
   `spec/assurance/` — a live consumer's real, validated `AssuranceProfile` and
   `MeasurementPlan` instances, not just skeletons.
3. **What each artifact actually requires**, derived live from the installed
   `schemas/*.schema.json` (so it can never drift from what Quire enforces),
   plus the measurement-collection and observation field checklist restated
   from the Rust validator — the fields a valid record needs before it is
   submitted, not after it is rejected.

Re-run it any time the installed module version changes, or when onboarding a
different repository. This is a separate, human/agent-facing orientation step
from the native `engineering-assurance onboarding` command: that command reads
one machine protocol request and returns a structured repository inventory for
the "Inventory before proposing" step below; `onboard.js` is prose and links,
run once, to learn how this all fits together in the first place.

## Required inputs

Obtain the repository root, the exact decision boundary, and the human decision
owner. If the boundary or owner is missing, request it and create nothing.

## Inventory before proposing

Resolve every path within the selected repository root and refuse absolute,
parent-traversing, or symlink-escaping targets. Inspect before proposing and
report five separate collections:

1. existing decisions;
2. measurement definitions;
3. assurance artifacts and their Quire validation results;
4. evidence-producer configuration and evidence references; and
5. unresolved inputs or conflicts.

Reuse an applicable valid artifact. Preserve malformed or conflicting artifacts
byte-for-byte and ask the decision owner to select or correct them. Do not create
a generic AssuranceProfile or MeasurementPlan when the stated decision does not
justify one. Do not create one MeasurementPlan per related quantity, either —
two related quantities (e.g. cost and latency for the same subject) belong
under one plan's observations, distinguished by `dimensions` (see the worked
example in the `MeasurementPlan` skeleton); the duplicate-observation check
keys on metric plus dimensions together, so this does not collide.

A plan that grades a tool against recorded ground-truth labels never reports
a raw agreement/accuracy rate alone — state it next to the
constant-predictor baseline (what a fixed, corpus-blind answer would score)
and gate on the margin between them, per the worked example and formula in
the `MeasurementPlan` skeleton. Compute the baseline from the corpus at
measurement time; a hand-derived or previously-written percentage goes stale
exactly like a hard-coded label would, and picking one constant across every
answer-space family instead of one per family silently understates it.

When the decision owner states which way the metric should move, record it in
the plan's optional `objective` block rather than only in prose:
`direction` is one of `higher`, `lower`, `zero`, or `target`, and `bound` is
the goal the metric should reach, required when `direction` is `target` (see
the `MeasurementPlan` skeleton). `bound` is informational and never evaluated;
only `decision_rule` is. Do not invent an objective the owner has not stated.
The objective may also carry optional steering fields, in units the plan's
body states: `weight` (relative value, non-negative), `value_half_life` (how
fast the value of improving decays, strictly positive), and `budget` (time,
tokens, or compute per attempt, non-negative). They are advisory only: they
never gate and never feed `decision_rule`. Record them only when the owner
states them.

A plan with a `statistical_design` states its bar as data a checker can
evaluate, not as a sentence: `estimator` is one of `proportion`, `count`,
`mean`, `median`, or `ratio`, and `decision_rule` is
`{ comparator, threshold }` or `{ comparator, baseline, margin? }`, where
`comparator` is one of `gt`, `ge`, `lt`, `le`, `eq` and `baseline` is one of
`constant-predictor` (only with `estimator: proportion`), `prior-collection`,
`best-seen` (not with `eq`), or `external-reference` (a value the calling
checker resolves from a source outside the plan, e.g. an owner-declared
budget; no estimator restriction, and usable with `eq`). `margin` is in the
metric's own units and
signed in the direction of improvement: positive demands the result beat the
baseline by that much, negative allows a regression of up to that much; `eq`
takes none. When `objective` is present, the comparator agrees with it:
`higher` takes `gt`/`ge`, `lower` takes `lt`/`le`, `zero` takes `eq` or `le`
against threshold 0. The rule applies to the plan's own `metric`, which is
then required; do not restate `metric`, `repetitions`, or
`minimum_population` inside the rule. Gate on the constant-predictor margin as
`{ comparator: gt, baseline: constant-predictor, margin: <owner's bar> }`, and
do not invent a threshold or margin the owner has not stated. `population`,
`sampling`, `error_model`, and `uncertainty` remain prose.

List the files that produce the plan's number in `protected_apparatus`: the
harness, the labels, corpus, or answer key, the file that selects the
population, and the checker configuration. Each entry is a repository-relative
file path or a directory entry ending in `/**` (every file under that
directory, recursively); no other wildcard is allowed, `**` alone is refused,
and so are absolute paths, `.`/`..` segments, empty segments, control
characters, and `\ ? [ ] { } :`. The list is non-empty with no repeats, and a
gate-stage plan, or any plan declaring an `apparatus-edit` control, must have
one. Quoin's intake resolves each entry (case-sensitive, dotfiles included,
symlinks refused, a directory entry must hold at least one file, an entry that
names nothing refuses the collection) and records the (path, digest) set; any
difference in that set is an apparatus change. A change that edits a
protected file changed the measurement, not the thing measured, and earns no
credit. Protect the population through the file that selects it;
`statistical_design.population` stays prose.

A gate-stage plan also declares at least one `negative_controls` entry,
`{ kind, description }`: a gaming scenario the plan says it guards against and
how. `kind` is one of `suppressed-observation`, `gain-within-noise`,
`stale-evidence`, `apparatus-edit`, or `selective-reporting`. The declaration
is checked for shape only; Quoin's checker exercises the kinds it can detect.
Declare only controls the owner has stated.

The objective's `direction` and `bound`, `estimator`, `decision_rule`, and
`protected_apparatus` list are all part of the measurement definition (the
objective's steering fields are not; retuning them needs no new version): when you add, remove, or change
any of them on an existing plan, also change `definition_version`, so results
under the old and new definition are never compared as one series. Editing a
protected file needs a new `definition_version` too; Quoin detects that edit
through the file's digest.

When an artifact is justified, render it from the installed module skeleton,
write a same-directory staging file, validate it with Quire, and expose it only
with an atomic rename after validation succeeds. A failed validation must leave
the intended path absent.

## Workflow routing

Use the definitions under `workflows/`:

- `assurance-intake` for a bounded profile decision;
- `architecture-evaluation` for a scenario-based architecture decision;
- `measurement-promotion` for a one-stage measurement decision; and
- `change-assurance` for a bounded change decision.

Run them through ix-flow and preserve its state for resume. Terminal transitions
remain named human decisions; never acknowledge or override those gates.

Drive the run through the `engineering-assurance workflow-host` command, which
reads one `engineering-assurance.workflow-host/v1` request on standard input and
writes one result on standard output. Use its `start_or_resume` operation to
create or resume the stable run and its `run_binding` item. Record interviews
and evidence items with ix-flow itself. At `decision_ready`, use the `decide`
operation only after the bound owner supplies an explicit `accept` or `reject`;
send no choice to leave the run non-terminal.

### Measurement promotion and the checker's verdict

An AssuranceProfile may carry `measurement_policy: { mode, stages }`, where
`mode` is `recommend` or `require` and `stages` is a non-empty list of distinct
MeasurementPlan stages. In a `measurement-promotion` run:

- Record the governing profile's policy as one `measurement_policy` item
  (`profile_path`, `mode`, `stages`) before evidence. Record nothing when the
  profile has none: every stage is then `recommend`.
- Record the independent checker's `quoin.measurement-verdict.v1` document
  unchanged as a `measurement_verdict` item, and bind the `promotion_evidence`
  item to it with `plan_id` (the plan's frontmatter `id`) and `candidate` (the
  collection id the checker decided). The verdict must be for the evidence's
  `definition_version`.
- The verdict comes from `quoin measurement verify`. Before planning a
  `require` stage, check that `quoin measurement --help` lists `verify`.
  Without it no verdict can be recorded, so a `require`
  stage refuses with `promotion_checker_missing`. Tell the owner, and offer
  `recommend` or an owner-stated `exception` instead.
- `recommend`, and any stage the policy does not list: `measurement.promotion_ready`
  never refuses on the checker, including when no result was recorded. Tell
  the decision owner what the recorded verdict says.
- `require` for the proposed stage: the promotion is refused unless the bound
  result says `accept` and its `orderSource` is `git-first-parent-add`.
  The codes are `promotion_checker_missing`, `promotion_checker_mismatch`,
  `promotion_checker_not_accepted`, and `promotion_checker_order_unattested`.
- The owner overrides a refusal only with a current `exception` item (owner,
  expiry, rationale, impact), which stays in the run as the record of the
  override. Never record an exception the owner has not stated.
- An accepted verdict does not promote. The owner still decides at the
  human-gated terminal transition. Recommend `require` at `gate` only.
