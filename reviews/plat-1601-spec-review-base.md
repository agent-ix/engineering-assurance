---
id: SR-20261113170521
title: "Base requirements review of the OSV advisory scan amendment (PR 234)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance; spec/functional/FR-044-osv-scanner-advisory-scan-producer-request.md; spec/non-functional/NFR-006-single-sbom-tool-single-execution-path.md; spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-044
    type: references
---

## Summary

The new scanner request is bounded to the retained SBOM and the existing producer execution path. The base review found a P0 version-pinning violation in the research and fixture requirements; this review fails until those pins are removed.

## Verdict

**FAIL** — the specification pins an exact scanner release and vulnerable fixture package versions, contrary to the no-pin rule.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Research and fixture instructions pin an exact scanner release and package versions | spec/functional/FR-044-osv-scanner-advisory-scan-producer-request.md:41-43,105-110 | wrong-requirement |

## Dispositions

| FND | outcome | reason / after_excerpt |
| --- | --- | --- |
| FND-001 | fixed | `The documented behavior below was reviewed from the OSV-Scanner project documentation and source.` The fixture requirement now names ecosystems and fixture contracts without pinning vulnerable package versions; output records observed report fields without pinning a scanner release. |
