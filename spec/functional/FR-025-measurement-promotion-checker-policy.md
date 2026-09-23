---
id: FR-025
title: "Read a typed checker result when promoting a measurement, advisory or required per AssuranceProfile"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-005"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-005"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-016"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-024"
    type: "requires"
---

# FR-025: Read a typed checker result when promoting a measurement, advisory or required per AssuranceProfile

## Description

The `measurement-promotion` workflow SHALL accept the independent measurement
checker's result as a typed run item, and its `measurement.promotion_ready`
invariant SHALL bind the promotion evidence to that result. The governing
AssuranceProfile's `measurement_policy` SHALL decide, per proposed stage,
whether the result is only reported (`recommend`) or must be an attested
accept (`require`). An owner overrides a required result only through a
current `exception` item, and the override stays visible in the outcome. An
accepted result never promotes: the decision owner still makes the terminal
choice ([FR-005](./FR-005-resumable-human-decisions.md)).

## Inputs

- An AssuranceProfile frontmatter document with an optional
  `measurement_policy: { mode, stages }`, where `mode` is `recommend` or
  `require` and `stages` is a list of MeasurementPlan stages.
- A `measurement-promotion` run projection
  ([FR-016](./FR-016-rust-onboarding-and-workflow.md)) with:
  - one `promotion_request` interview item and its matching complete
    `promotion_evidence` item, which may carry `plan_id` (the plan's
    frontmatter `id`) and `candidate` (the collection id the checker decided);
  - zero or more `measurement_policy` items, each `{ profile_path, mode,
    stages }` recorded from the governing profile;
  - zero or more `measurement_verdict` items, each one
    `quoin.measurement-verdict.v1` document from Quoin's measurement checker,
    with the members `schema`, `planId`, `definitionVersion`, `verdict`
    (`accept`, `reject`, or `inconclusive`), `reasons`, `claimed`,
    `candidate`, `decisions`, `findings`, `regressedRuns`, `orderSource`, and
    `counts`;
  - zero or more `exception` items;
  - the evaluation instant.

## Outputs

- A schema validation result for the profile.
- For `measurement.promotion_ready`, once the stage step and evidence
  checks pass: an outcome carrying a `checker` report `{ mode, status,
  verdict, reasons, exception_override }` whether it passes or fails, where
  `status` is `accepted`, `order_unattested`, `not_accepted`, `mismatch`, or
  `missing`.

## Behavior

- The AssuranceProfile schema SHALL accept `measurement_policy` with exactly
  `mode` and `stages`, `mode` from the same `recommend`/`require` vocabulary
  as `review_policy`, and `stages` a non-empty list of distinct values from
  the MeasurementPlan `stage` enum.
- The policy SHALL be recorded into the run as a `measurement_policy` item.
  The invariant evaluator reads only the run projection
  ([FR-016-CON-3](./FR-016-rust-onboarding-and-workflow.md)), so it SHALL NOT
  read the profile file.
- The mode in force SHALL be `require` when any recorded policy has mode
  `require` and lists the proposed stage, and `recommend` otherwise,
  including when no policy is recorded.
- A checker result SHALL be read only when its `schema` is
  `quoin.measurement-verdict.v1` and its `planId` equals the evidence's
  `plan_id`. When the evidence has no `plan_id`, or no result is read, the
  status SHALL be `missing`.
- A read result SHALL match when its `definitionVersion` equals the
  evidence's `definition_version` and its `candidate` equals the evidence's
  non-empty `candidate`. When results are read but none matches, the status
  SHALL be `mismatch`.
- A matching result SHALL have status `accepted` when its `verdict` is
  `accept` and its `orderSource` is `git-first-parent-add`, the only intake
  order the producer cannot choose after the fact; `order_unattested` when
  its `verdict` is `accept` with any other `orderSource`; and `not_accepted`
  otherwise. When several results match, the least favourable status SHALL
  win (`not_accepted`, then `order_unattested`, then `accepted`).
- The report SHALL carry the selected result's `verdict` and `reasons`, and
  `null` and an empty list when no result matches.
- In `recommend` mode the invariant SHALL pass whatever the status, carrying
  the report.
