---
type: how-to
title: Regulated assurance path
description: Record a human-owned assurance decision with the public baseline toolchain.
---

# Regulated assurance path

Use this path to present engineering records to a named release owner. People
own risk acceptance, safety classes, evidence sufficiency, and approval.

## Intended use and boundary

Engineering Assurance supplies artifact schemas and Git-based checks for a
Baseline and signed Release. Quoin and Quire provide safety records, reports,
dossier documents, and exports.

The public baseline uses Git, GitHub rulesets and attestations, public Sigstore,
and stock `ssh-keygen`. Customer-specific integration, licensed content packs,
and a licensed validation package are outside this repository's baseline.

Engineering Assurance records and verifies inputs. People assign safety
classes and decide risk acceptance, evidence sufficiency, and release approval.
The tools do not determine product safety or compliance or qualify their own
use.

## Prepare the project

Install Engineering Assurance, Quoin, Quire, and required modules as described
in the [repository README](../README.md#install). Choose a repository boundary
and name its risk and release owners.

Author requirements, the hazard-to-harm chain, controls, estimates, and risk
policy. Record each safety class with its human assignee and rationale.

Validate the authored records with Quire and review the traceability matrix:

```sh
quire validate --scope . 'spec/**/*.md'
quire matrix --strict
```

Give Quoin a person-authored policy to report class-driven activities:

```sh
quoin class obligations --class-policy <policy-file> <scope>
```

## Assemble release evidence

Create SOUP records from the software inventory. Review the report with its
clause binding and evaluation date.

```sh
quoin soup promote <sbom-file>
quoin soup report --binding <binding-file> --as-of <RFC-3339-time>
```

Check anomalies against Git history and review the risk summary. Keep unresolved
items visible to the release owner.

```sh
quoin anomaly check <scope>
quoin risk-summary <scope>
```

Complete the change-assurance review and its signed approval tag after this
gate. The release owner reviews that approval with the remaining release
inputs.

Prepare the evidence-status, SOUP report, anomaly list, risk summary, residual-risk evaluation, risk management review, and release snapshot. Engineering Assurance reads these files as supplied.

## Record and verify the release

Use `release-history` to list changes since the previous release. Present the
release inputs through the human-gated `release-decision` workflow. The owner
chooses release or reject and names each accepted anomaly.

The workflow emits a Release record and tag message. The owner commits the
record and snapshot, signs the annotated tag, then runs `release-verify`.

Follow these steps:

1. Author the risk chain and record each human-owned decision.
2. Validate the documents and review the strict matrix.
3. Prepare SOUP records and review the report.
4. Check anomalies, then complete the signed change-assurance approval.
5. Present the Release record to its owner and verify the signed release tag.
6. Generate the dossier and export it through Quoin and Quire.

For dossier generation and export, pass each required module root with a
separate `--module <path>` option to both commands. See the known limitations
below.

## Result

`release-verify` reports whether the signed tag, Release record, Baseline, and
release commit meet its checks. The result records the person's decision.

## Request schemas

The native `engineering-assurance` CLI publishes request schemas for the
release and approval commands below. Each command prints its own closed request
contract to standard output; `--schema request` does not execute a request.
Use these commands to inspect the fields before preparing JSON:

```sh
engineering-assurance release-history --schema request
engineering-assurance release-decision --schema request
engineering-assurance release-verify --schema request
engineering-assurance approval-tag --schema request
engineering-assurance producer-execute --schema request
engineering-assurance producer-execute --schema result
```

For each of the four release and approval commands, submit JSON text with
`--request '<json>'` or pass `--request -` to read JSON from standard input.
The schema and request options cannot be combined. The `--request` value is
JSON text; a path is not opened as a file. Other commands that accept
`--request` do not publish a schema and use the request contract in their
owning requirement.

Quoin's `evidence export-test-result` requires a caller-supplied subject. Pass
the required JSON subject array with `--subject`.

## Known limitations

These behaviors were observed with Quoin 0.36.8, Quire 0.52.0, and Engineering
Assurance 0.10.1:

- `module ensure-defaults` leaves stale module references. `IX_CONFIG_ROOT` is
  ignored by `module ensure-defaults` and `module list`; use `--config-root`.
- The module registry stores absolute paths. Moving the home directory can
  make registered manifests unreachable.
- The `spec-artifacts-iso` FR skeleton marks `Dependencies` optional, but its
  manifest requires it. Follow the manifest.
- SOUP promotion omits SBOM `supplier` and `license` values. Check each
  promoted record against the source inventory.
- Dossier generation and export require a repeated `--module <path>` option
  for every module root.
