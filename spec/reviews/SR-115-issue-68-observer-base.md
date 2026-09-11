---
id: SR-115
title: "Base review of the compatibility-observation absence requirement"
type: SpecReview
analysis: base
scope: "FR-012, FR-015, TM-001 TC-083, TC-103, TC-130"
review_set: base
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-012
    type: reviews
  - target: ix://agent-ix/engineering-assurance/FR-015
    type: reviews
---
# SR-115: Base review of the compatibility-observation absence requirement

## Summary

This review covers the requirement changes for agent-ix/engineering-assurance#68:
FR-012 now rules that a recorded artifact absent from the observed tree is an
absence that withholds the gate rather than a silently skipped path, and FR-015
now rules that each distinguishable incomplete-retention failure of the accepted
corpus carries its own typed reason. It does not cover the corpus walk bounds
named in the issue, which do not exist in this tree.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
|---|---|---|---|---|
| FND-208 | high | **Closed:** FR-012's Error Conditions instructed the observing program to skip a recorded artifact this tree does not contain. That made an absent artifact indistinguishable from a matching one, so the shipping observer reported a satisfied gate having hashed none of the eleven recorded artifacts. The requirement now rules absence a distinct reported condition that withholds the gate, consistent with the matrix's own `absent_is_not_pass` rule for components. | FR-012 Error Conditions; FR-012-AC-5; FR-012-AC-10; TC-083; TC-130 | wrong-requirement |
| FND-209 | medium | **Closed:** FR-012-AC-5 was satisfied by a unit helper, so the criterion could pass while the production path had no equivalent guard. The criterion now names the observing program explicitly. | FR-012-AC-5; TC-083 | correct-requirement-no-evidence |
| FND-210 | medium | **Closed:** FR-015 required a typed refusal reason per semantic failure but left six distinguishable incomplete-retention failures sharing one code, separated only by prose. The requirement now enumerates the six and requires a distinct typed reason for each. | FR-015 Error Conditions; FR-015-AC-3; TC-103 | wrong-requirement |
| FND-211 | low | **Open:** FR-012-CON-3 carries validation type `Inspection` and no TC row in TM-001. Pre-existing; out of scope for #68. | FR-012-CON-3; TM-001 | correct-requirement-no-evidence |

## Checklist Result

- ID formats: every touched identifier uses the three-digit `FR-NNN-AC-N`,
  `FR-NNN-CON-N` and `TC-NNN` forms. No short forms and no duplicates.
- Gaps and duplicates: no new FR, NFR, StR, US, IT or TC identifier was
  allocated; sibling branches are allocating TC identifiers concurrently, so the
  changed rows reuse TC-083, TC-103 and TC-130.
- Validation link integrity: FR-012-AC-5 resolves to TC-083, FR-012-AC-10 to
  TC-130, and FR-015-AC-3 to TC-103, each in both directions.
- Measurability: the new text is countable rather than adjectival — the observer
  publishes the number of artifacts it hashed, absence and drift are separate
  reported conditions, and six retention failures carry six distinct codes.
- Error modes: the changed criteria are themselves unhappy paths; the happy path
  they qualify was already covered.
- Coverage: every acceptance criterion in the two changed requirements maps to at
  least one test case. FR-012-CON-3 remains the one unmapped constraint and is
  recorded above as pre-existing.
