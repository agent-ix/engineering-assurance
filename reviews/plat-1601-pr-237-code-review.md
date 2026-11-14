---
id: SR-20261113192938
title: "Code review of the OSV-Scanner producer harness (PR 237)"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance; Makefile, tests/osv_scanner_producer_cli.rs, FR-044"
review_set: subset
---

## Summary

Reviewed PR #237's Makefile integration lane and Rust producer-execution harness, including its FR-044 test traceability and acceptance-criterion coverage. The harness exercises fixture execution, but the reused test identifiers and missing behavioral assertions leave several stated criteria unverified.

## Verdict

**CONDITIONAL** — medium findings remain in test traceability and acceptance-criterion coverage; the full Rust foundation gate also stops on source-audit failures outside this diff.

## Review method and evidence

The change adds only `Makefile` and `tests/osv_scanner_producer_cli.rs`; no production source or spec files change. Rust review covered the request shape, subprocess fixture cases, database validation, ignored scanner-backed case, test assertions and trace tags. `quire matrix --strict` exits 1; its FR-044 rows leave AC-3 through AC-9, AC-11 through AC-14 and CON-1 untagged. It reports the new tags as `criterion-tagged-outside-row`; the repository already assigns TC-378 through TC-382 to unrelated Baseline and release tests in `spec/tests.md` lines 749–753.

`make lint` passed. `make test` passed (78 passed). The locked `make rust-foundation-gate` passed format, Clippy and workspace/all-target/all-feature check, then failed in `tests/source_audit.rs`: `tc_127_producer_execution_owns_the_only_first_party_process_capability` reports `src/approval_trail_host.rs` outside the producer executor, and `tc_117_every_first_party_rust_test_uses_canonical_trace_syntax` reports the existing missing trace criterion in `tests/document_converter_producer_cli.rs`. The new OSV test target ran within that attempt: 4 passed, 1 ignored. Remaining foundation-gate steps did not run after the test failure. `make package-audit` passed through `locked-build.sh` (61 wheel files, 60 npm files; accepted). `rust-docs` passed. The first `cargo deny --locked check` attempt could not acquire the default Cargo advisory-database lock because that path is read-only; rerunning through `locked-build.sh` with a temporary writable `CARGO_HOME` passed. `cargo audit` passed with one allowed unsoundness advisory, RUSTSEC-2026-0486 for rustix 1.1.4.

The ignored live-scanner case was not run: this machine has no local Debian `all.zip`. Advisory matching remains unverified. The tests make no compliance claim, and QSL was not tested or claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | New OSV tests reuse TC-378 through TC-382, which already identify unrelated Baseline and release cases; Quire therefore recognizes no FR-044 bindings for these tests. | tests/osv_scanner_producer_cli.rs:241,320,364,387,428; spec/tests.md:749-753 |
| FND-002 | medium | The Debian fixture scan accepts a report with no vulnerabilities; it asserts package identity only and does not test FR-044-AC-14's expected advisory match. | tests/osv_scanner_producer_cli.rs:456-484 |
| FND-003 | medium | The AC-3 case runs an existing fixture executable that exits 127, which tests a started process failure rather than a request whose executable path does not exist and yields `unavailable`. | tests/osv_scanner_producer_cli.rs:322-327 |
