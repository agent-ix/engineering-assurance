---
id: SR-135
title: "Base specification review of EA-117 sink and signature criteria"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance; spec/functional/FR-055-per-repo-sink-configuration-and-visibility-default.md; spec/functional/FR-056-public-and-private-record-split.md; spec/non-functional/NFR-010-no-cryptography-of-our-own.md"
review_set: subset
---

## Summary

The new null-refusal and signature-input criteria are concrete, observable, and consistent with the existing closed-schema and signing boundaries. The added NFR criteria are independently falsifiable and match the EA-117 acceptance; no base checklist defect was found. Criterion-strength analysis was skipped because its installed skill forbids execution without the Jev client, which is unavailable in this environment.

## Scope

Examined the changed criteria FR-055-AC-4, FR-056-AC-9, FR-056-AC-10, NFR-010-AC-1, and NFR-010-AC-2; checked their surrounding requirement statements, dependencies, the existing FR-057 gate, NFR-008 closed-schema rule, and current code/test declarations for grounding. The PR changes only the three requirement artifacts listed in frontmatter; it does not change `spec/tests.md`, test code, or mint test/TC identifiers. The reviewed text contains no paywalled standard material.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found (placeholder). | - |
