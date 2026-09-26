---
id: SR-135
title: "Gap analysis — engineering-assurance Python-port acceptance-criterion alignment"
type: SpecReview
analysis: gap-analysis
scope: "spec/, src/workflow_invariants.rs, engineering_assurance/, tests/python_port/, tests/workflow_invariants_parity.rs, spec/tests.md; ACs of FR-001, FR-003, FR-005, FR-007, FR-016, FR-017, FR-018, FR-020, FR-021, FR-023, FR-024, FR-025, NFR-003, NFR-005"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/TM-001"
    type: references
---

## Summary

A repository-driven audit of the ported Python suite (`tests/python_port/`,
branch `ea-19-port-python-tests`) against the acceptance criteria it serves.
Plan completion was not assessed. On arrival, eighteen ported tests carried no
acceptance-criterion trace, two more carried only a constraint id, one carried a
trace to a criterion it does not exercise, and one doc comment held a bare
`FR-005` that Quire could not bind; `tc_117` and `tc_118` both failed. After
remediation in the working tree every first-party Rust test carries a
canonical trace, Quire binds every first-party marker, and no Test Matrix row
is newly unbacked (13 unbacked rows, against 17 on `main`, all pre-existing
pending rows). The verdict stays FAIL because those pre-existing unbacked rows
remain.

## Verdict

**FAIL** — 13 pre-existing matrix references (TC-039, TC-097, TC-108, TC-114
and their FR-016/FR-018/NFR-002 criteria) still have no backing test; none was
introduced or worsened by this change, and every finding scoped to the ported
suite is resolved.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Pre-existing, unchanged: 13 matrix references have no backing tagged test. Each is marked pending in the matrix (no status lie): TC-039 needs a retained live evaluation aggregate, TC-097/TC-114 the executable-path census and rollback population, TC-108 an ix-flow host interface. None is cheap to back honestly from this repository. | TC-039; TC-097; TC-108; TC-114; FR-016-AC-4; FR-018-AC-1; FR-018-AC-3; NFR-002-AC-1..AC-3 |
| FND-002 | high | Resolved: five ported invariant tests (non-adjacent promotion, missing change arrays, unexpected invalid exception, architecture and change review binding) drove the JavaScript provider through the pilot path and pinned per-binding fail-closed rules that no criterion owned. Replaced by TC-201 in `tests/workflow_invariants_parity.rs`, which breaks one binding of a passing projection at a time, asserts the Rust evaluator's code and the retained provider's bytes, derives stage order from the MeasurementPlan schema, and was mutation-checked; owned by the new FR-016-AC-5. | tests/python_port/workflows.rs; FR-016-AC-5; TC-201 |
| FND-003 | medium | Resolved: the three `onboard.js --summary` tests exercised shipped behaviour with no owning criterion. Traced to the new FR-001-AC-9 / TC-202, with a missing all-valid exit-0 case added and the module-version check made Python-truthy and equal to the checkout's manifest version. | tests/python_port/onboarding_script.rs; FR-001-AC-9; TC-202 |
| FND-004 | medium | Resolved: the pilot-inventory tests traced only `FR-007-CON-1`, which the trace grammar does not accept as a criterion. `compatible_pilot_inventory_is_exact` now traces TC-043 to the new FR-007-AC-4; `workflow_inventory_and_versions_are_exact` was deleted as a duplicate whose extra assertion was a workflow-version pin (versions are compared pilot-to-canonical by TC-036). | tests/python_port/workflows.rs; FR-007-AC-4; FR-007-CON-1; TC-043 |
| FND-005 | medium | Resolved: `ground_truth_kind` and `preregistration.bar_digest` schema rules had tests but no criterion. Both tests were strengthened (every kind at and below gate, the gate requirement as the only finding, case and empty refusals; uppercase, length, algorithm, type and extra-member digest refusals) and traced to the new FR-024-AC-11 / TC-203 and FR-024-AC-12 / TC-204. | tests/python_port/module.rs; FR-024-AC-11; FR-024-AC-12 |
| FND-006 | medium | Resolved: `every_matrix_test_reference_has_a_test_case_row` traced to FR-021-AC-13, which it does not exercise, and allowlisted TC-016 and TC-118 as dangling although both rows exist (its pattern missed padded cells). Rewritten without the allowlist, proven to fail on a synthetic dangling reference, and traced to the new NFR-005-AC-5 / TC-205. | tests/python_port/module.rs; NFR-005-AC-5; TC-205 |
| FND-007 | medium | Resolved by deletion: eight `module.rs` tests had no honest criterion and did not earn one. Six restated schema literals (profile optional members, stage/statistical-design literals including a pinned description string, subject identity, component-contract required members, the argument "score" substring); their skeleton fields are pinned behaviourally by TC-195's recorded verdicts and stage order by TC-201. `repository_has_only_governed_review_evidence` pinned exact `docs/` and `plan/` listings (ceremony; review documents are validated by `make validate-docs`). `module_payload_is_visible_to_git_and_rights_checks` is superseded by the package audit's per-member rights check and allowlist from a clean checkout (TC-111, TC-018). `structural_coverage_never_collapses_unknowns_into_success` grepped a document for phrases. | tests/python_port/module.rs |
| FND-008 | medium | Resolved: the `(FR-005)` prose in a doc comment was read by Quire as a tag it could not bind, failing `tc_118`; reworded. | tests/python_port/workflows.rs::measurement_promotion_declares_checker_and_policy_items; NFR-005-AC-3 |
| FND-009 | low | TC-115 is now bound by `only_the_configuration_package_entry_point_remains_python`, which covers only the Python slice of FR-018-AC-4 / NFR-005-AC-4 while `onboard.js` and `invariants.js` remain. The matrix rows now say so; the criterion stays pending. | TC-115; FR-018-AC-4; NFR-005-AC-4 |
| FND-010 | low | `src/integration_evidence_host.rs` pins `REQUIRED_TEST_CASES = 164`, but `spec/tests.md` already declared 194 test cases on this branch before this change (199 now), so `make integration-traceability` refuses on population regardless of this work. The pin guards nothing in its current state; it should be raised or reconsidered by its owner. | src/integration_evidence_host.rs::REQUIRED_TEST_CASES; FR-017-AC-10 |
| FND-011 | low | Two retained traces are loose rather than wrong: `every_schema_and_skeleton_is_valid` (TC-195 / FR-003-AC-8) also checks body-locator headings, which FR-017-AC-7 owns, and `pilot_invariant_surface_delegates_to_canonical` (TC-036 / FR-007-AC-2) checks delegation by a path string rather than equivalence. Left as is. | tests/python_port/module.rs; tests/python_port/workflows.rs |

## Coverage

- Reconciliation: quire coverage (quire 0.33.0, module spec-artifacts-process)
- Plan completion: not assessed
- Rows backed by a tagged test: 395 / 410 (branch before this change: 384 / 399; `main`: 382 / 399)
- Test-case group `spec/tests.md`: 195 / 199 backed
- Unbacked matrix references: 13 (all pre-existing; `main` has 17)
- First-party unmatched or untracked Rust markers: 0 (was 1)
- Untraced behaviors / stubs in scope: 0 remaining; 20 untraced or constraint-only tests dispositioned: 6 traced to new criteria (2 of them strengthened), 5 replaced by TC-201, 9 deleted; 1 mistraced test rewritten and retraced (TC-205); 1 new test added (summary exit 0)
- Semantic review: ran over the ported suite's tests and the criteria they serve (intent↔test↔code judged per test, recorded above)
