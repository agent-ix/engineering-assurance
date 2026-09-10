---
id: SR-066
title: "Rust review of bounded onboarding"
type: SpecReview
analysis: code-review
scope: "FR-016-AC-1 and FR-016-CON-4; src/onboarding.rs; onboarding CLI host adapter; TC-105"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-016"
    type: reviews
---

## Summary

The repository rules and `agent-skills/rust-review/SKILL.md` were applied to
the additive Rust port of inventory-first onboarding. The final surface keeps
request parsing, rendering, and recommendation in the I/O-free library while a
binary-only adapter owns selected-root traversal, Quire invocation, staging,
synchronization, and atomic no-replace publication. Eight findings were
corrected during specification and implementation review; no open Rust finding
remains in this slice.

The Rust CLI differentially matches the retained Python inventory and decision
behavior on the supported domain. One deliberate fail-closed correction is
retained as part of the reviewed contract: duplicate-key and merge-key YAML
cannot supply artifact identity even though PyYAML previously selected a value.
The slice does not implement ix-flow lifecycle, redirect canonical or pilot
workflows, or remove the retained Python onboarding path.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-129 | high | Closed: the initial installed-module guard used conjunction between missing `manifest.yaml` and missing `skeletons`, so a directory containing only one was admitted. The predicate now requires both, and restoring the defective conjunction makes TC-105 fail with success where exit 2 is required. | `src/onboarding_host.rs:90`; `tests/onboarding_cli.rs:373`; FR-016-AC-1 |
| FND-130 | high | Closed through `/specify` and base `/spec-review`: the maintained Rust YAML parser rejects duplicate keys while PyYAML selected the last key, and merge expansion could synthesize identity. Artifact identity now requires one explicit top-level string `type`; duplicate and merge keys remain malformed and cannot enter selection. Focused old/new observations plus the complete CLI inventory disposition are asserted. | `src/onboarding.rs:466`; `tests/onboarding_core_parity.rs:293`; `tests/onboarding_cli.rs:407`; FR-016-AC-1 |
| FND-131 | medium | Closed: recursive traversal made repository-controlled directory depth a production stack-growth input. Inventory now uses an explicit pending-directory collection and retains final lexical sorting. | `src/onboarding_host.rs:100`; FR-016-AC-1 |
| FND-132 | medium | Closed: the first renderer replaced a heading for a whitespace-only title, while the retained renderer did not. The typed renderer now requires non-whitespace title content; every installed skeleton is differentially exercised with populated and whitespace-only titles. | `src/onboarding.rs:451`; `tests/onboarding_core_parity.rs:225`; FR-016-AC-1 |
| FND-133 | medium | Closed through `/specify` and base `/spec-review`: the versioned error contract omitted the dedicated unsupported-protocol code already exposed by the Rust type. `unsupported_onboarding_protocol` is now normative and exercised at the CLI boundary. | `spec/functional/FR-016-rust-onboarding-and-workflow.md:98`; `src/onboarding.rs:279`; `tests/onboarding_cli.rs:394` |
| FND-134 | medium | Closed: TC-105 originally exercised pure outputs but did not gate the required I/O-free seam. The source audit now proves the binary host is absent from the library and rejects filesystem, environment, process, network, and system-clock capability spellings in the reusable module. | `tests/onboarding_core_parity.rs:353`; FR-016-CON-4 |
| FND-135 | medium | Closed: adding capability-confined publication introduced a transitive SPDX license expression absent from dependency policy, causing `cargo deny` to fail. The policy now explicitly admits `Apache-2.0 WITH LLVM-exception`; advisories, bans, licenses, and sources all pass. | `deny.toml:9`; `Cargo.lock`; NFR-005-AC-1 |
| FND-136 | low | Closed: exact-toolchain Clippy rejected a manual destructuring match and needless by-value error helpers. The adapter now uses `let ... else` and borrowed error formatting with no lint suppression. | `src/onboarding_host.rs:244`; `src/onboarding_host.rs:422` |
| FND-137 | low | No open Rust finding remains. Production code contains no unsafe block, panic, unchecked cast, lint suppression, async or lock surface. Requests/results are closed typed structures; the library has no host capability; the CLI bounds stdin; traversal does not follow symlinks or recurse; and publication validates a synchronized same-directory staged file before a capability-confined atomic no-replace link. | `src/onboarding.rs`; `src/onboarding_host.rs`; `src/main.rs:65`; FR-016-CON-4 |

## Gate results

| Gate | Result |
| --- | --- |
| Exact Rust 1.98.1 formatting | pass — `cargo +1.98.1 fmt -- --check` |
| Exact Rust 1.98.1 Clippy | pass — workspace, all targets, all features, locked, `-D warnings`, two build jobs |
| Exact Rust 1.98.1 check | pass — workspace, all targets, all features, locked, two build jobs |
| Exact Rust 1.98.1 tests | pass — 43 tests, including eleven TC-105 Rust tests |
| Exact Rust 1.98.1 rustdoc | pass — workspace, all features, no dependencies, `-D warnings` |
| `cargo deny --locked check` | pass — advisories, bans, licenses, and sources; duplicate transitive dependency versions remain warnings under existing policy |
| `cargo audit` | pass — 78 locked crate dependencies scanned against 1,243 advisories |
| Retained-Python differential | pass — recommendation states, complete sorted inventory, all five installed skeletons, populated/whitespace title behavior, and supported frontmatter classification |
| Fail-closed probes | pass — malformed/extended/oversized requests, unknown protocol/type, malformed/duplicate/merge-key frontmatter, incomplete module root, unavailable Quire, absolute/parent/symlink targets, existing destination, and invalid staged artifact |
| Mutation probes | pass — weakened module-root predicate and bypassed Quire publication validation each fail TC-105 |
| `make lint` | pass |
| `make test` | pass — 189 tests |
| `make package-audit` | pass |
| Local Quire validation | pass — inherited duplicate-provider diagnostics only |
| Slice traceability | pass — TC-105, FR-016-AC-1, and FR-016-CON-4 are bound by eleven canonical `ix_trace_rs::trace` markers and are not reported unbacked |
| Whole-port integration traceability | expected incomplete — 211/242 while later FR-016 lifecycle/cutover, FR-017 qualification, FR-018 migration/removal, and NFR-005 final-state obligations remain pending; no baseline or status gate was weakened |

## Review disposition

**PASS for FR-016-AC-1 and FR-016-CON-4.** This is an additive onboarding
slice. FR-016-AC-3 and FR-016-AC-4 remain pending; no retained implementation
or invocation is removed by this change.
