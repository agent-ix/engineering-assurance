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
node <installed-module-root>/engineering_assurance/skills/assurance-onboarding/scripts/onboard.js --repo <repo_root>
```

(`<installed-module-root>` is wherever the module was installed per
`engineering_assurance/INSTALL.md` — the same bundle root every host's plugin
manifest resolves. Add `--json` for a machine-readable report.)

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
