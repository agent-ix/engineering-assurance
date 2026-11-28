---
type: how-to
title: Regulated assurance path
description: Record a human-owned assurance decision with the public baseline toolchain.
tool_version: 0.11.1
---

# Regulated assurance path

Use this path to present engineering records to a named release owner. People
own risk acceptance, safety classes, evidence sufficiency, and approval.

## Intended use and boundary

Engineering Assurance supplies artifact schemas and Git-based checks for a
Baseline and signed Release. Quoin and Quire provide safety records, reports,
dossier documents, and exports.

The public baseline uses Git, GitHub rulesets and attestations, public Sigstore,
and stock `ssh-keygen`. Engineering Assurance records and verifies inputs.
People assign safety classes and decide risk acceptance, evidence sufficiency,
and release approval.

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
separate `--module <path>` option to both commands.

## Result

`release-verify` reports whether the signed tag, Release record, Baseline, and
release commit meet its checks. The result records the person's decision.

## Inspect command inputs

Use each command's `--help` output to see its invocation. Commands that accept
`--request` read JSON text or `-` for standard input. The release and approval
commands do not expose request schemas through a `--schema` option. The
producer-execution command publishes its request and result schemas:

```sh
engineering-assurance producer-execute --schema request
engineering-assurance producer-execute --schema result
```

For a full CLI usage reference, run `engineering-assurance --help` and
`engineering-assurance <command> --help`.

## Verify the toolchain

Check the released tool versions before using the examples:

```text
$ engineering-assurance --version
engineering-assurance 0.11.1
$ quoin --version
0.38.2
$ quire --version
quire 0.53.5 (engine 0.63.5)
```
