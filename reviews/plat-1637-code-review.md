---
id: SR-20261115040621
title: "Code review — feature risk rating"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance; src/feature_risk_rating_host.rs, tests/feature_risk_rating_cli.rs, tests/python_port/workflows.rs, engineering_assurance/skills/assurance-onboarding/workflows/feature-risk-rating/def.yaml"
review_set: subset
---

## Summary

Reviewed PR #315 at frozen head `437d3a0372a1ef3f62496048a744786c0ef4d79f` against PLAT-1637, FR-021 and linked FR-027. Production signature verification fails closed in inspected branches, but trace tags collide with the local FR-027 and the subprocess failure/protocol boundary lacks focused test oracles.

## Verdict

**CONDITIONAL** — two medium findings need correction or explicit disposition before acceptance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Risk-rating tests reuse `FR-027-AC-*`, which resolves in this repository to unrelated atomic-publish criteria; most negative-case tests are also tagged as AC-2 despite testing other reasons. The computed matrix therefore falsely credits atomic-publish coverage and leaves the linked risk-rating criteria untraceable. | tests/feature_risk_rating_cli.rs:173-285 |
| FND-002 | medium | The focused tests exercise response classification and signed git fixtures, but do not exercise the verifier process boundary for start failure, timeout, nonzero exit, malformed stdout, or exact request shape. Regressions in the fail-closed transport and versioned request contract can pass these tests. | src/feature_risk_rating_host.rs:868-918 |
| FND-003 | low | `rustfmt --check src/feature_risk_rating_host.rs` reports formatting changes in imports in the changed Rust module. | src/feature_risk_rating_host.rs:15 |

## Review Scope

- PLAT-1637 acceptance: signed/mapped/principal match/meaning accepted; exact rejection reasons; later unsigned mutation; workflow rated_by validation; exact unrated feature list; verifier dependency boundary.
- FR-021 and linked FR-027 AC-1..AC-13: accepted signature, rejection classification, request contract, process and output failures, allowed-signers failures, malformed protocol response, and version report.
- Human gate transitions and terminal choice in the feature-risk-rating workflow.
- Rust API, input validation, process limits, scrubbed environment, CLI registration, package/workflow parity changes.

## Evidence

- `quire matrix --scope . --format json` exited 0. Local `spec/functional/FR-027-atomic-publish.md` owns FR-027-AC-1/2; the new feature-risk-rating tests use those same ids. The unmapped-signer and principal-mismatch tests (lines 251 and 270) are also tagged AC-2, and missing-meaning is tagged AC-2 (line 216).
- `verify_rating_commit_with_executable` writes a request with protocol/repository/revision/kind/allowed_signers and maps spawn failure, nonzero, and invalid JSON to `unverifiable` (lines 868-918). No focused test invokes those branches or asserts the serialized request contract.
- Workflow YAML configures both terminal transitions with `defaultGate: hitl` (lines 20-21). Rust classifier rejects protocol/kind/status/mapping/principal/meaning mismatches.
- Coder-provided focused evidence: cargo check passed; feature-risk-rating unit/CLI tests and exact workflow equivalence test passed. Reviewer ran `cargo test --locked --features full --test feature_risk_rating_cli`: 6 passed. `rustfmt --check src/feature_risk_rating_host.rs` failed with import-order diffs at lines 15, 23 and 1069. No aggregate suite run.

## Methods

Rust review was folded into code-review. Focused source and diff inspection plus Quire trace matrix were used. `cargo fmt`, clippy, and broader Rust gates were not rerun; coder reports clippy's unchanged-file failures and the PR body records focused checks.
