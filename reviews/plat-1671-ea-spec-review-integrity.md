---
id: SR-20261113055255
title: "Integrity review of the FR-039 licence fixture and FR-039-AC-8 (engineering-assurance PR 223)"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-039-syft-sbom-evidence-ingestion.md (PR #223 diff, head 9ca41138)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-039
    type: references
---

# SR-20261113055255: Integrity of the FR-039 licence fixture and FR-039-AC-8 (engineering-assurance PR 223)

## Summary

Ticket: PLAT-1671. Examined FR-039-AC-8, its consistency with FR-039-AC-1 to AC-7, the new
`references` edge to quoin FR-150, and the Dependencies section. The AC id follows the sequence with no
gap or duplicate. The FR-150 edge resolves only once quoin PR 847 lands; the PR body states the two
merge together, which is the correct pairing. NFR-006 and FR-044 stay consistent.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-039-AC-7 still says "Test code that counts the array in AC-1 to AC-4 is the only reader", but the AC-8 test reads the licence fixture's components, purls and `licenses`; an inspection applying AC-7 as written fails on the test AC-8 requires. Fix: "Test code that checks the artifact in AC-1 to AC-4 and AC-8 is the only reader". | spec/functional/FR-039-syft-sbom-evidence-ingestion.md:74-75 |
| FND-002 | low | The new frontmatter edge to quoin FR-150 is not reflected in Dependencies: Downstream names only "any assurance case", though FR-150-AC-12 consumes the licence fixture's captured output. Fix: add `ix://agent-ix/quoin/FR-150` to Downstream. | spec/functional/FR-039-syft-sbom-evidence-ingestion.md:10-11; spec/functional/FR-039-syft-sbom-evidence-ingestion.md:80 |

## Verdict

Not mergeable until FND-001 is fixed: AC-7 and AC-8 contradict as written (a one-line fix). FND-002 is
for the same round.
