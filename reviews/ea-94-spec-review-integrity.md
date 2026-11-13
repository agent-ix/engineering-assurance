---
id: SR-20261113144003
title: "Integrity review of the fifth canonical workflow against the merged workflow-population requirements (engineering-assurance PR 230)"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/engineering-assurance@d67d1c9; spec/tests.md; spec/functional/FR-002-canonical-discovery-bundle.md; spec/functional/FR-007-pilot-compatibility.md; spec/functional/FR-017-rust-evaluation-and-qualification.md; spec/functional/FR-048-human-release-decision-workflow.md; spec/non-functional/NFR-001-cross-agent-parity.md; spec/integration/IT-001-local-source-agent-discovery.md; spec/integration/IT-002-package-workflow-discovery.md"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-048
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-002
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-007
    type: references
---

# Integrity review of the fifth canonical workflow (engineering-assurance PR 230)

## Summary

Ticket: EA-94. The PR's only spec edit is twelve TC rows in spec/tests.md (quire validate exit 0). FR-048 requires a canonical `release-decision` workflow, so the canonical population goes from four to five. The PR changes code and tests to five but leaves the merged population requirements at four.

FR-007 judgement: making `release-decision` canonical-only fits FR-007-CON-1 and FR-007-AC-4, which freeze the pilot at the four names present before promotion. But FR-007-AC-2 ("pilot and canonical discovery yield equivalent workflow names") is now false as written, and TC-036 satisfies it only by deleting `release-decision` from the canonical set before comparing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Merged requirements now contradict FR-048 and the code. FR-002-AC-3 says "exactly the four promoted workflow names". NFR-001 Scope, its metric and NFR-001-AC-2 say "all four promoted workflows" and "4 of 4". FR-017 says "exactly the four declared canonical and pilot workflows". IT-001 and IT-002 say four workflows. tests.md TC-011 and its interface row say "exactly four". FR-007-AC-2 requires equivalent names. TC-011, TC-038 and TC-036 were edited to five or carved out, with none of these amended. Amend them in this PR: five canonical; the pilot equivalence is per pilot name. | spec/functional/FR-002-canonical-discovery-bundle.md:47; spec/non-functional/NFR-001-cross-agent-parity.md:24,37,45; spec/functional/FR-007-pilot-compatibility.md:52; spec/functional/FR-017-rust-evaluation-and-qualification.md:422; spec/integration/IT-001-local-source-agent-discovery.md:38,49; spec/integration/IT-002-package-workflow-discovery.md:46; spec/tests.md:287,809 |
| FND-002 | low | FR-048 is silent on choices the code makes and should state them in Inputs/Outputs: the required `release_id` (REL-n) and `record_path` at `present` and `meaning` on `release`; Release `name` = Baseline name, `title` "Release of <name>" and tag subject "Release <name>"; every supplied anomaly, whatever its status, must be named accepted (faithful to FR-162 "carried"); `baseline_commit` may be any revision (resolved, the string bound); and the input member names read. | spec/functional/FR-048-human-release-decision-workflow.md:62-83 |

## Verdict

Spec inconsistency introduced by the PR: FND-001 is high and blocks merge until the population requirements are amended alongside the code. FND-002 is a low FR-048 amendment.
