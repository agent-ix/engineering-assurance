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
justify one.

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

Use `engineering_assurance.workflow.start_or_resume` to create or resume the
stable run and its `run_binding` item. Record interviews and evidence items with
ix-flow itself. At `decision_ready`, use `engineering_assurance.workflow.decide`
only after the bound owner supplies an explicit `accept` or `reject`; pass no
choice to leave the run non-terminal.
