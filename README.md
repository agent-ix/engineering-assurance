# Engineering Assurance

[![Discord](https://img.shields.io/badge/Discord-Join%20us-5865F2?logo=discord&logoColor=white)](https://discord.gg/6qsdhSPE)

Engineering Assurance is the Agent IX assurance module for making engineering
decisions explicit, reviewable, and owned by a person. It gives a coding agent
one canonical onboarding skill and four resumable workflows for deciding what
assurance work a repository actually needs.

It is deliberately not a compliance oracle. Engineering Assurance does not
calculate a trust score, import external rules, or approve a release. The
decision owner remains responsible for the boundary, claims, exceptions,
evidence sufficiency, and terminal decision.

## How it fits

Engineering Assurance is a small part of the Agent IX documentation toolchain:

| Component | Responsibility |
| --- | --- |
| [quire-rs](https://github.com/agent-ix/quire-rs) | Rust engine that parses and validates Markdown documents against Quire modules. |
| [quire-cli](https://github.com/agent-ix/quire-cli) | CLI distribution of the quire-rs engine used by the validation commands below. |
| [Quoin](https://github.com/agent-ix/quoin) | Installs Quire modules and provides specification, evidence, and planning skills. |
| [ix-flow](https://github.com/agent-ix/ix-flow) | Owns resumable workflow state, event history, and human gates. |
| Engineering Assurance | Supplies assurance artifact schemas, skeletons, onboarding, and governed workflows. |

Quire-rs validates the artifacts, Quoin installs and composes the modules, and
ix-flow records the workflow lifecycle.

This module uses the Rust `quire-rs` engine through `quire-cli`

This repository is where the artifact **schemas and skeletons** are defined
and shipped — it is not where any one project's own assurance artifacts live.
Those live in that project's own repo, validated against the schemas this
module ships as an installed Quire module. `agent-ix/quoin`'s
[`spec/assurance/`](https://github.com/agent-ix/quoin/tree/main/spec/assurance)
is a working example: quoin's own `AssuranceProfile` and `MeasurementPlan`
instances. Whether quoin should also *consume* this module's evidence-accounting
and measurement code, not just its schemas, is a separate, still-open question
tracked at [`agent-ix/engineering-assurance#98`](https://github.com/agent-ix/engineering-assurance/issues/98) —
not resolved here.

## Install

### 1. Install the toolchain

These are the versions recorded in
[`engineering_assurance/compatibility-matrix.json`](engineering_assurance/compatibility-matrix.json):

The [v0.4.1 compatibility release gate](docs/compatibility-release-gate.md)
records the exact accepted matrix and the patch release's verification scope.

```bash
npm install --global \
  @agent-ix/quire-cli@0.33.0 \
  @agent-ix/quoin@0.24.1 \
  @agent-ix/ix-flow@0.2.3
rustup toolchain install 1.98.1
```

The matrix is the live source for supported versions. A newer release that the
matrix has not seen is classified `unknown`: untested, not rejected. It may
work, but it has not been checked against this module. See
[Verify your install](#4-verify-your-install) to classify what you have.

Versions are published as Git tags. GitHub Releases are not cut for every tag,
so use `git tag` or the repository's tags page, not the Releases page, to find
the newest version.

Install the native CLI from the tagged source checkout:

```bash
cargo +1.98.1 install \
  --git https://github.com/agent-ix/engineering-assurance \
  --tag v0.4.1 \
  --locked \
  --bin engineering-assurance
```

### 2. Install the Quire module

Install the module directory from the same tag. The `//engineering_assurance`
suffix selects the module root inside this repository:

```bash
quoin module install \
  github:agent-ix/engineering-assurance//engineering_assurance@v0.4.1
```

The installed module contains `manifest.yaml`, `schemas/`, and `skeletons/`.
Quire-rs consumes those files when validating assurance artifacts.

### 3. Install the agent skill

The same `assurance-onboarding` skill is available to Claude Code, Codex,
OpenCode, and GitHub Copilot. Use the section for your agent.

<details>
<summary><b>Claude Code</b></summary>

```text
/plugin marketplace add agent-ix/engineering-assurance@v0.4.1
/plugin install engineering-assurance@engineering-assurance
```

From a shell or a script:

```bash
claude plugin marketplace add agent-ix/engineering-assurance@v0.4.1
claude plugin install engineering-assurance@engineering-assurance
```

</details>

<details>
<summary><b>OpenAI Codex</b></summary>

```bash
codex plugin marketplace add agent-ix/engineering-assurance --ref v0.4.1
codex plugin add engineering-assurance@engineering-assurance
```

You can also install it from the Codex `/plugins` menu after adding the
marketplace.

</details>

<details>
<summary><b>OpenCode</b></summary>

OpenCode discovers the Agent Skills layout. Install the canonical skill with
GitHub CLI so it is available in every repository:

```bash
gh skill install agent-ix/engineering-assurance \
  engineering_assurance/skills/assurance-onboarding \
  --pin v0.4.1 \
  --scope user \
  --agent opencode
```

</details>

<details>
<summary><b>GitHub Copilot</b></summary>

With Copilot CLI:

```bash
copilot plugin marketplace add agent-ix/engineering-assurance
copilot plugin install engineering-assurance@engineering-assurance
```

If you want the skills-only route, use GitHub CLI instead:

```bash
gh skill install agent-ix/engineering-assurance \
  engineering_assurance/skills/assurance-onboarding \
  --pin v0.4.1 \
  --scope user \
  --agent github-copilot
```

</details>

### 4. Verify your install

Engineering Assurance has three installed pieces: the Quire module, the native
CLI, and the agent plugin or skill. Install all three from the same tag. A
module from one tag and a CLI from another is not detected for you, and the
skill will drive workflows through a CLI whose schemas do not match the
module.

| Piece | Check | Ready when |
| --- | --- | --- |
| Quire module | `quoin module list` | `engineering-assurance` is listed with `ref` set to the tag you installed, such as `v0.4.1`. A bare commit SHA means it came from an untagged commit. |
| Native CLI | `engineering-assurance --version` | It prints the same version. `command not found` means `~/.cargo/bin` is not on `PATH`. |
| Claude Code plugin | `claude plugin list` (or `/plugin` in a session) | `engineering-assurance@engineering-assurance` is listed as enabled at the same version. |
| Codex plugin | `codex plugin list \| grep engineering-assurance` | `engineering-assurance@engineering-assurance` shows `installed, enabled` at the same version. |

Check the plugin row for each agent you use. An agent finds a newly installed
plugin's skill only in a new session, so restart it after installing.

The `@v0.4.1` suffix on `marketplace add` pins the plugin to that tag. Without
it, the marketplace follows this repository's default branch and can move past
the tag you installed for the module and the CLI.

Then classify the toolchain against the compatibility matrix:

```bash
engineering-assurance compatibility-observe --root <engineering-assurance-checkout>
```

It prints one verdict per component and exits non-zero unless every component
is `compatible`. A component on a newer, unlisted version reports `unknown`
with the reason "not approved and not rejected". That is expected when you run
newer toolchain releases. It is not an install failure.

The `engineering-assurance` row is read from the Git tag of the `--root`
directory, not from the installed binary. Point `--root` at a checkout of this
repository at the tag you installed. Run from any other directory, that row is
unobserved, or reports that directory's own tag. Use
`engineering-assurance --version` for the installed CLI instead.

## Upgrading artifacts from an earlier release

Documents written against v0.3.x or earlier can fail the current schemas.

- **AssuranceProfile `profile_version`.** The field is now `schema_version`.
  Only a `status: retired` profile may keep `profile_version`. On any other
  profile Quire reports `{"required":["profile_version"]} is not allowed`;
  rename the field to fix it.
- **MeasurementPlan with a prose `statistical_design`.** Current plans need
  the typed estimator and decision rule. When the old design cannot be
  expressed in them faithfully, set the plan to `status: retired`, which keeps
  the prose valid, and author a current plan as its successor. Do not invent
  numeric thresholds to convert an old categorical plan.
- **Validating with `--module`.** `quire validate --module <path>` replaces
  module discovery rather than adding to it. Pass every module your documents
  use, for example this module plus `spec-artifacts-iso` and
  `spec-artifacts-process`. With this module alone, the link types its
  artifacts declare are reported as unknown.

## What it provides

### Artifact types

The module supplies Quire schemas and Markdown skeletons for five artifact
types:

| Type | Purpose |
| --- | --- |
| `AssuranceProfile` | State a decision boundary, impacts, evidence policy, and exceptions. |
| `MeasurementPlan` | Define a measure, its population, collection procedure, and interpretation. |
| `ArchitectureDescription` | Record system boundaries, views, architecture decisions, and risks. |
| `ComponentAssuranceContract` | State required component behavior, failure handling, controls, and replacement. |
| `AssuranceArgument` | Record a claim, reasoning, sufficiency decision, and challenges. |

### Agent skill and workflows

The first time `assurance-onboarding` is used in a repository, run its
`onboard` report once:

```bash
node engineering_assurance/skills/assurance-onboarding/scripts/onboard.js --repo <repo_root>
```

It explains how this module's `schemas/`/`skeletons/` relate to the target
repository's own `spec/assurance/`, links the real worked examples in
`corpus/spec/evidence/measurements/` and `agent-ix/quoin`'s `spec/assurance/`
instead of requiring a read of `quoin-measurement`'s Rust source, and lists the
fields each artifact type actually requires — derived live, straight from the
installed schemas — plus the measurement-record field contract, restated (not
derived) from `quoin-measurement`'s Rust validator.

After that, ask the agent to use `assurance-onboarding` and provide the
repository root, exact decision boundary, and human decision owner. The skill
inventories before proposing anything and preserves malformed or conflicting
existing artifacts.

It routes bounded work through these ix-flow definitions:

| Workflow | Use it for | Terminal decisions |
| --- | --- | --- |
| `assurance-intake` | Decide which assurance artifacts a bounded subject needs. | `accepted` / `rejected` |
| `architecture-evaluation` | Evaluate an architecture against declared scenarios. | `accepted` / `rejected` |
| `measurement-promotion` | Decide whether a recorded measure advances one stage. | `promoted` / `not_promoted` |
| `change-assurance` | Decide a bounded change from impact, review, and assurance records. | `approved` / `rejected` |

Example prompt:

```text
Use assurance-onboarding for this repository. The decision boundary is the
database migration in this change, and Jane Doe owns the terminal decision.
Inventory the existing assurance context before proposing work.
```

To record a measurement against a `MeasurementPlan` with `quoin measurement
record` and read it back with `quoin report`, follow the
[measurement walkthrough](docs/measurement-walkthrough.md).

Validate Markdown documents with quire-cli (the quire-rs-backed CLI):

```bash
quire validate --scope . 'spec/**/*.md'
```

## Native CLI reference

The native CLI is primarily an automation boundary. Every stdin/stdout
protocol is versioned and documented by the schemas under
`engineering_assurance/schemas/`.

| Command group | Purpose |
| --- | --- |
| `onboarding` | Inventory a repository and emit a bounded onboarding result. |
| `workflow-host` | Coordinate a bound workflow lifecycle through ix-flow. |
| `compatibility` / `compatibility-observe` | Classify explicit or locally observed toolchain versions. |
| `manifest-validate` | Qualify the module against explicit repository and registry roots. |
| `integration-evidence` | Verify Quire traceability and retained release evidence. |
| `content-rights-tree` | Inspect a Git-selected repository tree for rights violations. |
| `agent-evals` / `agent-evals-provider` | Run or serve the Engineering Assurance evaluation contract. |
| `evaluation-aggregate` / `evaluation-aggregate-verify` | Build or re-check retained evaluation aggregates. |
| `package-audit` / `package-lifecycle` | Audit private distributions and stage or refuse npm publication. |
| `workflow-invariants` | Evaluate a closed workflow projection against named invariants. |

Run `engineering-assurance --help` or
`engineering-assurance <command> --help` for argument details. Maintainer-only
commands do not make a release decision and should not be invoked as a
substitute for the human workflow gates.

## Development

```bash
make lint
make test
make package-audit
make rust-foundation-gate
make integration-gate
```

Read [CONTENT_RIGHTS.md](CONTENT_RIGHTS.md) before adding content. The
repository is public. Registry packages remain private and unpublished
until separate, explicit authorization is given.

## License

AGPL-3.0-or-later
