---
id: SR-088
title: "Rust review of ix-flow matrix acceptance and v0.2.1 release"
type: SpecReview
analysis: code-review
scope: "FR-012-AC-1, FR-012-AC-4, FR-016-AC-3, FR-017-AC-3, TC-079, TC-082, TC-107, TC-111; compatibility acceptance, release metadata, and wheel-build confinement"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-012"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-016"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

## Summary

The repository rules and exact `agent-skills/rust-review/SKILL.md` checklist
were applied to the human-acceptance transcription, the `v0.2.1` source-release
metadata, the embedded Rust compatibility result, and the package-audit change
needed to exercise the release from a clean worktree.

Peter Krenesky explicitly accepted the complete matrix that pins public
ix-flow 0.2.3 on 2026-09-10. Before transcription, all eight live TC-107 cases
passed against the installed 0.2.3 executable. The matrix, documentation, Rust
classifier fixtures, module/package metadata, and rollback description now
agree on the accepted `v0.2.1` release.

The first clean-root package audit exposed a pre-existing confinement defect:
the Python wheel frontend wrote build products into the selected repository.
Wheel construction now runs from a bounded byte snapshot staged beneath the
invocation-owned temporary directory. The original selected root remains
read-only, while the built wheel is still independently decoded, checked
against the repository-owned allowlist, installed, and compared with the npm
artifact's canonical bytes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-205 | high | Closed: wheel construction ran with the selected repository as its working directory. A fresh worktree created `build/` and `engineering_assurance.egg-info/`, then failed its own root-population gate; a worktree already carrying those ignored directories could conceal the write. The builder now receives a bounded snapshot under the audit temporary directory and the selected root is checked around the child invocation. | `src/package_audit_host.rs`; FR-017-AC-3; TC-111 |
| FND-206 | medium | Closed: the end-to-end package audit asserted cleanup only for npm staging destinations, not the Python build-output names that escaped. The live CLI test now also requires `build/` and `engineering_assurance.egg-info/` to be absent after a successful audit. | `tests/package_audit_cli.rs`; FR-017-AC-3; TC-111 |
| FND-207 | low | No open Rust finding remains in this slice. The change adds no unsafe block, panic, lint suppression, shell invocation, unbounded input, public API, new dependency, hosted CI, or automatic decision. Existing and changed Rust requirement tests retain bare `ix_trace_rs::trace` markers. | reviewed diff; NFR-005 |

## Gate results

| Gate | Result |
| --- | --- |
| Human acceptance | pass; Peter Krenesky accepted the complete ix-flow 0.2.3 matrix on 2026-09-10 and directed transcription |
| Live ix-flow candidate | pass; installed `ix-flow --version` reports 0.2.3 |
| TC-107 | pass; all eight live lifecycle cases against ix-flow 0.2.3 |
| Exact Rust 1.98.1 formatting | pass |
| Exact Rust 1.98.1 Clippy and compile | pass; workspace/all targets/all features/locked, `-D warnings`, two build jobs |
| Exact Rust 1.98.1 tests | pass; 117 tests across library, binary, and integration targets |
| Exact Rust 1.98.1 rustdoc | pass; workspace/all features/no dependencies/locked, `-D warnings` |
| `cargo deny --locked check` | pass; advisories, bans, licenses, and sources; existing duplicate transitive versions remain policy warnings |
| `cargo audit` | pass; 151 locked dependencies scanned against 1,243 advisories |
| `make lint` | pass |
| `make test` | pass; content-rights tree accepted 278 entries, 168 retained Python tests passed, 2 dependency-based cases skipped, and module validation passed |
| `make package-audit` | pass from a root with no pre-existing Python build outputs; wheel 70 files, npm 60 files, and six canonical installed files agreed; no staging or build outputs remained |
| Local Quire validation | pass; inherited duplicate-provider diagnostics only |
| Whole-port integration traceability | expected incomplete; 231/248 relationships and 115/121 test cases while later FR-016, FR-017, FR-018, and NFR-005 slices remain open |
| Hosted CI | not run; local-only policy preserved |

## Mutation evidence

- With the prior repository-root wheel invocation, the clean worktree creates
  `build/` and `engineering_assurance.egg-info/` and returns
  `package_audit_root_output_changed`.
- With the temporary source snapshot, the same real wheel and npm build passes
  while both escaped Python output roots remain absent.
- The existing child-created-output fixture still proves that a child writing
  into a selected root is refused.

## Review disposition

**PASS for the ix-flow 0.2.3 acceptance transcription, the `v0.2.1` release
metadata, the Rust compatibility result, and the package-audit confinement
remediation.** The release makes the already-reviewed classifier and lifecycle
host consumable; it does not complete FR-016-AC-4, FR-017, FR-018, or the whole
Rust-port traceability gate.
