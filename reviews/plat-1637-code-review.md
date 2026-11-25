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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The exact-request test is tagged to FR-036-AC-8, which requires malformed or unreadable inbound requests to return a machine error; this test instead asserts the outgoing request's five-member shape. The matrix therefore falsely credits AC-8. | src/feature_risk_rating_host.rs:1297-1349 |
| FND-005 | medium | The uncommitted rating-file test is tagged to FR-036-AC-15, which requires signature-verifier commit-id results; the test only asserts the acceptance failure reason `not_committed`. | tests/feature_risk_rating_cli.rs:316-324 |
| FND-006 | medium | FR-037-AC-1 requires successful SSH verification to report the mapped principal. The mapped-principal-mismatch and rated_by-validation tests instead test acceptance policy/input validation, so their bindings falsely credit AC-1. | src/feature_risk_rating_host.rs:1153-1209; tests/feature_risk_rating_cli.rs:300-313 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Risk-rating tests no longer carry the local `FR-027-AC-*` tags; local FR-027 atomic-publish criteria are no longer credited by these tests. |
| FND-002 | fixed | Added focused tests for exact five-member request serialization, unavailable verifier, timeout, nonzero exit, and malformed stdout. |
| FND-003 | still-open | `rustfmt --check src/feature_risk_rating_host.rs` still fails with import-order diffs at lines 15, 23 and 1087. |
| FND-004 | still-open | The exact-request test remains tagged `FR-036-AC-8` despite exercising outgoing request serialization, not malformed inbound request behavior. |
| FND-005 | still-open | The test remains tagged `FR-036-AC-15` but does not assert the verified commit id or a signature-verifier verdict. |
| FND-006 | still-open | Tests for acceptance principal matching and rated_by input validation remain tagged `FR-037-AC-1`; they do not establish the criterion's successful signature-verification output. |
| FND-003 | still-open | `rustfmt --check src/feature_risk_rating_host.rs` still reports import-order diffs at lines 15, 23 and 1087. |
| FND-004 | fixed | The request-shape test no longer carries the incorrect FR-036-AC-8 trace tag. |
| FND-005 | fixed | The uncommitted-file test no longer carries the incorrect FR-036-AC-15 trace tag. |
| FND-006 | fixed | The acceptance-policy tests no longer carry the incorrect FR-037-AC-1 trace tag. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | The workflow binds the human-gate approver to `decision_owner` and separately validates the signature principal against draft `rated_by`, but never requires these identities to match. The workflow can therefore record one person's approval for a rating signed by another person, contrary to the ticket's same-person acceptance/signing flow. | src/feature_risk_rating_host.rs:360-379; src/feature_risk_rating_host.rs:963-965 |
