---
id: SR-20261115040626
title: "Integrity review of the FR-048 risk management amendment (engineering-assurance PR 300)"
type: SpecReview
analysis: integrity
scope: "agent-ix/engineering-assurance@feb71ac91; spec/functional/FR-048-human-release-decision-workflow.md; engineering_assurance/skills/assurance-onboarding/workflows/release-decision/def.yaml"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-048
    type: references
---

# Integrity review of the FR-048 risk management amendment (engineering-assurance PR 300)

## Summary

Ticket: EA-140 (part 2). This review covers the FR-048 sections the PR changes: Description (the presented list and the new "presented and never judged" paragraph), Out of scope, Inputs, Requirements, Acceptance Criteria AC-18..AC-20, and Dependencies. It also covers the `def.yaml` `release_presentation.required` change.

The amendment holds together on its central point. People accept risk, and the workflow assigns no score, class or acceptability. The Out of scope line now excludes computing, aggregating or deciding risk acceptability. AC-18..AC-20 each map to one requirement statement, and each can fail.

The cited targets were checked on origin/main. `ix://agent-ix/spec-objects-safety/FR-305` (risk summary report) and FR-107 (overall residual-risk evaluation) exist on spec-objects-safety main. FR-109 (risk management review) exists only on spec-objects-safety#51, which is still open. The stated merge order (#51 first) is therefore required for the reference to resolve. `quoin risk-summary --format json` exists on quoin main and prints an object with `rows`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The Inputs bullet still says "The workflow reads these members" and lists only the evidence, SOUP, anomaly and unreadable members. It omits the risk summary's `rows` list and the two documents' frontmatter `type`, which the new Description paragraph and the code read. The list is no longer the complete set of members read. | spec/functional/FR-048-human-release-decision-workflow.md:94-99 |
| FND-002 | low | Dependencies › Upstream adds only `ix://agent-ix/spec-objects-safety/FR-109`. The amendment depends equally on FR-107 (the `overall_residual_risk_evaluation` type it checks) and FR-305 (the risk summary's shape), both cited in the Description. List all three, or none. | spec/functional/FR-048-human-release-decision-workflow.md:200 |

## Verdict

The amendment is consistent with the code and with spec-objects-safety FR-104/FR-107/FR-109. Two low completeness inconsistencies. Merge spec-objects-safety#51 first so that the FR-109 reference resolves.
