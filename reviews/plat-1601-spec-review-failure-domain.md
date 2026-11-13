---
id: SR-20261113170522
title: "Failure-domain review of the advisory scan requirement (PR 234)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/engineering-assurance; FR-044 partial-scan and unsupported-package behavior"
review_set: all
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-044
    type: references
---

## Summary

Reviewed scanner exit and output handling, incomplete scans, unsupported components, and report ingestion. The spec allows a partial lookup to appear as a completed scan without requiring the consumer to preserve an unknown status. It also requires a descriptor path with no `.cdx.json` filename suffix even though successful SBOM classification has only been established for a named SBOM path.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | A partial scan can be completed and read as clean without a required incomplete state | spec/functional/FR-044-osv-scanner-advisory-scan-producer-request.md:52,135-143,174 | missing-requirement |
| FND-002 | high | The required descriptor path has no SBOM filename suffix, so AC-1's required scan is unverified and may fail before scanning | spec/functional/FR-044-osv-scanner-advisory-scan-producer-request.md:49,89-95,162,172 | wrong-requirement |
