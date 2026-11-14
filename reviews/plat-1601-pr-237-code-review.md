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

## Round 3 coder disposition

| Finding | Disposition | Evidence |
| --- | --- | --- |
| FND-001 | Addressed in `bea85e4`; TC-900 through TC-910 have rows in `spec/tests.md`. | `quire matrix --strict --scope . --format tsv` binds AC-3/4/6/7/9/12; AC-13/14 are ignored-only. |
| FND-002 | Addressed in `bea85e4`; the ignored Debian scan asserts `DEBIAN-CVE-2023-4863` or its CVE alias for the exact libwebp package. | No local Debian database is available, so the live advisory match remains unverified. |
| FND-003 | Addressed in `bea85e4`; the producer request names a nonexistent executable path and asserts `unavailable` with no artifact. | Covered by the focused OSV producer test target. |
| FND-004 | Open: authentic npm, Python and Go expected advisory matches cannot be established from current fixtures without local database results. | AC-1 and AC-2 remain untagged; no scanner output was fabricated or downloaded. |
| FND-005 | Addressed by this change: the real OSV-Scanner empty-database test returned `failed` with no result artifact, while preserving stderr. The generic executor now exposes output artifacts only for completed results. | Explicitly ran with installed OSV-Scanner 2.7.1; a generic regression test writes an artifact then exits 7 and asserts the same fail-closed contract. |
| FND-006 | Test added but runtime verification remains open: bubblewrap `--unshare-net` isolates the real scanner, but the local advisory database needed for a completed scan is unavailable. | AC-8 is tagged only by an ignored test; no database or scanner output was downloaded. |
| FND-007 | Addressed by this change: the fixture replaces the upstream SBOM after staging and verifies the scanner received the original bytes at `in/sbom.cdx.json`. | TC-907 passed in the focused test target. |

The strict matrix is not complete: global summary is 186 untagged criteria and 14 ignored-only. FR-044 AC-1 and AC-2 remain untagged; AC-5 and AC-8 are ignored-only; AC-10 remains `method-without-symbol`; AC-13 and AC-14 are ignored-only. No QSL behavior or compliance claim is made.

## Round 4 coder disposition

The caller-supplied OSV archives stayed in a user-local database directory outside the repository; no archive or OSV record was added to the repository. The opt-in `make osv-scanner-integration` lane was run through `locked-build.sh` with OSV-Scanner 2.7.1 and passed all six ignored cases: Debian expected advisory, empty-database refusal, network-isolated scan, and npm, PyPI, and Go fixture matches. The fixture SBOMs were generated by Syft 1.57.1 from package manifests with the external `$schema` URL omitted; expected-result files contain package identities and advisory identifiers only.

| Finding | Updated disposition | Evidence |
| --- | --- | --- |
| FND-002 | Addressed: Debian matching accepts the report's actual `DSA-5497-2` id and the expected CVE/debian ids in the record's `upstream` field. | TC-904 passed against the supplied Debian archive. |
| FND-004 | Addressed for all three ecosystems with Syft-generated npm, PyPI, and Go SBOM fixtures and exact package purl/advisory assertions. | TC-908, TC-909, and TC-910 passed against their caller-supplied local archives. No OSV records are stored in the repository. |
| FND-005 | Addressed by the real scanner test and the shared executor output-artifact boundary. | TC-905 passed: failed state, empty result artifacts, stderr retained. |
| FND-006 | Addressed by a real scanner invocation inside bubblewrap's `--unshare-net` namespace. | TC-906 passed with the supplied Debian archive. |
| FND-007 | Addressed by the retained-input replacement test. | TC-907 passed. |

The strict matrix now binds AC-1/2, but marks these and scanner/database tests as ignored-only because they run in the named opt-in lane. Global strict summary is 184 untagged and 16 ignored-only; AC-10 remains `method-without-symbol`, so matrix coverage is not complete. Fixture results remain test behavior only and make no compliance or QSL claim.
