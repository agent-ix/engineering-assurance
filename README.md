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
```

After producing the retained four-host aggregate, run the complete release gate
through one stable target:

```bash
make integration-gate \
  EVAL_AGGREGATE_REPORT=evals/reports/aggregate-<revision>.json
```

The gate runs rights, Ruff, pytest, manifest, package, Quire document, and
traceability checks; then it revalidates every retained report and transcript,
the 28/28 complete-only aggregate, current governing files, and current Quire,
Quoin, ix-flow, and cli-evals executable identities.

Read [CONTENT_RIGHTS.md](CONTENT_RIGHTS.md) before adding content.