- In `require` mode the invariant SHALL fail with
  `promotion_checker_missing`, `promotion_checker_mismatch`,
  `promotion_checker_not_accepted`, or `promotion_checker_order_unattested`
  for the corresponding status, carrying the report, unless a current
  exception exists.
- A current exception SHALL be one with a non-blank owner, rationale, and
  impact and an RFC 3339 expiry later than the evaluation instant, the same
  test `shared.exceptions_ready` applies. When one exists and `require` mode
  would fail, the invariant SHALL pass with `exception_override: true`. The
  flag SHALL be false in every other case.
- The Rust evaluator and the retained JavaScript provider SHALL produce the
  same outcome for every valid projection. The Rust evaluator SHALL refuse a
  projection whose checker or policy item has an unknown member, an unknown
  `verdict` or `mode`, or a mistyped member.
- The workflow's terminal transitions SHALL remain human-gated. A passing
  `measurement.promotion_ready` SHALL only let the run reach `decision_ready`.
- Engineering Assurance SHALL own the item schema it accepts and SHALL NOT
  depend on Quoin code.

## Error Conditions

A profile policy with a missing or extra member, an unknown mode, an empty,
repeated, or unknown stage list fails validation. A malformed checker or
policy item is a refused request, not a failed invariant. In `require` mode
an absent, unbound, stale, rejected, inconclusive, or order-unattested checker
result fails the invariant with its typed code unless a current exception is
recorded.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-025-AC-1 | The AssuranceProfile schema accepts no `measurement_policy` and a policy with either mode and a non-empty list of distinct MeasurementPlan stages; it refuses a missing mode or stage list, an unknown mode, an empty, repeated, unknown, or non-list stage list, an extra member, and a non-object policy; its stage values equal the MeasurementPlan `stage` enum and its modes equal `review_policy`'s. | Test (TC-163) |
| FR-025-AC-2 | With no policy, a `recommend` policy, or a `require` policy that does not list the proposed stage, `measurement.promotion_ready` passes for a missing, mismatched, rejected, order-unattested, and accepted checker result, carrying a `recommend` report with the matching status, verdict, and reasons. | Test (TC-160) |
| FR-025-AC-3 | With a `require` policy listing the proposed stage, an attested accept passes; a missing result, evidence with no or another plan id, another schema, a reject, an inconclusive, another definition version, another or null candidate, evidence with no candidate, and a `caller-supplied`, `none`, or `git-shallow` order fail with the typed code and report; conflicting matching results fail as `not_accepted` in either order. | Test (TC-161, TC-164) |
| FR-025-AC-4 | With a `require` policy, a current owned exception lets a rejected or missing result pass with `exception_override: true`; an exception expiring at the evaluation instant or without an owner does not; the flag is false for an accepted result and in `recommend` mode. | Test (TC-162) |
| FR-025-AC-5 | For every case of AC-2 to AC-4 the Rust evaluator and the retained JavaScript provider produce the same outcome bytes, and the Rust evaluator refuses an unknown verdict or mode, a mistyped reasons or stages member, and an unknown member on either item. | Test (TC-160, TC-161, TC-162) |
| FR-025-AC-6 | The canonical and pilot `measurement-promotion` definitions are equal, declare `measurement_verdict` and `measurement_policy` item schemas with every document member required, gate `evidence_ready -> decision_ready` on `measurement.promotion_ready` and `shared.exceptions_ready`, and keep both terminal transitions `hitl`. | Test (TC-164) |
| FR-025-AC-7 | The AssuranceProfile skeleton carries a valid `measurement_policy` and a section explaining it, the onboarding skill describes the policy, the run items, the attested order, the failure codes, and the override, and the onboarding checklist lists the policy's modes, closed stage values, list shape, and required members with no warning. | Test (TC-165) |

## Dependencies

- **Upstream**: [FR-005](./FR-005-resumable-human-decisions.md) for the
  human terminal decision; [FR-016](./FR-016-rust-onboarding-and-workflow.md)
  for the invariant request and result protocol;
  [FR-024](./FR-024-measurement-plan-protected-apparatus.md) for the
  measurement definition the checker's result is bound to.
- **Downstream**: Quoin's measurement checker (Linear PLAT-961), which
  produces the `quoin.measurement-verdict.v1` document.
