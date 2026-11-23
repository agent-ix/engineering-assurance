---
id: SR-20261115040621
title: code-review of blank Signed-off-meaning as meaning_missing
type: SpecReview
analysis: code-review
scope: agent-ix/engineering-assurance; src/approval_verify.rs; tests/approval_verify_cli.rs;
  spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md; spec/tests.md
review_set: subset
relationships:
- target: ix://agent-ix/engineering-assurance/FR-041
  type: references
---

## Summary

Code review with Rust review lane (rust-style loaded). Ticket: EA-134. PR: agent-ix/engineering-assurance#298, head `40319b3`, diff `git diff origin/main...HEAD` (4 files, +36/-15).

`decide_valid` now treats a `Signed-off-meaning` that is absent, or whose values include any value that is empty after `str::trim` (Unicode White_Space), as `meaning_missing`. FR-041 step 3, CON-1, AC-5 and TC-321 are updated to match.

## Verdict

PASS with three low findings; mergeable.

The change is coherent. FR-037 still reports an empty trailer as `present` with an empty value. That is the raw trailer layer, and FR-041 owns the approval decision, so there is no contradiction. The rule agrees with FR-042 (tag creation refuses an empty or space-padded meaning) and FR-059 (an empty GitHub review meaning yields no approval). It is stricter in the right direction for quoin consumers that require the meaning to equal exactly one role: `["reviewed", ""]` used to come back `approved` and then be refused downstream, and now it is `meaning_missing` at the source.

Rejecting a mixed blank-and-real list, rather than filtering out the blank value, follows the FR-041 rule "SHALL NOT default a missing trailer" and keeps a garbled approval from passing. The Rust change is minimal and idiomatic, adds no public API, has no panic path, and the `Decision::status` path structurally carries no approval members.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-041-AC-9 / TC-310 property oracle still models meaning by presence only; the pure decision's new blank-value clause of CON-1 has no property/unit coverage and `is_approval`'s "the one combination CON-1 allows" comment is now incomplete | tests/approval_verify.rs:55-75 |
| FND-002 | low | TC-321 mixed case (`v-blank-and-real`) asserts only the status, not the "no `meaning`" AC-5 states for it (nor no `approver`) | tests/approval_verify_cli.rs:481-493 |
| FND-003 | low | "whitespace-only" in FR-041 step 3 / CON-1 / AC-5 does not say which whitespace; FR-037 trims ASCII only while the code uses Unicode `trim` and TC-321 pins U+00A0/U+2003 | spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:120,218,235 |

## Checks

- Focused test at PR head through locked-build.sh: `cargo test --features full --test approval_verify_cli tc_321` gave 1 passed.
- Mutation in a scratch copy (worktree untouched), with TC-321 run against each mutant. All four mutants were killed:
  - old presence-only rule: fails at `v-empty`;
  - no trim: fails at `v-unicode`;
  - `trim_ascii`: fails at `v-unicode`;
  - `any` instead of `all`: fails at `v-blank-and-real`.
- The `v-spaces` case collapses to `""` in the FR-037 parser (ASCII trim), so it does not exercise `decide_valid`'s trim independently. `v-unicode` covers that.
- Grep of spec/, docs/ and README for the old "empty value is approved" wording found none remaining. FR-037 AC-6 and TC-270/274 intentionally keep the raw `present`-with-empty-value behaviour.

## Examined scope

```yaml
scope:
- id: FR-041-AC-5
  path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md
  role: examined
  excerpt: A valid, mapped tag with matching record trailers and no `Signed-off-meaning` trailer yields `meaning_missing` and no `meaning` default; the same tag with a trailer whose value is empty or whitespace-only, alone or repeated, or beside a non-blank value, yields `meaning_missing` and no `meaning`.
- id: FR-041-CON-1
  path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md
  role: examined
  excerpt: A result SHALL NOT be `approved` unless ... and a `Signed-off-meaning` trailer whose every value holds a non-whitespace character.
- id: FR-041-AC-9
  path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md
  role: examined
  excerpt: The decision from the FR-036 verdict and the binding is exercised without starting any process, covering every combination of verdict status, binding member and meaning presence, and `approved` is returned for exactly one combination (CON-1).
- id: FR-037-AC-6
  path: spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md
  role: context_only
  excerpt: A message with no such trailer yields `missing` with no values, and a trailer with an empty value yields `present` with an empty value.
- id: src/approval_verify.rs#decide_valid
  path: src/approval_verify.rs
  role: examined
  excerpt: meaning.presence == Presence::Present && meaning.values.iter().all(|value| !value.trim().is_empty())
```
