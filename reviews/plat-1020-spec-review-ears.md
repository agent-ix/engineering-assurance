---
id: SR-16400
title: "EARS review of plan-declared constant-predictor answer families (engineering-assurance PR 199)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/engineering-assurance@7efdaee; spec/functional/FR-021-measurement-plan-decision-rule.md"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-021
    type: references
---

# EARS review of plan-declared constant-predictor answer families (engineering-assurance PR 199)

## Summary

Ticket: PLAT-1020. Examined the new and edited FR-021 Behavior statements and the Error Handling sentence. `quire validate` exits 0; its grammar pass reports warnings on the new statements alongside pre-existing ones on lines 93, 97 and 109 that this PR does not touch.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new `answer_families` statement packs six SHALLs into one bullet and opens with an agentless passive ("SHALL be required exactly when"), and the checker statement carries two SHALLs (quire `quality:agentless-passive` line 118, `ears:non-singular` line 123). Split into one requirement per SHALL with the schema as subject (presence/absence; entry shape), matching AC-16's cases. | spec/functional/FR-021-measurement-plan-decision-rule.md:118-124; spec/functional/FR-021-measurement-plan-decision-rule.md:125-129 |

## Verdict

Wording-only finding; no behaviour is wrong.
