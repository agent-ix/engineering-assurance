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

## Dispositions

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | At `0b433be1f127208a38d6bde77038a4d13c57271f`, the Rust trace attribute and both TC-991 coverage-map rows bind to FR-070-AC-16 only. The former AC-1 association is absent from all three changed sites. The existing missing-check assertions are additional checks and do not claim AC-1 coverage. |
| FND-002 | fixed | The chained path expression is wrapped as rustfmt requires; `rustfmt --check --edition 2024 tests/receipt_workflow_contract.rs` exits 0. |

## Current review verdict

**PASS** at `0b433be1f127208a38d6bde77038a4d13c57271f`. Both original findings are fixed, and independent review of the complete PR diff found no additional defect. The original findings above remain the historical record.

The review examined the deleted main receipt caller, revised workflow inventory and FR-070-AC-16, both TC-991 rows, and the complete changed Rust integration test. `change-receipt.yml`, `tag-receipt.yml`, `tag-receipt-push.yml`, and `missing-receipt-check.yml` remain present and unchanged. The new absence assertions inspect the checkout rooted at `CARGO_MANIFEST_DIR`; the other receipt tests still parse the reusable workflow, covering its continued availability. No production Rust function, dependency revision, or public interface changed.

The requirement and tests agree on the current condition: all three protected policy files are absent, and the main receipt caller is absent. The computed Quire matrix now reports FR-070-AC-16 as tagged by `main_receipt_caller_is_absent_and_missing_check_is_declared`; FR-070-AC-1 retains its separate reusable-workflow test binding. No applicable AssuranceProfile exists under `spec/`.

## Recheck scope

| Unit | Role | Excerpt |
|---|---|---|
| FR-070-AC-16 | examined | While Engineering Assurance lacks protected workflow identities, approval policy and sink configuration, it has no `.github/workflows/receipt.yml` main-branch caller; the reusable `change-receipt.yml` remains available to adopters with those protected inputs. |
| TC-991 (FR-070 case) | examined | With protected workflow identities, approval policy and sink configuration absent, Engineering Assurance has no main-branch receipt caller; its missing-receipt caller remains scheduled/on-demand with read-only permissions |
| FR-070-AC-1 | context_only | Engineering Assurance's tests parse its checked-in `.github/workflows/release-attestation.yml` and `.github/workflows/change-receipt.yml` reusable workflows and confirm their triggers, jobs and per-job permissions match the table; each adopter's tests parse only its own checked-in caller workflows. No parsed workflow declares `pull_request_target` or `workflow_run`. |

## Recheck verification

The six tests in `receipt_workflow_contract` passed on the reviewed checkout using the machine build wrapper and the full feature set. `make lint`, `make test` (79 passed), `make package-audit`, focused rustfmt, `git diff --check`, targeted specification/review validation, and the full content rights tree check passed. The package audit accepted 62 wheel files, 61 npm files and 8 installed canonical files. Quire 0.52.0 (engine 0.62.2) reported the existing duplicate module discovery diagnostic while returning exit 0; matrix/trace also reported existing conditional-compilation symbol collisions outside this PR. No full Rust suite or dependency move was required for this workflow deletion and focused test update.
