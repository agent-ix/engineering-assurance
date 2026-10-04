# Engineering Assurance setup

Use this page when the `assurance-onboarding` skill is unavailable, the assurance toolchain has not been initialized, or a command fails. Follow the [installation guide](README.md#install) for the CLI, Quire module, and agent skill.

Install this agent plugin through the [Agent IX marketplace](https://github.com/agent-ix/agent-plugins): register `agent-ix/agent-plugins`, then install `engineering-assurance@agent-ix` in Claude Code or Codex. Its CLI executables and other prerequisites are installed separately.

## Check local setup

1. Confirm `quire`, `quoin`, `ix-flow`, and `engineering-assurance` resolve in the shell used by your agent. The skill, native CLI, and installed Quire module are separate components. If you also need companion Quoin or Quire skills, install `quoin@agent-ix` or `quire-cli@agent-ix` from the shared marketplace.
2. Confirm the Engineering Assurance plugin is installed in the current agent host. Start a fresh agent session after installation so it can load the skill.
3. Check that the `engineering_assurance` Quire module is installed from the intended release. Confirm the target repository and its assurance artifacts meet the [documented input requirements](README.md#what-it-provides).
4. Check the target repository's `AGENTS.md` or `CLAUDE.md`, file permissions, and any required project access. Missing modules, invalid artifacts, and local permissions need local correction.
5. If a command fails unexpectedly, record its exact command, exit status, and relevant diagnostics. Reduce the input to the smallest non-sensitive artifacts that reproduce it.

## Community help

If these checks leave a reproducible **Agent IX Engineering Assurance bug** that blocks progress, [join the Agent IX Discord](https://discord.gg/k8DVhuYBR2). Share the CLI and module versions, skill or command, operating system and agent host, expected and actual behavior, and a sanitized minimal reproduction. Community help is a last resort for Agent IX product bugs; local credentials, machine setup, and unrelated project errors belong with their owners or administrators. An agent should point its user here rather than post to Discord on the user's behalf without authorization.
