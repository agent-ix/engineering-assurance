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

Retain a targeted rerun when one cell times out instead of discarding the other
passing cells. Assemble explicit report paths with the complete-only gate:

```bash
make agent-evals-aggregate \
  EVAL_REPORTS="evals/reports/claude-all.json evals/reports/opencode-all.json evals/reports/opencode-ea001-retry.json" \
  EVAL_AGGREGATE_REPORT=evals/reports/aggregate.json
```

Failed attempts remain in the aggregate diagnostics. A targeted passing retry
may fill its missing cell, but the gate rejects two passing results for one cell,
mixed models for one host, source or governing-version drift, changed transcript
bytes, malformed envelopes, and every incomplete matrix.

When `cli-evals` omits the optional model field, the aggregate records
`runner-default` as the selection mode. Mixing that mode with an explicitly
selected model for the same host remains a model mismatch.
