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
the newest version. v0.5.0 is prepared but not yet tagged, so the snippets
below stay on `v0.4.1` until it is.

Install the native CLI from the tagged source checkout:

```bash
cargo +1.98.1 install \
  --git https://github.com/agent-ix/engineering-assurance \
  --tag v0.4.1 \
  --locked \
  --features full \
  --bin engineering-assurance
```

`--features full` is required: `cargo install --path .` without it exits 0 but
installs nothing, because the binary requires `full`.

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

Check the row for each of Claude Code and Codex that you use. For OpenCode or
GitHub Copilot, confirm the skill appears in that agent's skill list. An agent
finds a newly installed plugin's skill only in a new session, so restart it
after installing.

The `@v0.4.1` suffix on `marketplace add` pins the plugin to that tag. Without
it, the marketplace follows this repository's default branch and can move past
the tag you installed for the module and the CLI.

Then classify the toolchain against the compatibility matrix:

```bash
engineering-assurance compatibility-observe --root .
```

It prints one verdict per component and exits non-zero unless every component
is `compatible`. A component on a newer release the matrix has not seen
reports `newer_untested`, with the reason "not approved and not rejected". That
is expected when you run newer toolchain releases. It is not an install failure,
and it is distinct from `incompatible` and from `unknown` (absent, older than the
pin, or not a plain release).

The result also carries a `module` object comparing the installed
`engineering-assurance` Quoin module (the `version` in its `manifest.yaml`,
under `IX_CONFIG_ROOT`, when set and non-empty, or `~/.ix`) with the binary. The gate stays closed
unless they match, or if no module is found.

The `engineering-assurance` row is the version of the running binary, so it
does not depend on `--root`. Run it from any directory. A pass describes the
installed executable, not the source tree `--root` points at. It is a
build-consistency check (the binary against the matrix compiled into it), not an
observation of your environment; the `module` comparison is the environmental
signal for this package. `--root` is only checked to be a directory.

## Upgrading artifacts from an earlier release

Documents written against v0.3.x or earlier can fail the current schemas.

- **AssuranceProfile `profile_version`.** The field is now `schema_version`.
  Only a `status: retired` profile may keep `profile_version`. On any other
  profile Quire reports `{"required":["profile_version"]} is not allowed`;
  rename the field to fix it.
- **MeasurementPlan fields.** From v0.3.1 to v0.4.x:
  - `statistical_design.estimator` changed from free text to one of
    `proportion`, `count`, `mean`, `median`, `ratio`;
  - `statistical_design.decision_rule` changed from a string to an object:
    a `comparator` plus either a `threshold` or a `baseline`, and an optional
    `margin`;
  - a plan with a `statistical_design` must state its `metric`;
  - a `gate`-stage plan that is not retired must have `ground_truth_kind`,
    `negative_controls`, and `protected_apparatus`;
  - `objective`, `subject_identity`, and `preregistration` are new and
    optional.

  The [MeasurementPlan skeleton](engineering_assurance/skeletons/MeasurementPlan.md)
  shows every field in its current shape.
- **MeasurementPlan with a prose `statistical_design`.** Current plans need
  the typed estimator and decision rule. When the old design cannot be
  expressed in them faithfully, set the plan to `status: retired`, which keeps
  the prose valid, and author a current plan as its successor. Do not invent
  numeric thresholds to convert an old categorical plan.
- **Validating with `--module`.** `quire validate --module <path>` replaces
  module discovery rather than adding to it. Pass every module your documents
  use, for example this module plus `spec-artifacts-iso` and
  `spec-artifacts-process`. `spec-artifacts-iso` supplies the link types this
  module's artifacts declare; without it they are reported as unknown.

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

## Using as a library: Cargo features

**Default features are empty: enable the capability you need, for example
`features = ["source-audit"]`; the CLI needs `full`.** A bare dependency on the
crate exposes no module. Every module has its own capability feature (for
example `source-audit`, `manifest`, `package-audit`), so a consumer enables only
the modules it uses and pulls only the dependencies they need. The table of
features, modules and dependencies is at the top of the crate documentation in
`src/lib.rs`. The `engineering-assurance` binary requires `full`, so build or
run it with `--features full`. `make rust-features` checks that every feature
compiles alone, and that its unit tests compile.

Why empty: `exact-numbers` turns on `serde_json/arbitrary_precision`, and Cargo
unifies features across the whole build, so a default that enabled it would
switch it on for every crate in a consumer's workspace. Serializers other than
`serde_json` (yaml, toml, ciborium) then re-serialise a `serde_json::Value`
number as a private marker map. `evidence`, `semantics`, `evaluation` and
`evaluation-reports` imply `exact-numbers` because their digests and refusals
depend on exact numbers; nothing else does, and a consumer can opt in
explicitly with `features = ["exact-numbers"]`.

### Known duplicate crate versions

A consumer that bans duplicate versions (`cargo deny`,
`multiple-versions = "deny"`) resolves none for the default graph or for any
capability feature alone, `source-audit` included, with two exceptions.
Measured on normal and build edges over every target; `tests/duplicate_crates.rs`
fails on any other duplicate and on a row below that stops being true.
`campaign` hashes git object ids with `sha1` on the same RustCrypto `digest`
generation as `sha2`, so it adds no duplicate.

| Duplicated crate | Versions | Reached by | Owner and reason |
|---|---|---|---|
| `syn` | 2 and 3 | `manifest`, `full` | `jsonschema` 0.56 and 0.58: `strum_macros` 0.28 (syn 2, no syn 3 release), `zerocopy-derive` 0.8 via `ahash`, and on wasm targets `wasm-bindgen-macro-support`; this crate's `syn = 3` and the serde and thiserror derives are on syn 3 |
| `io-lifetimes` | 2.0.4 and 3.0.1 | `full` | `cap-std` 4.0.3 (latest): `fs-set-times` 0.20.3 is on 2, `cap-primitives` and `io-extras` on 3 |
| `windows-sys` | 0.59, 0.60 and 0.61 | `full` | `cap-std` subtree: `fs-set-times` and `winx` on 0.59, `io-extras` on 0.60; 0.61 is also ours (`rustix`, `tempfile`) |
| `windows-targets` and the `windows_*` target crates | 0.52 and 0.53 | `full` | the `windows-sys` 0.59 and 0.60 split above |

`cap-std` reaches only `full` (the `*_host` modules and the binary), and
`jsonschema` reaches only `manifest`, so no other feature inherits either.

## Development

```bash
make lint
make test
make package-audit
make rust-foundation-gate
make integration-gate
```

A bare `cargo test` runs only the tests that need no feature
(`default_features`, `duplicate_crates`; default features are empty); use `cargo test --all-features`, which is what CI runs, for the full
suite.

Read [CONTENT_RIGHTS.md](CONTENT_RIGHTS.md) before adding content.
The repository is public. Registry packages remain private and unpublished
until separate, explicit authorization is given.

## License

AGPL-3.0-or-later
