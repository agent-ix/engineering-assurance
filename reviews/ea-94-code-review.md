---
id: SR-20261113144001
title: "code-review and rust-review of the human release-decision workflow (engineering-assurance PR 230)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@d67d1c9; src/release_decision_host.rs; src/workflow_host.rs; src/workflow.rs; src/workflow_invariants.rs; src/release_host.rs; src/package_install.rs; src/main.rs; engineering_assurance/skills/assurance-onboarding/scripts/invariants.js; engineering_assurance/skills/assurance-onboarding/workflows/release-decision/def.yaml; tests/release_decision_cli.rs; tests/discovery.rs; tests/python_port/workflows.rs; tests/workflow_host_cli.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-048
    type: references
---

# Code review and Rust review of the human release-decision workflow (engineering-assurance PR 230)

## Summary

Ticket: EA-94. Head d67d1c9, diff `origin/main...HEAD` (17 files, +1758/-25). Rust-review lane folded in (Agent IX rust-style loaded).

Measured: `release_decision_cli` 12/12, `workflow_host_cli` 10/10, `workflow_invariants_parity` 13/13, `discovery` 5/5 and `python_port workflows` pass; `cargo clippy --all-features --all-targets -D warnings` and `cargo fmt --check` are clean; `make package-audit` is `accepted` (wheel 61, npm 60, installed canonical 7). The bin unit test `package_install::tests::installed_discovery_and_workflow_failures_are_typed` (TC-111) FAILS on this head (left 6, right 5); it passes on origin/main. Mutation testing: removing the unaccepted-anomaly refusal fails TC-453; setting `decision_ready->released` to `auto` in def.yaml fails TC-450 and TC-455. Source-audit scratch run (tc_117/tc_127 patched to list every hit, outside the branch): hits are only the pre-existing files (document_converter_producer_cli.rs tc_376, signature_verify_cli.rs tc_276/tc_277, signature_verify_gitsign.rs tc_297, approval_trail_host.rs); none of this PR's files are flagged.

What holds: both terminal transitions are `hitl`; the request type has no override member and `deny_unknown_fields`; refusals (meaning, snapshot path, anomalies) all run before `run.decide`, so no gate is acknowledged on refusal; no raw `Command`, no hashing, no tag/commit/push; git reads reuse `resolve_baseline` and the hardened `run_git` with `--end-of-options`; JS `terminalKeys` and Rust `terminal_transitions` agree; the TC-459 round trip yields `released` from `release-verify`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR turns TC-111 red: `installed_discovery_and_workflow_failures_are_typed` asserts `canonical_file_count() == 5`, and the fifth workflow makes it 6. The test passes on origin/main and fails on this head. | src/package_install.rs:484 |
| FND-002 | medium | Rust/JS invariant parity is broken for the new workflow. `workflow-invariants` refuses a real release-decision projection with `workflow_invariant_request_invalid` (`unknown field release_request`) because `Items` is `deny_unknown_fields` and lacks `release_request` and `release_presentation`, while invariants.js evaluates the same projection. No parity case covers release-decision. | src/workflow_invariants.rs:441-460; tests/workflow_invariants_parity.rs:142 |
| FND-003 | medium | A decision can change after the gate. A repeated `decide release` on a `released` run recomputes the Release document and the tag message from the new request. Reproduced: a second call with a different `meaning` and a reordered `accepted_anomalies` returns the same decision event but a different document and a different `Signed-off-meaning:`. The run keeps neither the meaning nor the accepted ids. | src/release_decision_host.rs:517-523; src/release_decision_host.rs:529-598 |
| FND-004 | medium | The generic `workflow-host` accepts `release` for `release-decision`, so a run reaches `released` with no presentation, Baseline read, anomaly acceptance or snapshot-path check. Reproduced with start_or_resume, two raw ix-flow `advance` calls and `decide release`. def.yaml has no invariant requiring a `release_presentation` before `decision_ready`. | src/workflow.rs:102-112; engineering_assurance/skills/assurance-onboarding/workflows/release-decision/def.yaml:16-19 |
| FND-005 | medium | The 4 MiB per-file bound does not bound the JSON-escaped presentation. Reproduced: a 4 MiB snapshot of U+0001 bytes is added by ix-flow, and then the reply exceeds the 16 MiB host output cap (`ix_flow_outcome_indeterminate`). Every later present, decide or reject fails with `ix_flow_response_invalid`, so the run is wedged for good. | src/release_decision_host.rs:40; src/release_decision_host.rs:336 |
| FND-006 | low | Supplied input files are opened with `File::open` without a regular-file check. A FIFO path blocks the command indefinitely (release-verify requires a regular file for allowed_signers). | src/release_decision_host.rs:439-441 |
| FND-007 | low | `WorkflowKind::terminal_phase` maps invalid pairs to plausible outcomes, for example `(AssuranceIntake, Release)` to `accepted` and `(ReleaseDecision, Accept)` to `released`, instead of making them unrepresentable or fallible. The `takes` guard only runs in `parse_request_bytes`. The doc comment still reads "One of the four canonical Engineering Assurance workflows". | src/workflow.rs:61; src/workflow.rs:116-135 |
| FND-008 | low | `decide` on an unknown run id creates and binds a new run through `bind_run` before refusing with `release_presentation_missing`, so a refusal changes state. | src/release_decision_host.rs:184; src/release_decision_host.rs:507-511 |
| FND-009 | low | TC-455's source-string scan (`"tag"`, `Command::new` and similar in one file) passes on a renamed helper, and it does not cover the release_host helpers this file calls. The behavioural ref/HEAD/tag/status checks already back AC-6, so the scan adds no evidence. | tests/release_decision_cli.rs:483-496 |

## Verdict

Not mergeable. The core gate is correct: no automatic decision path in the command, refusals before acknowledgement, no git write. But FND-001 leaves a red unit test, and FND-002 to FND-005 are real defects: the Rust/JS parity break, post-gate mutation of the emitted release, the generic-host bypass of every FR-048 refusal, and a permanent wedge from an in-bounds input. Each needs a fix and a test in this PR. FND-006 to FND-009 are low.
