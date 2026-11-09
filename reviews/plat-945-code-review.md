---
id: SR-16307
title: "code-review and rust-review of dead PGM-01 compatibility and bounded-report removal (engineering-assurance PR 196)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@e0cc38d1; src/semantics/mod.rs; src/semantics/pgm01.rs (deleted); src/semantics/report.rs (deleted); tests/semantics_contract.rs; tests/semantics_parity.rs; tests/python_port/migration_contract.rs; tests/fixtures/semantics-report-render.json (deleted); engineering_assurance/schemas/{assurance-report-projection-v1,pgm01-compatibility-view-v1}.schema.json (deleted); engineering_assurance/fixtures/verification-semantics/{pgm01-v1,pgm01-v2,report-projection}.json (deleted); campaign/generated/rust/src/types/{campaign_raw_artifact,campaign_run,campaign_source}.rs"
review_set: subset
---

# Code review and Rust review of the dead PGM-01 compatibility and bounded-report removal (engineering-assurance PR 196)

## Summary

Ticket: PLAT-945. Frozen head e0cc38d1, diff `origin/main...origin/plat-945-remove-dead-compat` (31 files, +117/-1873). Rust-review lane folded in (Agent IX rust-style loaded).

The Rust change is a clean deletion. `pgm01` and `report` modules, their re-exports, the two protocol constants and the seven PGM-only `SemanticErrorKind` variants are removed, and nothing left in `src/` references them. A repo-wide grep (src, tests, CLI hosts, plugin and skill bundles, docs, schemas, `setup.cfg`/`package.json` include globs, Makefile) finds no live use of `map_pgm01_bytes`, `validate_report_bytes`, `Pgm01*`, `Report{Projection,Claim,...}`, either deleted schema filename, either protocol string, or the deleted fixtures. Packaging uses directory globs (`schemas/*.json`, `fixtures/verification-semantics/*.json`), so no include list names the deleted files. Sibling checkouts show no Rust consumer: the only hit, `quire-contract-runtime/tests/shared_assurance.rs`, is an absence census that asserts `map_pgm01_bytes` is gone. No replacement ledger, pin, shim or compatibility layer is added. The `semantics` feature's dependencies (`serde`, `serde_json`, `thiserror`) are still used by the remaining module.

Measured: `cargo test --features full --test semantics_contract --test semantics_parity --test claim_strength` 17/17 pass; `cargo test --features full --test python_port migration_contract` 8/8 pass; `cargo clippy --features semantics --lib --test semantics_parity --test claim_strength -D warnings` clean; `cargo fmt -- --check` clean.

The kept ownership-registry code (`validate_ownership_registry_bytes`, `validate_embedded_ownership_registry`) has no production caller in this repo or in sibling checkouts; it is public `semantics` library API exercised only by tests (TC-053, TC-150..TC-153). It did not depend on the deleted modules, so this PR does not orphan it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Out-of-scope reformat of generated code. Three files under `campaign/generated/rust/` (excluded from the workspace and produced by codegen) were rustfmt-reflowed with no content change. This is unrelated to PLAT-945, makes the committed output differ from what the generator emits, and will be reverted by the next regeneration. Drop these hunks from the PR. | campaign/generated/rust/src/types/campaign_raw_artifact.rs:8; campaign/generated/rust/src/types/campaign_run.rs:204; campaign/generated/rust/src/types/campaign_source.rs:8 |

## Verdict

Correct and complete as a code deletion: no dangling Rust reference, no live dependent, no panic or wire surface added, focused tests, clippy and fmt clean. One low finding (unrelated generated-file reformat). Code is mergeable once FND-001 is dropped or accepted; the blocking items for this PR are in the gap-analysis and spec-review artifacts.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit d2836165 restores the three campaign/generated/rust files; `git diff origin/main d2836165 -- campaign` is empty. |
