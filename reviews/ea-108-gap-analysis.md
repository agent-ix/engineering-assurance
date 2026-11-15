---
id: SR-20261115040602
title: "Gap analysis — EA-108 receipt workflow structural test"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/engineering-assurance#280; FR-070-AC-3; changed receipt-workflow test evidence"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-070"
    type: reviews
---

## Summary

Checked the computed FR-070 matrix row and the PR's new workflow test tags against their actual assertions. The matrix's FR-070-AC-3 row is tagged by an older approval-gate test that does not verify action-reference structure; the new receipt tests are not represented by a matching Test Case row, while TC-559 still describes SHA-pin behavior that Peter has removed.

## Verdict

**CONDITIONAL** — the matrix record and Test Case text do not faithfully describe the changed criterion or the added evidence.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The computed matrix reports FR-070-AC-3 as tagged through `tests/approval_gate_cli.rs::workflow_is_explicitly_opt_in_and_has_no_signing_identity`, but that test checks the review-gate's opt-in permissions and triggers, not action-reference structure or checkout credentials. The new receipt tests tag the AC directly but lack a matching Test Case ID; `spec/tests.md` TC-559 still asserts 40-character SHA pins. Correct the TC mapping and stale wording so the matrix identifies the new assertion as evidence. | spec/tests.md:340,919; tests/approval_gate_cli.rs:223-229; tests/receipt_workflow_contract.rs:52-58,105-110 |

## Coverage

- `quire matrix --scope . --format tsv` — FR-070-AC-3 is `tagged` only through the unrelated TC-925 binder above; it does not list the new receipt-workflow test as its binder. The matrix also has unrelated repository-wide untagged criteria, not attributed to this PR.
- Read the new test bodies and the criterion-to-Test-Case rows directly; no hand-written matrix was used.
- Plan completion: not assessed.
