---
id: SR-138
title: "Code review — ix-cli-kit branch pin"
type: SpecReview
analysis: code-review
scope: "Cargo.toml; Cargo.lock; .github/workflows/; src/ kit call sites; tests/producer_execution.rs"
review_set: subset
---

## Summary

Reviewed the PLAT-1140 manifest and lockfile diff against the Rust checklist, the unchanged CI workflows, and kit call sites. Cargo.lock resolves `main` to `be26cf02836e25f8ec7efa6de51e974d331a5816`. The first integration gate passed rights, package audit, documentation, formatting, clippy, and toolchain checks, then stopped on an existing producer-execution test assertion about an unchanged path dependency.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | Cargo.toml; Cargo.lock |

## Verdict

No findings in the reviewed diff. The test failure is unrelated to the ix-cli-kit source change and remains a separate baseline gate issue.

## Gate results

The explicit `make test`, `make rust-docs`, and `make rust-deps` checks passed. `make rust-audit` reported existing `zip 8.6.0` advisories, and `make integration-traceability` reported the repository's incomplete 408/429 backing. None of those packages or trace artifacts changed in this diff.
