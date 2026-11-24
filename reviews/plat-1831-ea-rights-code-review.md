---
id: SR-20261116045019
title: "PLAT-1831 engineering-assurance PR302 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance PR302 head f6af8f2; tests/content_rights_parity.rs, spec/functional/FR-017-rust-evaluation-and-qualification.md"
review_set: subset
date: 2026-11-24
---

## Summary

Ticket: PLAT-1831. Code review, with the rust-review lane, of engineering-assurance PR #302 at f6af8f28d555da0beba94ae6b0ac9f7d2d2f192d. The PR adds two FR-017 clause-source rows for the EU GMP Annex 11 set and a test for them. It also replaces the `rows.len() == 16` pin in the parity test with a non-empty check. No Rust source changed: `src/content_rights.rs` reads the FR-017 table through `include_str!` and admits a URL only on exact string equality with `scheme://host+path[?query]` at an exact member path.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**APPROVE.** No code defects.

- (1) URL match: the Annex 11 JSON on spec-clauses-eu-law main (read with gh api) contains exactly three URL occurrences. Two are the PDF URL, in `source.officialUrl` and in `rights.attribution` inside `<...>`. One is `commission.europa.eu/legal-notice_en` (scheme omitted) in `rights.licence`, inside `(...)`. The classifier's URL regex `https?://[^\s)\]>"']+` stops at `"`, `>` and `)`, so the extracted strings equal the two new rows byte for byte. The wheel path matches the repo layout. The npm path matches `stage-npm.mjs`, which stages `clauses/` at the package root, the same as the other EU rows. Matching is `url == rule.url` together with `path == wheel_member || path == npm_member`. A longer path, another or missing query, another host, or another member path is refused.
- (3) Test: `gmp_annex_11_urls_are_admitted_only_at_their_member_paths_and_query` hard-codes both URLs. Without the spec rows, both URLs would be refused at the Annex paths, so the "must admit" assertions would fail; the test is red without the rows and green with them. The PR body reports this red run, and the code path confirms it. It is the only test that does not derive its expectations from the table it checks, so it is the real guard for these rows. The refusal half partly overlaps the table-driven tests, which is acceptable.
- Count pin: the pin only counted rows in the same table that the classifier parses, so adding a row meant editing the test. The table-driven tests fail on a malformed row: the column-count assert, and the classifier returning None, which refuses every clause URL. The one case the pin also caught, a blank line inserted mid-table that truncates both parsers alike, still fails the consuming pack's release gate on its first refused URL. Removing the pin loses no protection that matters.
- rust-review: the test follows the file's idioms (URL literals split with `concat!`-style `[..].concat()` so the file's own tree scan passes, `#[trace]` tags, `expect` with a reason). `cargo fmt --check` is clean.
- Focused tests: `cargo test --features full --test content_rights_parity --test content_rights_tree_cli` gave 14 + 6 passed. The brief's command without `--features` fails because `content_rights_parity` requires `content-rights` and `content_rights_tree_cli` requires `full`.

Gap analysis did not run: the diff changes no production code, only a spec table and a test.
