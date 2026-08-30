# Onboarding agent evaluations

This suite defines seven fictional variants for each supported host: existing
profile reuse, no justified profile, malformed producer output, unavailable
producer execution, interruption and resume, explicit acceptance, and explicit
rejection.

Run the same suite separately for `claude`, `codex`, `opencode`, and `copilot`:

```bash
cli-evals run --suite evals/cli-agent-evals.config.mjs --all --agent <host>
```

A missing host or `cli-evals` executable is a not-executed result and keeps the
aggregate release gate failed. Do not replace a missing live run with a fixture
or deterministic command result.
