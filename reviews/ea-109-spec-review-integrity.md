---
id: SR-20261113192965
title: "EA-109 #260 FR-064 integrity review"
type: SpecReview
analysis: integrity
scope: "agent-ix/engineering-assurance#260; FR-064 output contract and AC-16..19"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-064"
    type: reviews
---

## Summary

Checked the new obligations for consistency, atomicity, observability, and single interpretation. The multi-result case is testable, but the output retains one `status` field without defining its value when matching statements have mixed statuses.

## Verdict

FAIL pending an explicit deterministic status aggregation rule or a result shape that reports each statement status.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The singular result status is ambiguous for a set of matching statements with mixed verification outcomes. | spec/functional/FR-064-verify-approval-statement-with-workflow-identity-policy.md:52-53,81-85,114 |

## Disposition

**FND-001 — Resolved.** FR-064 now defines a deterministic aggregate status in Behavior and FR-064-AC-20: `verified` is returned only when every matching statement verifies; otherwise the result uses the first present status in the fixed precedence `unverifiable`, `signature_invalid`, `untrusted_signer`, `wrong_subject`, `predicate_invalid`. When multiple matched results are `unverifiable`, the reason uses fixed precedence `input_unreadable`, `publish_refused`, `statements_not_retained`. AC-16 retains exact-key, all-match verification and counts only verified statements toward FR-061. AC-20 includes the mixed `verified`/`untrusted_signer` oracle.

## Dispositions

Round 1, verified against frozen PR head 3fbc37725b1751310cf9c3f9a519d7ad5eaf352f and current main 98b043ffa.

| FND | outcome | reason / after_excerpt |
| --- | --- | --- |
| FND-001 | fixed | For a non-empty matching set, the aggregate status is `verified` only when every statement verifies; otherwise it is the first present status in the fixed order `unverifiable`, `signature_invalid`, `untrusted_signer`, `wrong_subject`, `predicate_invalid`. Thus, one `verified` and one `untrusted_signer` statement produce aggregate `untrusted_signer`, while the role verdict counts only the verified statement; when multiple results are `unverifiable`, the result reason is the first present in the fixed order `input_unreadable`, `publish_refused`, `statements_not_retained`. |
