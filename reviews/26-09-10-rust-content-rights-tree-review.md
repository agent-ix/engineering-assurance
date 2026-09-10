---
id: SR-082
title: "Rust review of content-rights tree adapter and cutover"
type: SpecReview
analysis: code-review
scope: "FR-017-AC-3, FR-017-AC-4, FR-017-CON-2, FR-017-CON-3, FR-018-AC-2, FR-018-AC-3, TC-111, TC-112, TC-113, TC-114; src/content_rights.rs; src/content_rights_host.rs; src/process_host.rs; src/main.rs; src/workflow_host.rs; tests/content_rights_tree_cli.rs; tests/workflow_host_cli.rs; Makefile"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-018"
    type: reviews
---

## Summary

The repository rules and exact `agent-skills/rust-review/SKILL.md` checklist
were applied to the content-rights selected-tree result, capability-confined
filesystem/Git/environment adapter, shared binary-only bounded process
mechanic, CLI rendering, direct Make dispatch, and retained ix-flow behavior.

The adapter requires the exact Git worktree root, obtains the retained tracked
plus non-ignored-untracked NUL-delimited population without a shell, bounds
process time/output, validates path and protected-token populations, reads
regular files through a capability directory with individual and aggregate
byte ceilings, classifies links without following them, and returns one
deterministic typed result without source or token disclosure. The shared
process module contains no producer or assurance semantics and remains private
to the binary crate.

The direct tree gate is now Rust. The retained Python checker is deliberately
not deleted: a deletion probe proved that `scripts/audit_packages.py` imports
its classifier and Python test collection then fails. The repeated `/specify`
cycle records that dependency; final removal belongs with the package/archive
adapter rather than being concealed by a compatibility shim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-180 | high | Closed through a failed governed deletion probe and repeated `/specify`: deleting the Python checker broke `tests/test_packages.py` collection because `scripts/audit_packages.py` imports `text_findings`. The deletion was reverted, the direct tree gate remains Rust, and the retained checker cannot be removed until the archive consumer moves. | `scripts/audit_packages.py:22`; `scripts/check_content_rights.py`; FR-017 Behavior; FR-018 Behavior |
| FND-181 | medium | Closed: extracting the existing child-process implementation into the shared private adapter invalidated TC-107's source assertion that `Command::new` remained inside `workflow_host.rs`; the full suite failed. The gate now verifies one `process_host::run` seam from workflow code and exactly one direct `Command::new(executable)` in the shared adapter while retaining every no-shell/no-gate-override assertion. | `tests/workflow_host_cli.rs:552`; `src/process_host.rs:51`; TC-107 |
| FND-182 | medium | Closed: the initial NUL parser filtered every empty segment, so malformed `first\0\0` Git output silently reduced the selected population. Parsing now removes exactly one required terminal NUL and rejects every interior empty entry; malformed, non-UTF-8, duplicate, exact-limit, over-limit, and overlong populations are tested. | `src/content_rights_host.rs:194`; `src/content_rights_host.rs:365`; TC-111 |
| FND-183 | medium | Closed: process-error mapping, non-repository roots, and inspected-entry overflow were not independently exercised. Typed code tests and checked counters now cover those paths without deriving caller decisions from error prose. | `src/content_rights_host.rs:76`; `src/content_rights_host.rs:252`; `tests/content_rights_tree_cli.rs:230`; TC-111 |
| FND-184 | low | No open Rust finding remains in the reviewed slice. Production code contains no unsafe block, panic, lint suppression, shell invocation, network, async/lock state, unbounded output capture, arbitrary path deletion, or untyped emitted object. Every new Rust test uses a canonical bare `ix_trace_rs::trace` marker. | `src/content_rights.rs`; `src/content_rights_host.rs`; `src/process_host.rs`; `tests/content_rights_tree_cli.rs`; NFR-005 |

## Gate results

| Gate | Result |
| --- | --- |
| QUOIN specify and owner-selected base review | pass after the deletion dependency returned the implementation to `/specify`; SR-081 validated |
| Exact Rust 1.98.1 formatting | pass; inherited vendored ix-trace-rs nightly-option warnings remain visible |
| Exact Rust 1.98.1 Clippy and compile | pass; workspace/all targets/all features/locked, `-D warnings`, two build jobs |
| Exact Rust 1.98.1 tests | pass; 103 tests across library, binary, integration, and doc targets |
| Exact Rust 1.98.1 rustdoc | pass; workspace/all features/no dependencies/locked, `-D warnings` |
| `cargo deny --locked check` | pass; advisories, bans, licenses, and sources; existing duplicate transitive versions remain policy warnings |
| `cargo audit` | pass; 140 locked dependencies scanned against 1,243 advisories |
| Retained same-revision correspondence | pass; both paths accept the complete candidate tree with zero findings, and a temporary real Git repository proves identical accepted/withheld status plus protected-token, URL, and symlink tuples |
| Direct cutover | pass; `make test` invokes the exact Rust 1.98.1 CLI and 189 retained Python tests plus module validation pass |
| Rollback | pass; cutover commit `cf836d9` passed, revert `aef4d6b` restored the Python dispatch with 189 tests passing, and revert-of-revert `4a40b94` restored the Rust dispatch without rewriting history |
| Package audit | pass; the retained archive consumer remains functional and its dependency prevents premature checker deletion |
| Local Quire validation | pass; inherited duplicate-provider diagnostics only |
| Hosted CI | not run; local-only policy preserved |

## Mutation evidence

- Bypassing canonical Git top-level equality admits a nested partial scan and
  fails the owning TC-111 root-identity assertion.
- Changing the total-byte comparison from `>` to `>=` rejects the exact 64 MiB
  boundary and fails the owning TC-111 resource test.
- Omitting `--others` drops the untracked candidate and fails the exact
  inspected population before its findings can disappear silently.
- Restoring the Python Make dispatch fails the TC-112 exact Rust-dispatch gate.
- Reading only the process-output limit rather than limit plus one makes an
  overflow indistinguishable from exact admission and fails the bounded-host
  process test.

## Review disposition

**PASS for the reviewed Rust tree adapter and direct-dispatch cutover.** The
Python checker remains a governed parity dependency of the pending
package/archive adapter; this review does not claim its deletion, complete
TC-111/TC-112 coverage, or aggregate FR-018 removal.
