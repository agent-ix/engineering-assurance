---
id: SR-20261113192938
title: "Code review of EA-117 source audit and release-record traces"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance; spec/tests.md; src/approval_trail_host.rs; tests/document_converter_producer_cli.rs; tests/record_sink.rs; tests/signature_verify_gitsign.rs; tests/support/approval_trail_host.rs"
review_set: subset
---

## Summary

Reviewed PR #239 at `fc35e57db999afa5d085d59c779887c54947d7df`, including the complete diff and relevant source, trace, and requirement context. The source-audit relocation is test-only and preserves the strict audit, but the new NFR-010-AC-2 trace relies on an incomplete source blacklist.

## Verdict

**CONDITIONAL** — one medium test-evidence finding remains; the code change does not weaken the source audit.

## Scope

Examined every changed file: `spec/tests.md`, `src/approval_trail_host.rs`, `tests/document_converter_producer_cli.rs`, `tests/record_sink.rs`, `tests/signature_verify_gitsign.rs`, and `tests/support/approval_trail_host.rs`. Checked all changed test-to-criterion bindings, the moved unit test, test-source audit coverage, and the NFR-010 sink boundary. No applicable AssuranceProfile was found under `spec/`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-651's finite substring blacklist can miss a signing or hash primitive imported under an alias or supplied by an unlisted API, while its new trace claims coverage of NFR-010-AC-2's broad source-inspection obligation. | tests/record_sink.rs:1287-1303 |

## Validation

- `git diff --check origin/main...HEAD` — passed (exit 0).
- Wrapped `cargo fmt --check` — passed (exit 0).
- Wrapped `cargo test --locked --all-features --test source_audit` — interrupted while compiling after lane acquisition (exit 130); no tests executed. The command ran only through the `locked-build.sh` wrapper.
- The PR body reports wrapped passes for `source_audit` (16), the approval-trail host unit test, and `record_sink` (29). I had no separate output log to inspect.
- No workflow or gate configuration files changed.
