---
id: SR-16310
title: "code-review of the traceability backing and withdrawals (EA-84)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@ea-84-traceability; tests/evaluation_parity.rs; tests/python_port/module.rs; tests/manifest_host_cli.rs; tests/onboarding_cli.rs; tests/workflow_host_cli.rs; tests/package_boundary.rs; tests/evidence_parity.rs; tests/semantics_parity.rs; src/workflow_host.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/NFR-002
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-018
    type: references
---

# Code review of the traceability backing and withdrawals (EA-84)

## Summary

Ticket: EA-84. PR engineering-assurance#198, diff against main only. The Rust
lane (rust-style) is folded in here.

Re-tagged tests were read against their criteria. `tc_131_cli_emits_one_versioned_machine_result_for_explicit_roots`
asserts one JSON line, a declared protocol and empty stderr (TC-098). The four
TC-099 binders assert refusal before a write: escaping, absolute and existing
targets, unavailable Quire, a bad module root, malformed, oversized and
unsupported-protocol requests (onboarding); an unavailable ix-flow with an
unchanged state tree, malformed and oversized requests (workflow host); timeout,
overflow and malformed host responses (process adapter). `tc_096` backs
StR-003-VC-1 and the four TC-100 tests back StR-003-VC-3. No tag inflation found
in the re-tags.

The three new NFR-002 tests each went red under a temporary mutation of the
production check they exercise, reverted afterwards: removing the
unsupported-addition push failed the AC-1 test; returning no
`UnexpectedTerminalEvent` failed the AC-2 test, and so did dropping the blank-owner
check; counting a failing unqualified-host envelope as a failure failed only the
AC-3 test, no other test in the binary. The touched test binaries pass, and
`make integration-traceability` passes.

The new executable-language audit `only_approved_foreign_language_paths_remain`
fails on a stray `.sh` file. It also fails on gitignored files, and it misses
files under `spec/` and `reviews/`.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The audit walks the working-tree filesystem, not the Git-selected population. It skips only `.git`, `target`, `node_modules`, `.worktrees` and `*-target`, so gitignored `.venv/`, `build/`, `dist/`, `vendor/`, `target-*/` and `.agent-evals/` content makes it fail on a normal dev checkout (shown with `.venv/lib/site.py`). It also skips `spec/` and `reviews/` outright, so a script placed there passes (shown with `spec/tools/hidden.py`). Use the `git ls-files` population that the other FR-018 audits use. | tests/python_port/module.rs:279-306 | implementation-bug-despite-evidence |
| FND-002 | medium | The host-configuration part of the audit is one substring check, `config.contains("agent-evals-provider")`, on one file. Arbitrary assertion logic in that file still passes. Plugin manifests, `opencode.json` and workflow YAML are not inspected, and extensionless shebang scripts are not detected. Yet the test is tagged to StR-003-VC-4 and NFR-005-AC-4, which require a host-configuration audit. | tests/python_port/module.rs:352-360 | correct-requirement-no-evidence |
| FND-003 | low | The "approved but absent" reverse loop and the pilot one-line re-export check duplicate TC-036 (`pilot_invariant_surface_delegates_to_canonical`) and TC-106 parity, which already fail if either provider is removed or changed. The reverse loop turns the allowlist into a file inventory. Delete both. | tests/python_port/module.rs:344-360 | implementation-bug-despite-evidence |
| FND-004 | low | `path.is_dir()` follows symlinks, so a symlinked directory is walked outside the repository or recursively. Check `symlink_metadata` (or walk the Git population, per FND-001). | tests/python_port/module.rs:294 | implementation-bug-despite-evidence |
| FND-005 | low | The NFR-002-AC-1 and AC-2 tests mostly re-assert behaviour that `every_incomplete_semantic_class_withholds_aggregation` (TC-110) and `tc_049` already pin. The per-cell sweep and blank-owner case are the only new coverage. This is acceptable, but it is duplicate coverage. | tests/evaluation_parity.rs:503-558 | implementation-bug-despite-evidence |

## Verdict

Not mergeable yet. FND-001 makes a tagged gate test depend on the checkout's
ignored files and blind to two directories. FND-002 overstates what the audit
proves. Both are small fixes inside this PR. All re-tags are sound and the
NFR-002 tests can fail.
