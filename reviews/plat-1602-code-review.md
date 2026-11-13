---
id: SR-16423
title: "code-review and rust-review of the document converter producer requests (engineering-assurance PR 222)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@406604ec; tests/document_converter_producer_cli.rs; Cargo.toml; Makefile"
review_set: subset
---

# Code review and Rust review of the document converter producer requests (engineering-assurance PR 222)

## Summary

Ticket: PLAT-1602. Diff `origin/main...HEAD` (one commit, 6 files, +525/-1). The Rust-review lane is folded in, with Agent IX rust-style loaded.

The change adds one integration test target, `tests/document_converter_producer_cli.rs`, gated on `full` like `syft_producer_cli`. It adds a `converter-integration` Makefile target that copies the `syft-integration` guard pattern. There is no `src/` change. The tests drive the Cargo-built `engineering-assurance producer-execute` binary with requests shaped per quire-cli FR-032. Stand-in converters are shell scripts written into a tempdir. The fixture Markdown is an inline fictional string. No binary, template or reference document is added. The only `include_str!` reads this repo's own published request schema, which is the same file `producer-execute --schema request` emits (`src/producer_execute_host.rs:25`).

Measured, Linux, target dir in the worktree, debug=0:
- `cargo test --locked --features full --test document_converter_producer_cli` gives 7 passed and 2 ignored (TC-374 and TC-375).
- `cargo clippy --locked --features full --test document_converter_producer_cli -- -D warnings` is clean.
- `cargo fmt --check` is clean.

Rust idioms are fine:
- The `Step` enum is closed and its `match` is exhaustive.
- Panics are `expect` with messages, which suits test code.
- The `u8::try_from` byte decode is checked.
- `TC-376` removes the inherited `EA_TEST_*` variables before it runs `make`, so it cannot recurse into a real cargo run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | TC-373 is mostly self-referential. Apart from the schema check, which `result()` also does for every other test, it compares the test's own `converter_request` output with literals copied from that same builder. Its argument check covers only `PandocDocx`, although FR-044-AC-5 lists `arguments` for every request. The `PandocTypst` and `TypstPdf` argument lists, which carry the PDF path, are never compared with the FR-032 table. Assert all three argument lists against the FR-032 table, or reduce the test to the schema check and the one fixed-field comparison. | tests/document_converter_producer_cli.rs:279-330 |

## Verdict

The code is correct and idiomatic. The focused tests, clippy and fmt pass. Nothing is vendored, and there is no pin, SHA or digest ceremony. The claim that no `src/` change is needed holds: the Syft precedent (FR-038) also has no `src/` code, and the stand-in tests pass against the existing executor unchanged. There is one low finding. The code is mergeable.
