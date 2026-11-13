---
id: SR-20261113085804
title: "EARS conformance review of the Baseline/Release requirements (engineering-assurance PR 225)"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-046..FR-050, spec/non-functional/NFR-007 (PR #225 diff, head 55e222ae)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-047
    type: references
---

# SR-20261113085804: EARS conformance of the Baseline/Release requirements (engineering-assurance PR 225)

## Summary

Ticket: EA-92. Examined every Description and Behavior statement and the NFR-007 statement
against EARS grammar. `quire validate --scope . "spec/**/*.md"` reports 10 grammar warnings on the
new files. All of them come from compound statements that join `SHALL ... and SHALL NOT ...` in one
bullet, or from one agentless passive. Where a line wrap puts `SHALL` at the start of a line, quire
also reports `ears:missing-subject` and `ears:unclassifiable`. NFR-007's statement is also compound,
though quire does not flag it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-046 has non-singular statements, quire lines 57 and 59: "... SHALL report that path `absent` and SHALL NOT substitute ..." and "SHALL read the commit only through git object access and SHALL NOT read the working tree". The line-53 statement ("SHALL read ... and SHALL report ...") is compound too. Fix: one bullet per SHALL. | spec/functional/FR-046-baseline-archetype.md:53-60 |
| FND-002 | low | FR-047 has a non-singular statement at line 94 ("SHALL derive the person only from ... and SHALL NOT derive one from ..."). Line 97 ("SHALL require `meaning` ... and SHALL NOT decide what a meaning permits") is reported as unclassifiable with a missing subject. The "SHALL compute the record digest ... and pass ..." statement is compound as well. Fix: split each into separate bullets. | spec/functional/FR-047-release-record-and-signed-release-tag.md:87-97 |
| FND-003 | low | FR-048 line 85 is an agentless passive: "A run id reused ... SHALL be refused". Fix: "If a run id is reused with a different repository, Baseline, baseline commit or decision owner, then the workflow SHALL refuse the run without changing either run." | spec/functional/FR-048-human-release-decision-workflow.md:84-85 |
| FND-004 | low | FR-049 has an unclassifiable statement with a missing subject at line 54 ("... SHALL NOT compute SOUP, evidence or anomaly status and SHALL read them from supplied files only"), and a non-singular statement at line 57 ("SHALL NOT mutate a Quoin record, and SHALL NOT embed a Quoin schema copy"). Fix: split each into separate bullets. | spec/functional/FR-049-release-ownership-split-and-data-contract.md:53-58 |
| FND-005 | low | FR-050 has a non-singular Description statement at line 23 ("The README command table SHALL list `release-verify`, and the maintainer-only note SHALL keep ..."). Fix: move each SHALL into its own Behavior bullet. | spec/functional/FR-050-record-named-persons-release-decision-stance.md:16-26 |
| FND-006 | low | The NFR-007 statement joins four obligations: report only for a mapped signer, make no decision, sign nothing, keep no baseline digest. Fix: split it, or keep the first obligation and leave the rest to FR-047 and FR-048. The last obligation falls away with base-review FND-009. | spec/non-functional/NFR-007-no-release-without-a-person.md:19-22 |

## Verdict

Six low grammar findings, all fixable by splitting compound bullets and naming the agent. Fix
them in the same round as the base-review findings, so that `quire validate` reports no warnings
for FR-046 to FR-050 or NFR-007.
