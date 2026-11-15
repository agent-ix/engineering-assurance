---
id: SR-20261115040601
title: "Code review — EA-108 receipt workflow and FR-070 action references"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance#280; .github/workflows/change-receipt.yml; .github/workflows/receipt.yml; .github/workflows/missing-receipt-check.yml; tests/receipt_workflow_contract.rs; tests/release_attestation_workflow.rs; tests/python_port/module.rs; FR-070-AC-3"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-070"
    type: reviews
---

## Summary

Reviewed the EA-108 workflow diff and its Rust workflow-contract tests, including Peter's ruling that SHA pins are optional and untested by default. The workflow structure is consistent with that ruling, but the claimed action-reference structural check accepts malformed references.

## Verdict

**CONDITIONAL** — one medium finding leaves the new structural validation weaker than FR-070-AC-3 claims.

## Scope

Examined all seven changed paths, the workflow permissions and identity boundaries, FR-070-AC-1..6, FR-074/075/076 behavior referenced by the new workflows, and the focused Rust tests. No AssuranceProfile applies. The deliberate `statements_not_retained` limitation is stated in the PR and was treated as deferred EA-123 integration work, not as an accidental stub.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `assert_actions_are_structural` accepts malformed action references such as `@v4`: it checks only that a split on `@` has a nonempty suffix, without validating the action path or reference grammar. Thus a future invalid `uses:` value can pass the test despite FR-070-AC-3 requiring structurally valid action references. | tests/receipt_workflow_contract.rs:25-32 |

## Validation

- `cargo test --locked --features full --test receipt_workflow_contract --test release_attestation_workflow` through `locked-build.sh` — passed (8 tests).
- `cargo test --locked --features full --test python_port packages_are_private_and_have_no_release_configuration` through `locked-build.sh` — passed (1 test).
- `cargo clippy --locked --features full --test receipt_workflow_contract --test release_attestation_workflow --test python_port -- -D warnings` through `locked-build.sh` — passed.
- `rustfmt --edition 2024 --check` on the three changed Rust test files — passed.
- `git diff --check origin/main...HEAD` — passed.
- `actionlint` — unavailable on this machine.
- `codex review --base origin/main` could not initialize its in-process app-server because the filesystem is read-only; the review was completed by direct inspection and focused checks.
