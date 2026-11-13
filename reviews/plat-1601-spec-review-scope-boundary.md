---
id: SR-20261113170527
title: "Scope and boundary review of the advisory scan requirement (PR 234)"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/engineering-assurance; FR-044 scanner inputs, package coverage, and responsibility boundaries"
review_set: all
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-044
    type: references
---

## Summary

Reviewed the producer boundary, its caller-owned offline database, supported fixture ecosystems, and downstream report ownership. The spec does not decide whether Lean manifest components are covered or must be surfaced as unsupported, despite that explicit issue requirement.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Lean component coverage is undecided, with no requirement to report unsupported components instead of allowing a clean result | spec/functional/FR-044-osv-scanner-advisory-scan-producer-request.md:103-113,162-163 | missing-requirement |

## Dispositions

| FND | outcome | reason / after_excerpt |
| --- | --- | --- |
| FND-001 | still-open | The updated fixture list now requires container OS-package coverage, but it still does not decide whether Lean manifest components are scanned or require an explicit unsupported outcome that cannot be read as clean. |
| FND-001 | fixed | `Lean lake manifest components are outside the supported advisory coverage of this producer. When a Lean component is present in a SOUP record, the quoin consumer SHALL surface it as unsupported. The quoin consumer SHALL NOT report a Lean component free of advisories.` |
