---
id: SR-20261113055254
title: "EARS conformance review of the FR-039 licence-fixture statements (engineering-assurance PR 223)"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-039-syft-sbom-evidence-ingestion.md (PR #223 diff, head 9ca41138)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-039
    type: references
---

# SR-20261113055254: EARS conformance of the FR-039 licence-fixture statements (engineering-assurance PR 223)

## Summary

Ticket: PLAT-1671. Examined the two Behavior statements the PR adds to FR-039 and FR-039-AC-8 against
them. Context: the existing "Each fixture SHALL yield ..." statement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new statement says components carry `licenses` entries "only for the lock entries that declare a licence", which bars licences on the unlicensed entry but never requires a declaring entry to carry its licence; AC-8 requires that positive direction, so the AC exceeds its statement. Fix: "... whose component for each lock entry that declares a licence carries that licence in `licenses`, and whose component for the entry that declares none carries no `licenses` entry". | spec/functional/FR-039-syft-sbom-evidence-ingestion.md:57 |
| FND-002 | low | "The test SHALL fail if every component ... lacks `licenses`" makes the test the subject and puts the condition after SHALL; it is test design, not system behaviour, and is implied by the corrected FND-001 statement. Fix: drop it, or recast as "If no component of the licence fixture's artifact carries `licenses`, then the licence-fixture test SHALL fail". | spec/functional/FR-039-syft-sbom-evidence-ingestion.md:58 |
| FND-003 | low | The new statement is compound (count and licences) and its count clause restates the existing "Each fixture SHALL yield ... exactly the declared package count" statement, which already covers the fifth fixture. Fix: keep only the licence clause. | spec/functional/FR-039-syft-sbom-evidence-ingestion.md:57 |

## Verdict

Not mergeable until FND-001 is fixed: the requirement statement must carry the positive licence
obligation AC-8 tests. FND-002 and FND-003 are wording for the same round.
