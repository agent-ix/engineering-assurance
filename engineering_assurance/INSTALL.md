# Installation

Module installation and agent-plugin installation are separate operations. The
module supplies Quire's `manifest.yaml`, `schemas/`, and `skeletons/`; the plugin
surfaces expose the canonical `assurance-onboarding` skill.

## Local-source module installation

From a trusted working copy, install the Python module into an isolated target:

```bash
python -m pip install --no-deps --target <module-root> .
```

For the npm package, install the local archive or working copy into the
selected project without publishing it:

```bash
npm install --ignore-scripts <local-package-source>
```

Point Quire at `<module-root>/engineering_assurance` for the Python installation,
or at the installed npm package root. Both retain `manifest.yaml`, `schemas/`,
and `skeletons/`.

## Repository-source module installation

Obtain a checkout of the public repository, pin its revision, and install that
checkout into an isolated target:

```bash
python -m pip install --no-deps --target <module-root> <repository-source>
npm install --ignore-scripts <repository-source>
```

Record the selected revision with the consuming repository. A registry package
is a distinct distribution source and must not be substituted unless its
published identity is explicitly selected and pinned.

## Local-source agent-plugin installation

Use the same trusted bundle root for every host. Claude Code reads
`.claude-plugin/plugin.json`, Codex reads `.codex-plugin/plugin.json`, opencode
reads `opencode.json`, and GitHub Copilot reads `.github/plugin/plugin.json`.
Each manifest resolves `engineering_assurance/skills/assurance-onboarding`.

For path-mode verification before registering a plugin, run:

```bash
ix-flow run change-assurance --path engineering_assurance/skills/assurance-onboarding
```

Register that bundle root using the selected host's local-plugin command. Keep
the bundle intact; do not copy `SKILL.md` into four host-specific trees.

## Repository-source agent-plugin installation

Check out the public repository at a pinned revision, then register the checkout
root with each host's repository or local-path plugin mechanism.
The four host manifests must remain beside the canonical
`engineering_assurance/skills` directory they reference.

The compatibility path remains available during this release:

```bash
ix-flow run change-assurance --path pilots/assurance-workflows
```
