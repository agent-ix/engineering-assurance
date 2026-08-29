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

Read [CONTENT_RIGHTS.md](CONTENT_RIGHTS.md) before adding content.
