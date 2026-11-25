---
id: SR-2026112016223475846355054
title: "Gap analysis — Engineering Assurance repository"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/engineering-assurance; spec/, src/, tests/"
review_set: subset
relationships:
  - { target: "ix://agent-ix/engineering-assurance", type: references }
---

## Summary

Ran the repository-wide computed matrix and checked the changed producer-identity requirements, their test bindings and touched implementation paths. All four new ACs are tagged, but the repository-wide matrix has untagged/ignored-only criteria and stale evidence; test-oracle inspection also found no independent check that a clean build embeds the actual current source commit.

## Verdict

**FAIL** — the repository-wide matrix contains untagged and ignored-only criteria, and a new producer-identity oracle does not independently verify its source revision.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The computed matrix reports 88 untagged criteria repository-wide. | quoin matrix |
| FND-002 | high | The computed matrix reports 18 criteria bound only by ignored tests. | quoin matrix |
| FND-003 | medium | Coverage reports 41 test symbols with trace ids that name no criterion, including four new TC-1200–TC-1203 tags. | quire coverage |
| FND-004 | medium | The evidence store reports 19 stale criterion runs; no run evidence is bound to 907 tagged criteria. | quoin matrix |
| FND-005 | medium | The source-revision assertion compares runtime output with the same compile-time macro used by the implementation, so it passes if a clean build embeds no revision or the wrong revision. | tests/signature_verify_cli.rs:250 |

## Coverage

- Reconciliation: `quoin matrix` (`quoin 0.36.8`, active quire matrix); `quire coverage` (`quire 0.40.0`) for untracked trace ids.
- Plan completion: not assessed
- Criteria: tagged 746 · untagged 88 · tagged-by-ignored-test 18 · method-without-symbol 74 · qualified-record 0 · retired 0.
- Evidence: undischarged 907 · stale 19 · suspect 0 · bound 0.
- Untracked test symbols: 41. Four are the new `TC-1200`, `TC-1201`, `TC-1202`, and `TC-1203` labels; each new FR criterion itself has a correct binder.
- New criterion bindings: `FR-036-AC-16` → `tests/signature_verify_cli.rs::tc_1200_unsigned_and_unverifiable_verdicts_carry_producer_identity`; `FR-041-AC-37` → `tests/approval_verify_cli.rs::tc_1201_result_without_trust_inputs_still_carries_producer_identity`; `FR-043-AC-26` → `tests/approval_trail_cli.rs::tc_1202_empty_trail_carries_producer_identity` and the existing trust/entry test; `FR-047-AC-17` → `tests/release_verify_cli.rs::tc_1203_not_released_result_without_approval_carries_producer_identity` and the existing nested-approval test.
- Changed-path reverse-gap review: the new producer identity fields have owning criteria; the four command paths and their result/test helpers are connected. No TODO/FIXME/XXX or placeholder implementation was found in changed source. The audit did not independently classify every pre-existing public behavior in the full repository.
- Stubs: no hollow implementation or assertion-only test was found in the changed source and CLI targets.
- Semantic review: ran for the four new criteria and the helper assertions. Every new criterion has a trace binding; however, the source-revision assertion is not an independent oracle. Tests assert package version and timestamp shape, and the trail/release paths check propagated timestamps and producer fields.

## Review Scope

- {id: FR-036-AC-16, path: "spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md", role: examined, excerpt: "Every verdict for both commit and tag requests, including `unsigned` and `unverifiable`, carries the running package version, the build-embedded Engineering Assurance source commit or `null` when unavailable, and one UTC `generated_at` value with source `host_clock`; all verdicts produced by one invocation share that value, which is not used as signing or trusted time."}
- {id: FR-041-AC-37, path: "spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md", role: examined, excerpt: "Every `approval-verify` verdict, including one whose trust inputs cannot be read and one whose status is not `approved`, carries the running `ea_version`, the build's `ea_source_revision` (or `null` when absent), and a UTC host-clock `generated_at` sampled once for the invocation with source `host_clock`; when `trust` is present, top-level `ea_version` equals `trust.ea_version`; the nested `signature` omits duplicate producer fields, and no producer field is presented as signing or trusted time."}
- {id: FR-043-AC-26, path: "spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md", role: examined, excerpt: "The trail result and every entry carry `ea_version`, `ea_source_revision`, `generated_at` and `generated_at_source`; all entries share the trail's `generated_at`, including an empty or incomplete trail. When result `trust` is present, top-level `ea_version` equals `trust.ea_version`; `trust` carries `allowed_signers_sha256`, `ea_version` and `git_version`, and `allowed_signers_commit` when the request's `allowed_signers` is a `<rev>:<path>` that resolved."}
- {id: FR-047-AC-17, path: "spec/functional/FR-047-release-record-and-signed-release-tag.md", role: examined, excerpt: "Every `released` or `not_released` verdict, including a verdict without `approval`, carries the executable's `ea_version`, build-embedded `ea_source_revision` (or `null` when unavailable), and one UTC `generated_at` value with source `host_clock`; when `trust` is present top-level `ea_version` equals `trust.ea_version`, and when `approval` is present its producer fields are identical; none of these fields is treated as signing or trusted time."}
- {id: TC-1200, path: "tests/signature_verify_cli.rs", role: examined, excerpt: "#[trace(\"TC-1200\", \"FR-036-AC-16\")]"}
- {id: TC-1201, path: "tests/approval_verify_cli.rs", role: examined, excerpt: "#[trace(\"TC-1201\", \"FR-041-AC-37\")]"}
- {id: TC-1202, path: "tests/approval_trail_cli.rs", role: examined, excerpt: "#[trace(\"TC-1202\", \"FR-043-AC-26\")]"}
- {id: TC-1203, path: "tests/release_verify_cli.rs", role: examined, excerpt: "#[trace(\"TC-1203\", \"FR-047-AC-17\")]"}
- {id: producer source identity oracle, path: "tests/signature_verify_cli.rs::assert_producer", role: examined, excerpt: "The test compares `ea_source_revision` to `option_env!(\"EA_SOURCE_REVISION\")`, the same compile-time value used by the producer, rather than an independently obtained expected source revision."}

## Findings Detail

### FND-001 — high

The engine reports 88 live criteria with `static_status: untagged`. They have no source test carrying their criterion ids. This is repository-wide and not introduced by PR #314; it remains part of the required whole-repository audit.

### FND-002 — high

The engine reports 18 criteria whose only tagged tests are ignored. The normal suite therefore does not run evidence for those criteria. These are repository-wide and not introduced by this PR.

### FND-003 — medium

`quire coverage` reports 41 untracked symbols. Four are newly added test labels `TC-1200` through `TC-1203`, none of which is a declared criterion. The FR trace tags beside them correctly bind every newly added AC. Remove or declare the extra TC labels so coverage data does not preserve stale identifiers.

### FND-004 — medium

The matrix reports 19 stale evidence entries and 907 undischarged entries. The brief did not request execution-evidence closure; this finding records stale evidence present in the repository matrix. Undischarged evidence is reported in coverage, not as an additional finding.

### FND-005 — medium

The CLI assertions compare `ea_source_revision` against `option_env!("EA_SOURCE_REVISION")`, while the implementation serializes that same compile-time value. If build metadata is absent or points at the wrong commit, both sides agree and the test passes. The claimed clean-tree focused check is useful run evidence but is not a committed independent oracle for the criterion.
