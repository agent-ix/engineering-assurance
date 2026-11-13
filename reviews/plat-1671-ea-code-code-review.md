---
id: SR-20261113055256
title: "code-review and rust-review of the licence-bearing npm fixture and TC-377 (engineering-assurance PR 224)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@870c8d19; tests/syft_producer_cli.rs; tests/fixtures/licence-npm/expected.json; tests/fixtures/licence-npm/scan/package.json; tests/fixtures/licence-npm/scan/package-lock.json; spec/tests.md"
review_set: subset
---

# Code review and Rust review of the licence-bearing npm fixture and TC-377 (engineering-assurance PR 224)

## Summary

Ticket: PLAT-1671. Frozen head 870c8d19, diff `origin/main...HEAD` (5 files, +64/-0). Rust-review lane folded in (Agent IX rust-style loaded). The change is test code and fixture data only. It adds no production code.

Measured in a separate detached worktree, with its own target dir that was deleted afterwards:

- `EA_TEST_SYFT=<syft 1.57.1> make syft-integration`: 6/6 pass (TC-300..TC-304 and TC-377).
- A default `cargo test --features full --test syft_producer_cli` with `EA_TEST_SYFT` unset: 5 pass and 6 ignored. The Syft-free tests stay Syft-free.
- `cargo clippy --features full --test syft_producer_cli -D warnings`: clean. `cargo fmt --check`: clean. `make content-rights-tree`: accepted, with no findings.

Mutation checks on `expected.json` and the lock file, each reverted afterwards:

- The test fails when the field for an `id` entry is changed to `expression`, when the `id` text is wrong, when the unlicensed entry is expected to be `id`, when a licensed entry is expected to be `none`, when the `field` key is missing, when an entry is dropped from the expected file (count mismatch), and when every `license` is stripped from the lock (the "no component carries licenses" guard).
- The test still passes when an `expression` entry's field name is misspelled (`expresion`). See FND-001.

The test runs through `producer-execute` with the FR-038 request: `sbom_for_source` → `syft_request` → `completed_sbom`. That path also schema-validates the request (NFR-006-AC-2).

The fixture uses fictional `fx-*` names and SPDX ids only (MIT, GPL-3.0-only, Apache-2.0, `MIT OR BSD-3-Clause`, and one entry with none). It holds no licence or standard text. The diff adds no digest, pin or SHA, no local path and no vendored content. The tests.md row has the six-column format of its neighbours.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `match field` catch-all `_ => &entries[0]["expression"]` treats any field value other than `id` (a typo, or an unknown name) as `expression`, so a malformed expected-result file with a misspelled `expression` still passes. FR-039-AC-8 allows only `id` or `expression`. Match `"expression"` explicitly and panic on any other value. | tests/syft_producer_cli.rs:407-410 |

## Verdict

The test is real. It can fail on wrong licence fields and wrong licence text, and on a missing or extra licence, because each was shown by mutation against real Syft. One low finding: unknown field names fall through to `expression`. The code is mergeable, and FND-001 is a small hardening that can be fixed in the PR or accepted.
