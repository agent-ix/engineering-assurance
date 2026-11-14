---
id: SR-20261113192952
title: "Gap analysis — engineering-assurance EA-107 change-record builders"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/engineering-assurance; src/change_record/{predicate.rs,bom.rs}; src/change_record_schemas/change-record-v1.schema.json; tests/change_record.rs; FR-072; FR-073; EA-107 acceptance criteria"
review_set: subset
relationships:
  - { target: "ix://agent-ix/engineering-assurance/FR-072", type: references }
  - { target: "ix://agent-ix/engineering-assurance/FR-073", type: references }
---

## Summary

Ran the computed matrix and reverse-gap review for the EA-107 change-record delta, then compared the tagged tests and new APIs with FR-072, FR-073, and EA-107's six acceptance criteria. Three FR criteria remain untagged, and the tagged digest test does not establish the full canonical-digest contract.

## Verdict

**FAIL** — FR-072-AC-5 and FR-073-AC-4/AC-5 are untagged, and the implementation exposes no path that guarantees the BOM stored privately is the BOM digest recorded in the predicate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-072-AC-5 has no tagged test or statement builder carrying the squash commit and merged tree as the statement subject | FR-072-AC-5, src/change_record/ |
| FND-002 | high | FR-073-AC-4 is untagged and the new builders do not connect a BOM to the private form or prove that the predicate digest names that private BOM | FR-073-AC-4, src/change_record/predicate.rs:203-208,311 |
| FND-003 | high | FR-073-AC-5 is untagged and cannot be represented by the required `AgentSession` and `agent_bom: String` inputs | FR-073-AC-5, src/change_record/bom.rs:132-141, src/change_record/predicate.rs:203-206 |
| FND-004 | high | FR-072-AC-7's tagged test checks only that changing a private-form value changes its digest; it does not verify the canonical digests, raw Markdown review digest, or private `change_record` omission required by the criterion | FR-072-AC-7, tests/change_record.rs:215-224 |
| FND-005 | medium | FR-072-AC-2's schema test applies the JSON Schema only to the grammar case; unknown-member and `repo` ticket cases exercise typed serde rejection instead | FR-072-AC-2, tests/change_record.rs:107-128 |

## Coverage

- Reconciliation: `quoin matrix --repo <review-worktree> --json` (quoin 0.30.1; quire 0.45.0, engine 0.60.0); `quire coverage --scope <review-worktree> --json` also ran.
- Relevant criteria: 10 tagged, 3 untagged, 0 tagged-by-ignored-test, and 0 method-without-symbol across FR-072/FR-073. All 10 tagged criteria have no run evidence in Quoin's matrix output.
- The repository-wide matrix output enumerates 719 criteria; the change-scoped findings above exclude unrelated pre-existing gaps. `quire coverage` diagnostics report untracked TC-672..TC-682 on unchanged EA-110 attestation files; these are base-only and not included as EA-107 findings.
- Untraced behaviors/stubs in the changed surface: missing merged-subject statement construction and missing-session failure handling; the BOM/private-form digest relation is not enforced.
- Plan completion: not assessed.
- Semantic review: skipped; no opt-in was requested.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The builder now creates an in-toto statement subject with repository, merged commit and merged tree; the tagged test asserts each value. |
| FND-002 | fixed | The builder stores its generated BOM in `PrivateRecord.agent_bom`, hashes that value into the predicate, and the test checks the digest and absence of the model name from public bytes. |
| FND-003 | fixed | `PredicateInput.session` is optional; the new test verifies absent session emits no BOM, failed status, and `contract_missing`. |
| FND-004 | fixed | The tagged test now recomputes statement, test-result, BOM and private-form canonical digests, checks raw review-byte SHA-256, and confirms the private change record omits `private_form`. |
| FND-005 | fixed | The test now runs the JSON Schema validator against all three malformed values: unknown member, out-of-grammar criterion, and `repo` ticket kind. |

