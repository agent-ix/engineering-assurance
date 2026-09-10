---
id: SR-080
title: "Rust review of npm package lifecycle"
type: SpecReview
analysis: code-review
scope: "FR-017-AC-3, FR-017-AC-4, FR-017-CON-2, FR-017-CON-3, TC-111, TC-112; src/package_lifecycle.rs; src/package_host.rs; src/main.rs; tests/package_lifecycle_cli.rs; package.json"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

## Summary

The repository rules and the exact `agent-skills/rust-review/SKILL.md`
checklist were applied to the Rust npm package-lifecycle protocol, confined
filesystem adapter, CLI rendering, and declarative npm hooks. The implementation
preflights a fixed source population, refuses links and special/non-portable
entries, streams bounded SHA-256 identities, stages through create-new writes,
verifies every result byte, rolls back only invocation-created top-level
destinations, and deletes a staged population only after complete current-source
correspondence.

Direct CLI invocations emit one versioned JSON result. Explicit npm-hook mode
leaves stdout empty so npm retains its own machine-output channel. The npm
archive remains the accepted configuration/artifact bundle and does not claim
to distribute the native Rust executable. Six review findings were corrected;
no open Rust finding remains in this slice.

This review does not claim the Rust archive audit, installed-bundle integration,
complete host-configuration census, evaluation-host interface, invocation
cutover, or final Python removal.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-173 | high | Closed through `/specify`: the retained MJS cleanup used force-recursive deletion without proving that package-root paths still corresponded to staged source. Rust cleanup now preflights the complete fixed population and deletes nothing on missing, extra, linked, special, or changed members. | `src/package_host.rs`; `tests/package_lifecycle_cli.rs`; FR-017 Behavior; TC-111 |
| FND-174 | high | Closed: inspecting selected children with `symlink_metadata` did not detect a symlink at the intervening `engineering_assurance/` module root. The adapter now validates the module root itself before traversal; removing that check makes the linked-root adverse case fail. | `src/package_host.rs`; `tests/package_lifecycle_cli.rs`; FR-017-CON-3 |
| FND-175 | high | Closed after a real integration failure: emitting the direct lifecycle JSON from `prepack` added a second document to `npm pack --json`, causing the retained package audit to fail. An explicit hook rendering mode now produces empty stdout for stage/clean, preserves refusal on stderr/status, and the real npm pack/install audit passes. | `src/main.rs`; `package.json`; `tests/package_lifecycle_cli.rs`; TC-111; TC-112 |
| FND-176 | medium | Closed: the first bounded plan retained complete source bytes and then constructed a second destination snapshot, doubling the logical package ceiling in memory. Inspection now streams SHA-256 identities through an 8 KiB buffer and copies each file with an exact-length bound. | `src/package_host.rs`; FR-017-CON-3 |
| FND-177 | medium | Closed: source-root links, individual/total byte ceilings, entry ceilings, and exact boundary admission were not all independently exercised. Tests now cover module and nested links, FIFO entries, non-portable names, exact and over-limit 8 MiB files, exact and over-limit 16 MiB populations, and exact and over-limit 4,096-entry populations. | `tests/package_lifecycle_cli.rs`; FR-017 Behavior; TC-111 |
| FND-178 | medium | Closed: a top-level file destination must not be registered for rollback until create-new succeeds, or a racing pre-existing file could be deleted. Registration now occurs after successful creation; the rollback test proves that only its explicit directory/file population is removed, and disabling rollback fails that test. | `src/package_host.rs`; TC-111 |
| FND-179 | low | No open Rust finding remains. Production library code has no I/O; host I/O is confined to the binary adapter. There is no unsafe block, panic, lint suppression, ambient environment read, shell invocation, network, clock, async/lock surface, unbounded traversal, or arbitrary path deletion. New requirement tests use canonical bare `ix_trace_rs::trace`. | `src/package_lifecycle.rs`; `src/package_host.rs`; `tests/package_lifecycle_cli.rs`; NFR-005 |

## Gate results

| Gate | Result |
| --- | --- |
| Exact Rust 1.98.1 formatting | pass; inherited vendored ix-trace-rs nightly-option warnings remain visible in direct formatting runs |
| Exact Rust 1.98.1 Clippy and compile | pass; all targets/all features/locked, `-D warnings`, two build jobs |
| Exact Rust 1.98.1 tests | pass; 93 tests, including nine lifecycle integration tests and one rollback unit test |
| Exact Rust 1.98.1 rustdoc | pass; workspace/all features/no dependencies/locked, `-D warnings` |
| `cargo deny --locked check` | pass; advisories, bans, licenses, and sources; existing duplicate transitive versions remain warnings under policy |
| `cargo audit` | pass; 140 locked dependencies scanned against 1,243 advisories |
| Retained MJS correspondence | pass before deletion; retained stage was accepted and cleaned by Rust, then Rust stage was accepted and cleaned by the retained path at the same candidate tree |
| Real package integration | pass; wheel build/install, npm pack/install, exact allowlists, content rights, discovery, workflow equivalence, and cross-package canonical bytes |
| Mutation probes | pass; removing module-root validation, cleanup correspondence, exact total-ceiling admission, or rollback removal fails the owning TC-111 test; omitting hook rendering reproduced the two-document npm JSON failure |
| `make lint` | pass |
| `make test` | pass; content rights, 189 Python tests, and retained module validation |
| `make package-audit` | pass |
| Local Quire validation | pass; inherited duplicate-provider diagnostics only |
| Slice traceability | pass; TC-111 and TC-112 tests use canonical `ix_trace_rs::trace` markers while their combined acceptance rows remain explicitly partial |
| Whole-port integration traceability | expected incomplete; 229/248 and 114/121 test cases while archive integration, remaining host integration, cutover, final removal, and aggregate NFR-005 obligations remain pending |

## Mutation evidence

- Removing explicit validation of the selected module root admits an
  intervening directory symlink and fails the linked-root TC-111 assertion.
- Replacing the cleanup correspondence comparison with a constant false guard
  deletes changed staged content and fails the no-deletion TC-111 assertion.
- Changing the total-byte ceiling from `>` to `>=` rejects the exact 16 MiB
  boundary and fails the exact-resource TC-111 test.
- Disabling rollback leaves both invocation-owned destinations and fails the
  rollback ownership test.
- Direct JSON rendering under `prepack` reproduces the retained package audit's
  `Extra data` JSON failure; explicit hook rendering kills that integration
  mutant.

## Review disposition

**PASS for the reviewed Rust implementation slice.** The two MJS npm lifecycle
paths are replaced by the typed Rust CLI and removed after same-revision
correspondence. TC-111 and TC-112 remain partial until their wider archive,
installed-bundle, integration, and host-configuration populations are complete.
