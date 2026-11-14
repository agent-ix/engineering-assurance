---
id: SR-20261113192970
title: "Code review — engineering-assurance receipt verification and missing-receipt implementation"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance; Cargo.toml, src/approval_statement.rs, src/attestation_signing.rs, src/lib.rs, src/main.rs, src/receipt.rs, src/receipt_host.rs; FR-070, FR-074–078, NFR-011"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance"
    type: references
---

## Summary

Reviewed EA-108 PR #272 at frozen head `069bef2a84a297cb0b4531620cc8dee0f1132562`, including the changed Rust API, command host, tests, and the receipt acceptance criteria. The review found a duplicate protocol schema, a false-passing receipt path, approval-row consistency gaps, grace/idempotency defects, and incomplete test evidence.

## Verdict

**FAIL** — the verifier can accept a receipt that records a failed Test Result as verified-passing, and the code duplicates an existing protocol model.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Receipt code redeclares the in-toto Verification Summary statement and predicate types already defined in `src/attestation_verify.rs` (including verifier, policy, input-attestation, and digest records). Keep one shared representation; the duplicate can drift from the existing verifier contract. | src/receipt.rs:689-742 |
| FND-002 | high | `verify_receipt` validates the Test Result’s own `FAILED` shape, but derives the summary result only from the change-record status. A signed Test Result with `result: FAILED`, a change record with `status: passed`, and a `PASSED` summary therefore verifies as `Verified`; reject this contradictory receipt so a failed gate cannot be represented as passing. | src/receipt.rs:498-557 |
| FND-003 | medium | The verifier compares only sets of statement digests and role names. It never proves that each change-record approval row’s digest belongs to a verified statement with that row’s role, and set comparison also tolerates duplicate rows for one role. Two valid role statements can be swapped between rows and the receipt still verifies; enforce one-to-one role/digest correspondence. | src/receipt.rs:572-623 |
| FND-004 | medium | The age/grace check skips every state before classification. A `VerifiedFailed` receipt pushed less than `grace_minutes` ago is therefore omitted from `failed_receipts`, even though it is already a valid recorded failure. Apply grace only to unresolved receipt states. | src/receipt.rs:198-213 |
| FND-005 | medium | The store operation treats any existing note as `receipt_note_conflict`; it does not verify an existing note and take the required idempotent skip path. Retrying a receipt workflow after a successful note write therefore exits as an error instead of recognizing the existing verified receipt. | src/receipt_host.rs:985-996 |
| FND-006 | medium | The production `receipt-verify` request supplies raw approval rows and `RequestApprovalReader` treats those caller-provided rows as the retained private-store result. No store read or retention check occurs, so a caller can make verification depend on copied signed proofs outside the configured retained source. Keep the production command fail-closed as `statements_not_retained` until it uses the backend tracked by EA-123. | src/receipt_host.rs:798-815,830-963 |
| FND-007 | medium | The test tagged to FR-076-AC-1 writes and reads a local Git note, but never performs a second push or exercises the default-branch notes ref update. The matrix marks the whole AC tagged although the asserted behavior is only a subset; add real coverage for the missing push/survival behavior when that integration path is implemented. | src/receipt_host.rs:1319-1350 |

## Review evidence

The Rust review lane inspected repository guidance, API surfaces, parsing and resource bounds, process boundaries, error mapping, and test assertions. The change contains no `TODO`, `FIXME`, `todo!()` or `unimplemented!()` in the reviewed files. The coder reports focused tests, compile checks, and content-rights passing; the aggregate gate was not run before the independent review, per the lead’s gate order. No applicable AssuranceProfile was found.

The source-workflow identity comparison remains covered by an existing explicit-claims test in `src/attestation_signing.rs`; this review did not rerun build gates.
