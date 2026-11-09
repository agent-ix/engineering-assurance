---
id: SR-16311
title: "gap-analysis of the traceability backing (EA-84)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@ea-84-traceability; spec/tests.md; tests/; src/workflow_host.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-014
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-017
    type: references
---

# Gap analysis of the traceability backing (EA-84)

## Summary

Ticket: EA-84. Plan completion: not assessed. `make integration-traceability`
on the PR head reports "integration evidence passed: traceability complete".
Every row this PR flips from pending to done has at least one test tagged with
its TC and AC: FR-014-AC-2/3, FR-015-AC-1..4, FR-016-AC-3, FR-017-AC-1..4,
FR-018-AC-2/4, NFR-002, NFR-005, FR-015-CON-2/3, FR-016-CON-2/5/6/7,
FR-017-CON-1/2 and FR-018-CON-1. The caveats dropped from TC-175 and TC-176
("Linux live sampling pending" and "Linux artifact observation pending") are now
true, because `tc_175_live_linux_host_observation_is_complete` and
`tc_176_live_output_tree_is_sealed_and_symlinks_refuse` exist. TC-184's dropped
Quoin-replay note describes another repository's work.

Two flipped rows claim more than their tests show.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-014-AC-2 requires that *every* machine-facing command emit exactly one declared-version JSON result with diagnostics only on stderr. TC-098 is now marked done on the strength of one command (`manifest-validate`), while the CLI exposes fourteen subcommands, most of them machine-facing. Either tag the CLI tests that already assert the one-line stdout contract for the other commands, or add a table-driven test over the subcommand list. | spec/tests.md:111; spec/tests.md:325; tests/manifest_host_cli.rs:49 | correct-requirement-no-evidence |
| FND-002 | medium | The FR-017-AC-4, FR-017-CON-2 and TC-112 rows dropped "remaining host-configuration census pending", but no census was added. The TC-112 binders inspect only the Makefile content-rights line, `package.json` scripts, the `package-audit` target and CI triggers. Plugin manifests, `opencode.json` and the cli-agent-evals config are not inspected for semantic logic. The NFR-005 row ("executable-language audit ... passing") inherits the same gap through SR-16310 FND-002. | spec/tests.md:130; spec/tests.md:248; spec/tests.md:340; spec/tests.md:226 | correct-requirement-no-evidence |

## Verdict

Traceability is complete and the gate passes. FND-001 and FND-002 are status
prose that overstates coverage. Back them with the tests named above, or keep an
honest caveat in the row.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-003 | low | The FR-014-AC-2 and TC-098 status cells still name only `tests/manifest_host_cli.rs`, although the new table-driven test in `tests/package_boundary.rs` is now the main backing. That table also omits the `agent-evals` capability, which emits the same machine error envelope, and gives no reason for the omission (`onboard-report` is human-facing and is rightly excluded). | spec/tests.md:111; spec/tests.md:325; tests/package_boundary.rs:58-130 | correct-requirement-no-evidence |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `tc_098_every_machine_facing_command_emits_one_versioned_json_result_on_stdout` drives twelve commands through a refusal and asserts a non-zero exit, non-empty stderr, exactly one newline-terminated JSON line, the error protocol and the capability. Making `scope-footprint` emit a second stdout line turns it red. |
| FND-002 | fixed | The FR-017-AC-4, FR-017-CON-2, TC-112 and NFR-005 rows now name what is audited and state that GitHub workflow YAML is not. |
