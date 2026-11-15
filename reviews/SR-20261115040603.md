---
id: SR-20261115040603
title: "Spec review — FR-070 action-reference structural validity"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance#280; spec/functional/FR-070-release-and-receipt-workflow-files.md; FR-070 action-reference wording"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-070"
    type: reviews
---

## Summary

Reviewed the FR-070 action-reference wording added for Peter's default-no-pin ruling. The removal of mandatory SHA pins is clear, but the new requirement and acceptance criterion do not define what constitutes a structurally valid action reference.

## Verdict

**CONDITIONAL** — the structural requirement needs a testable syntax contract to avoid accepting malformed `uses:` values.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-070 says action references must be “structurally valid” without defining their grammar or distinguishing external action references from local reusable-workflow references. As written, the AC cannot determine whether an empty owner/path such as `@v4` is invalid, and the new test consequently accepts it. Define the structure to be enforced while keeping SHA pins optional. | spec/functional/FR-070-release-and-receipt-workflow-files.md:62,102 |
