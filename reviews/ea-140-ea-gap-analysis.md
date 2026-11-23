---
id: SR-20261115040624
title: "gap-analysis of the risk management presentation in the release decision (engineering-assurance PR 300)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/engineering-assurance@feb71ac91; FR-048-AC-18, FR-048-AC-19, FR-048-AC-20; tests/release_decision_cli.rs; src/release_decision_host.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-048
    type: references
---

# Gap analysis of the risk management presentation in the release decision (engineering-assurance PR 300)

## Summary

Ticket: EA-140 (part 2). Plan completion: not assessed. `quoin matrix --repo . --json` (quoin 0.36.8) shows FR-048-AC-18, AC-19 and AC-20 as `tagged`, each bound by exactly one test in tests/release_decision_cli.rs:

- AC-18: `the_presentation_shows_the_risk_files_verbatim_or_not_provided`
- AC-19: `the_release_choice_does_not_depend_on_what_the_risk_files_say`
- AC-20: `a_risk_file_of_the_wrong_shape_is_unreadable_and_stops_before_the_gate`

Each binding was read against its AC and is correct. All 26 tests of `release_decision_cli` pass.

Reverse gaps: every new production branch has an owning statement. These are the present-only checks on the three request members, the `rows` check, the frontmatter `type` check and the `risk` presentation member. There are no stubs.

Mutations, applied temporarily and reverted (the worktree is clean at feb71ac91):

- M1 refused `release` whenever the presentation contained `not_accepted`. The AC-19 test killed it: the `not_accepted`/`no`/`not_ready` case failed. Gate independence from the supplied risk files is therefore enforced by a test.
- M2 added a host verdict member `"acceptability": "accepted"` to `presentation.risk`. It survived all three risk tests. This is recorded as FND-001 in the code-review artifact (SR-20261115040623) and not repeated here.

Static trace: `decide` and `release_outcome` never read `presentation.risk`, so no other refusal ground can depend on the risk files.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-048-AC-19 claims the `release` choice is "accepted and refused on exactly the same grounds" whatever the risk files say. Under the three risk variants, the test exercises only one refusal ground (`anomaly_not_accepted`) plus acceptance. `baseline_item_absent`, `record_path_present_at_baseline` and `meaning_invalid` are not exercised with risk files supplied. Static tracing shows the decide path never reads the risk member, so the risk is low. Either narrow the AC to the grounds tested, or add one variant that also supplies the risk files to the existing TC-467 refusal. | tests/release_decision_cli.rs:1247-1273 |

## Verdict

All three new ACs are tagged, bound correctly and can fail. M1 proves that the gate-independence test bites. One low coverage gap on AC-19's "exactly the same grounds". The medium oracle weakness on AC-19's "no verdict of its own" clause is in the code-review artifact.
