---
id: SR-117
title: "Review of the external-provider absent-optional-metadata fix"
type: SpecReview
analysis: code-review
scope: "FR-017-AC-9, TC-136, EC-018; the native agent-evals external-provider describe response"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

## Summary

`make agent-evals EVAL_AGENT=codex EVAL_RUN=canary` refused the described
scenario catalogue before any agent host started. `ProviderScenario` in
`src/agent_evals_provider.rs` was the only serialized structure in that module
without `skip_serializing_if = "Option::is_none"`, so a `describe` response
serialized an absent use case, title, or canary flag as JSON `null`:

```json
{"id":"EA-002","use_case":"no-profile","title":null,"canary":null}
```

The `cli-agent-evals.external-provider-result/v1` contract treats those three
fields as optional: the consuming validator accepts the field absent, or a
`string`/`boolean`, and refuses `null`. One refused field refuses the whole
catalogue, so no host-scenario cell could be produced and the retained
aggregate release gate could not advance.

This is the same defect class as the closed FND-158 in SR-090, in the opposite
direction: there Rust wrongly *accepted* a null the published TypeScript
contract forbids; here Rust wrongly *emitted* one.

The fix is the serialization attribute the rest of the module already applies.
No timeout was weakened or removed, no scenario was mirrored in JavaScript, and
the native host remains the only launcher.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-217 | high | Closed. `describe` serialized absent optional scenario metadata as JSON `null`, which the external-provider contract refuses, blocking every native agent evaluation. The three optional fields now skip serialization when absent. | `src/agent_evals_provider.rs:110`; `tests/agent_evals_provider_cli.rs:311` | missing-requirement |
| FND-218 | medium | Closed. No acceptance criterion required the emitted response to omit optional metadata it does not hold, so the passing provider tests asserted decoded values and never the emitted bytes. FR-017-AC-9 and TC-136 now state and bind that obligation. | `spec/functional/FR-017-rust-evaluation-and-qualification.md:537`; `spec/tests.md:287` | missing-requirement |

## Inherited gate failure

`make rust-foundation-gate` does not pass at `main` (d9d3023). TC-128
`minimal_downstream_compiles_only_producer_execution_feature` fails because the
library exposes `evaluation` and `workflow` — and their `time` dependency —
outside the full feature set. This change touches no library module, no
`Cargo.toml`, and no feature declaration: `src/agent_evals_provider.rs` is a
binary-only module declared in `src/main.rs`, so the failure is unchanged by
this branch and is not weakened, skipped, or worked around here. It is being
resolved separately on `agent-c/issue-34-producer-execution`.

## Scope limits

This review covers the describe-response serialization boundary only. It does
not claim TC-109 completion, a four-host aggregate, release evidence, or that
any other provider operation was re-reviewed.

## Gate results

| Gate | Result |
| --- | --- |
| `make lint` | pass |
| `make test` | pass (35 passed, 2 pre-existing skips) |
| `make rust-foundation-gate` | rust-format, rust-clippy and rust-toolchain pass; rust-tests fails only on the pre-existing TC-128 minimal-consumer feature-gating defect inherited from `main` (`src/evaluation.rs` and `src/workflow.rs` are not confined to the full feature), so rust-docs, rust-deps and rust-audit did not run |
| `cargo test --test agent_evals_provider_cli` | pass (6 tests) |
| TC-136 without the fix | fails on the emitted `title` null (regression guard verified red) |
