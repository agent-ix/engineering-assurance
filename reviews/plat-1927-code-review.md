---
id: SR-2026112016223475846355062
title: "Code review of PLAT-1927 main receipt caller removal"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance PR #325 at 9f1897cfd84e5d6bbfd6d22533bf7ece782309a0; .github/workflows/receipt.yml, spec/functional/FR-070-release-and-receipt-workflow-files.md, spec/tests.md, tests/receipt_workflow_contract.rs"
review_set: subset
---

## Summary

Reviewed the workflow deletion, revised FR-070/TC-991 contract, and changed Rust integration test. The reusable receipt workflow, tag receipt workflows, and missing-receipt check remain present; one traceability mismatch and one formatting failure remain.

## Verdict

**CONDITIONAL** — one medium-severity traceability finding and one low-severity formatting finding remain.

## Assurance Context

No applicable `AssuranceProfile` was found under `spec/`; ordinary review criteria apply.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | The revised `main_receipt_caller_is_absent_and_missing_check_is_declared` test no longer checks the reusable-workflow contract in FR-070-AC-1, but it retains the `FR-070-AC-1` trace tag and TC-991 remains mapped to AC-1 in `spec/tests.md`. Remove that stale mapping/tag; keep TC-991 bound to FR-070-AC-16 (and add the missing-check criterion tag if that assertion is intentionally retained). | tests/receipt_workflow_contract.rs:382-424; spec/tests.md:351,939; FR-070-AC-1, FR-070-AC-16 |
| FND-002 | low | `rustfmt --check --edition 2024 tests/receipt_workflow_contract.rs` fails at the changed assertion because the chained `.join(...).exists()` expression must be wrapped. Format the changed test before merge. | tests/receipt_workflow_contract.rs:396-399 |

## Review notes

- `git diff origin/main...HEAD` deletes only `.github/workflows/receipt.yml` among workflow files. `change-receipt.yml`, `missing-receipt-check.yml`, `tag-receipt.yml`, and `tag-receipt-push.yml` remain tracked and present.
- The changed Rust function is a test and carries `#[trace]`; no production implementation function was changed.
- The PR reports the focused `receipt_workflow_contract` target passing (6 tests). No full repository suite was run for this review.
- `rustfmt --check` on the changed integration-test file reports the formatting failure above. Repository-wide `cargo fmt --check` also reports pre-existing formatting diffs in unrelated files; those unrelated diffs are not attributed to this PR.
