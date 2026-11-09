---
id: SR-16397
title: "spec-review of plan-declared constant-predictor answer families (engineering-assurance PR 199)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@7efdaee; spec/functional/FR-020-measurement-plan-objective.md; spec/functional/FR-021-measurement-plan-decision-rule.md; spec/tests.md; engineering_assurance/skeletons/MeasurementPlan.md; engineering_assurance/skills/assurance-onboarding/SKILL.md"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-021
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-020
    type: references
---

# Spec review of plan-declared constant-predictor answer families (engineering-assurance PR 199)

## Summary

Ticket: PLAT-1020. Scope is `git diff origin/main...origin/plat-1020-spec` only. `quire validate --scope .` over the four changed spec/skeleton files exits 0; the new FR-021 statements add grammar warnings (see the EARS review). Sub-analyses run: integrity, EARS conformance, failure-domain; criterion strength judged by hand (no Jev run).

Checked against origin/main and found consistent:

- Rule shape vs `measurement-plan-frontmatter.schema.json`: `$defs/statistical_design` is `additionalProperties: false` with an existing `allOf` if/then on `decision_rule.baseline: constant-predictor` (estimator must be `proportion`); a sibling `answer_families` property plus an if/then/else on the same condition expresses the rule exactly. `decision_rule` is required inside `statistical_design`, so "threshold or other baseline" is the complete else-branch. The legacy retired prose design is closed and unaffected.
- FR-021 baseline semantics: the existing size-weighted best-constant definition is kept; the new sentence only fixes where families come from.
- Producer data shape: quoin origin/main FR-108-AC-17 and the PLAT-1020 forward note guarantee that the distinct `dimensions.family` values over item rows are exactly the caller's item families, none added, defaulted or merged, each non-empty. FR-021's set-equality comparison over distinct `family` values matches that shape.
- Definition-change choice: consistent with FR-020 (Inputs projection updated) and the FR-024 `protected_apparatus` precedent; the Rust check already reports a typed finding per changed member, so this is one more member, not hidden scope.
- Checker comparison is stated as Quoin's (`quoin-measurement`) with a forward note; no quoin file is edited.
- AC-16..18 / TC-240..242: ids unused on origin/main and on PR 198 (`ea-84-traceability`); PR 198's `spec/tests.md` hunks do not touch the FR-021 rows or the end of the TC table, and a three-way merge-tree of both branches over the shared base shows no conflict. Each AC names concrete accept/refuse cases that a test can fail.
- No ledgers, pins, digests or count assertions added; no workstation paths in committed files.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The skeleton's new declaration example `answer_families: [verdict, weakness_kind]` names a family (`verdict`) that appears nowhere, while the worked example directly below groups the corpus as `weakness_kind` and `adverse_case_coverage`, and Quoin's producer for that same corpus emits `weakness_kind` and `coverage` (quoin FR-108-AC-23). Under exact set equality these names are now load-bearing: a reader copying either spelling for that corpus gets an inconclusive verdict. FR-021-AC-18 also requires the worked example to show the declaration, which it does not. Use the worked example's families in the declaration example, show the declaration in the worked example, and align the family name with the producer's. | engineering_assurance/skeletons/MeasurementPlan.md:304; engineering_assurance/skeletons/MeasurementPlan.md:332-339; spec/functional/FR-021-measurement-plan-decision-rule.md:254 |
| FND-002 | low | Lists of measurement-definition members that this PR does not update still omit `answer_families`: the onboarding paragraph ("The objective's `direction` and `bound`, `estimator`, `decision_rule`, and `protected_apparatus` list are all part of the measurement definition"), the skeleton's Decision Rule section ("The estimator and rule are part of the measurement definition"), and FR-024's "SHALL include `protected_apparatus` alongside `objective`, `statistical_design.estimator`, and `statistical_design.decision_rule`". An author following them would edit the families without a `definition_version` bump. | engineering_assurance/skills/assurance-onboarding/SKILL.md:165-169; engineering_assurance/skeletons/MeasurementPlan.md:171-174; spec/functional/FR-024-measurement-plan-protected-apparatus.md:101-103 |
| FND-003 | low | The skeleton says a merged family is "refused" by the checker, in the present tense; FR-021 specifies a typed inconclusive reason, and the Quoin comparison does not exist yet. Say "inconclusive" and match the spec's outcome. | engineering_assurance/skeletons/MeasurementPlan.md:305-307 |

## Verdict

The rule itself is sound and fits the schema, the FR-021 baseline semantics, the FR-020/FR-024 definition model and the Quoin producer's data shape. AC-16 and AC-18 can fail as written; AC-17 is concrete except for reordering (failure-domain FND-001). Not mergeable until the medium findings here and in the integrity and failure-domain reviews are fixed; the lows are one-line edits in the same files.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit b8a5d3a: the declaration example and the worked example both use [weakness_kind, coverage], matching quoin FR-108-AC-23; the worked example now shows the declaration; no `verdict` or `adverse_case_coverage` remains outside review records. |
| FND-002 | fixed | Fix commit b8a5d3a adds answer_families to the onboarding list, the skeleton Decision Rule sentence, and FR-024's member list. |
| FND-003 | fixed | Fix commit b8a5d3a: the skeleton now says Quoin's checker 'is to' return a typed inconclusive reason, matching FR-021, and points at the Quoin follow-up. |
