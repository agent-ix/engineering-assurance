---
id: SR-16412
title: "EARS conformance review of the Syft SBOM producer specification (PLAT-1428)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/engineering-assurance; requirement statements of spec/functional/FR-038-syft-sbom-producer-request.md, spec/functional/FR-039-syft-sbom-evidence-ingestion.md, spec/non-functional/NFR-006-single-sbom-tool-single-execution-path.md (PR #212 diff vs origin/main)"
review_set: subset
---

## Summary

Ticket: PLAT-1428. Checked each Description, Behavior and Statement sentence
for EARS form. The unwanted-behaviour bullets in FR-038 and FR-039 ("If ...,
then ... SHALL ...") and the ubiquitous FR-038 and FR-039 descriptions conform.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-038 Behavior "a request whose `arguments` carry no `-o cyclonedx-json=` pair or carry a second `-o` pair is not a Syft producer request" is a definition, not a requirement; FR-038-AC-5 tests a refusal no statement requires. Restate as "If ..., then the builder SHALL refuse the request". | spec/functional/FR-038-syft-sbom-producer-request.md:59-61 |
| FND-002 | low | FR-038 Behavior "Before launch the executor removes any file at the fixed output path" has no SHALL and restates FR-019; mark it as context or cite FR-019 without restating. | spec/functional/FR-038-syft-sbom-producer-request.md:70 |
| FND-003 | low | NFR-006 Statement is compound: three SHALL clauses (only generator, only launch path, only reader). Each already has its own metric row; split or keep one SHALL with an enumerated list. | spec/non-functional/NFR-006-single-sbom-tool-single-execution-path.md:14-17 |

## Verdict

Wording-level only; no behaviour is wrong.
