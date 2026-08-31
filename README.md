# Engineering Assurance

Engineering Assurance is an opt-in Quire module for authoring explicit
decision boundaries, measurement plans, component contracts, architecture
descriptions, and assurance arguments.

The module is configuration-only. It does not calculate a trust or compliance
score, embed external rules, or make a release decision. Decision owners remain
responsible for claims, exceptions, evidence sufficiency, and terminal workflow
choices.

## Local use

```bash
quire validate --scope . 'spec/**/*.md'
ix-flow run change-assurance --path engineering_assurance/skills/assurance-onboarding
```

The canonical onboarding skill is
`engineering_assurance/skills/assurance-onboarding/SKILL.md`. Claude Code,
Codex, opencode, and GitHub Copilot discovery surfaces all resolve that same
tree.

The former pilot path remains compatible for this release:

```bash
ix-flow run change-assurance --path pilots/assurance-workflows
```

The scoped Quire installation must include this module and the ecosystem's
shared relation registry.

The repository and packages are private. Opening distribution requires a
fresh authorization of both content rights and release posture.

## Development

```bash
make lint
make test
make package-audit
make integration-gate
```

`integration-gate` is reproducible from tracked repository content. It runs
rights, Ruff, pytest, manifest, package, Quire document, and traceability checks.

Real-agent reports are operational evidence and remain ignored under
`evals/reports/`; do not commit workstation paths, session output, or transcripts.
After producing and retaining a four-host aggregate in that directory, run the
complete release gate through one stable target:

```bash
make release-gate \
  EVAL_AGGREGATE_REPORT=evals/reports/aggregate-<revision>.json
```

The release gate additionally revalidates every supplied retained report and
transcript, the 28/28 complete-only aggregate, current governing files, and
current Quire, Quoin, ix-flow, and cli-evals executable identities. It fails
closed when `EVAL_AGGREGATE_REPORT` is omitted.

Read [CONTENT_RIGHTS.md](CONTENT_RIGHTS.md) before adding content.
