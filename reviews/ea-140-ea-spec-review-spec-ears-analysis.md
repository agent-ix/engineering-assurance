---
id: SR-20261115040625
title: "EARS review of the FR-048 risk management statements (engineering-assurance PR 300)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/engineering-assurance@feb71ac91; spec/functional/FR-048-human-release-decision-workflow.md (statements added by the PR)"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-048
    type: references
---

# EARS review of the FR-048 risk management statements (engineering-assurance PR 300)

## Summary

Ticket: EA-140 (part 2). This review covers the three requirement statements the PR adds to FR-048:

1. "The workflow SHALL present a supplied risk summary, overall evaluation and risk management review verbatim, beside their sources." This is ubiquitous, singular and testable.
2. "The workflow SHALL NOT read, derive, require or check the risk decision, the plan-executed or readiness statement of a supplied risk file, and SHALL NOT refuse or permit a `release` choice because of what those files say."
3. "If a supplied risk summary is not a JSON object with a `rows` list, or a supplied evaluation or review is not a document of its type, then the workflow SHALL refuse with `release_input_unreadable` naming the file and leave the run before the gate." This is an unwanted-behaviour EARS statement with a clear trigger and response.

`quire validate` reports no new grammar warning on FR-048. The only `ears:non-singular` / `agentless-passive` warning is on a statement (the "outcome SHALL be stored" line) that the PR did not add; it is present on origin/main as well.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Statement 2 is compound: two `SHALL NOT` clauses with different subjects, reading the files and gating the choice. It also says the workflow does not "read" the risk decision, yet the workflow does read each file in full to present it verbatim. Split it into two statements and say "SHALL NOT interpret" or "SHALL NOT extract", so that the presented text is not counted as reading the decision. | spec/functional/FR-048-human-release-decision-workflow.md:134-135 |

## Verdict

Statements 1 and 3 conform to EARS. Statement 2 should be split and its verb sharpened (low). No blocking EARS defects.
