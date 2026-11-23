---
id: SR-20261115040622
title: gap-analysis of blank Signed-off-meaning as meaning_missing
type: SpecReview
analysis: gap-analysis
scope: agent-ix/engineering-assurance; FR-041 (AC-5, AC-9, CON-1); TC-321, TC-310; src/approval_verify.rs;
  tests/approval_verify_cli.rs; tests/approval_verify.rs
review_set: subset
relationships:
- target: ix://agent-ix/engineering-assurance/FR-041
  type: references
---

## Summary

Gap analysis scoped to the PR diff. Ticket: EA-134. PR: agent-ix/engineering-assurance#298, head `40319b3`. Plan completion: not assessed.

The changed criteria were traced to tests by their `#[trace]` tags. FR-041-AC-5 maps to TC-321 in `tests/approval_verify_cli.rs`. FR-041-CON-1 maps to TC-310 in `tests/approval_verify.rs`, and to TC-316 and TC-330 in `tests/approval_verify_cli.rs`.

## Verdict

AC-5 is fully backed. TC-321 covers the absent, empty, ASCII-space, Unicode-whitespace, repeated-empty and blank-beside-real cases. The test passes at head and kills all four mutants of the new clause.

The new blank-value clause of CON-1 is backed only by the TC-321 integration test. The TC-310 property test, which AC-9 makes the exhaustive CON-1 check, does not model it. See FND-001.

There is no stub, no ignored test and no untraced production change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-041-AC-9 / TC-310 enumerate meaning by presence only, so the "approved for exactly one combination (CON-1)" property no longer spans CON-1's blank-value clause | tests/approval_verify.rs:31-75 |

## Examined scope

```yaml
scope:
- id: FR-041-AC-5
  path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md
  role: examined
  excerpt: A valid, mapped tag with matching record trailers and no `Signed-off-meaning` trailer yields `meaning_missing` and no `meaning` default; the same tag with a trailer whose value is empty or whitespace-only, alone or repeated, or beside a non-blank value, yields `meaning_missing` and no `meaning`.
- id: FR-041-AC-9
  path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md
  role: examined
  excerpt: The decision from the FR-036 verdict and the binding is exercised without starting any process, covering every combination of verdict status, binding member and meaning presence, and `approved` is returned for exactly one combination (CON-1).
- id: FR-041-CON-1
  path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md
  role: examined
  excerpt: ... and a `Signed-off-meaning` trailer whose every value holds a non-whitespace character.
```
