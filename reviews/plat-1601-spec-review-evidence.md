---
id: SR-20261113170525
title: "Evidence review of the advisory scan requirement (PR 234)"
type: SpecReview
analysis: evidence
scope: "agent-ix/engineering-assurance; FR-044 acceptance-criteria verification methods"
review_set: all
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-044
    type: references
---

## Summary

Ran `quoin advise` and reviewed the authored methods for all FR-044 criteria. AC-6 and AC-13 are authored as Inspection even though their asserted request/run outcomes can and should be checked by tests; AC-13 explicitly requires an integration-run result.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | AC-6 assigns Inspection to equality checks over the request used by tests | spec/functional/FR-044-osv-scanner-advisory-scan-producer-request.md:167 | correct-requirement-no-evidence |
| FND-002 | medium | AC-13 assigns Inspection to outcomes that include recorded integration-run results | spec/functional/FR-044-osv-scanner-advisory-scan-producer-request.md:174 | correct-requirement-no-evidence |
