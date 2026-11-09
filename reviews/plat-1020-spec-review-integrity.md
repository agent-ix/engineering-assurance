---
id: SR-16398
title: "Integrity review of plan-declared constant-predictor answer families (engineering-assurance PR 199)"
type: SpecReview
analysis: integrity
scope: "agent-ix/engineering-assurance@7efdaee; spec/functional/FR-020-measurement-plan-objective.md; spec/functional/FR-021-measurement-plan-decision-rule.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-021
    type: references
---

# Integrity review of plan-declared constant-predictor answer families (engineering-assurance PR 199)

## Summary

Ticket: PLAT-1020. Examined: FR-021 Inputs, Outputs, Behavior and Error Handling edits, FR-021-AC-16..AC-18, FR-020 Inputs edit, tests.md rows for TC-240..TC-242, and the existing Rust definition-change types on origin/main (`MeasurementDefinition`, `DefinitionMember`). IDs are well formed, unique and contiguous (FR-021-AC-16..18, TC-240..242); every new AC has exactly one TC row and both tables agree.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-021-AC-17 requires the definition-change check (Rust `definition_change_without_version_bump` over `MeasurementDefinition`, whose `DefinitionMember` set is objective, estimator, decision_rule, protected_apparatus) to report `statistical_design.answer_families`, and quoin FR-108-AC-1 reads plan members through engineering-assurance's Rust types, so the Quoin comparison will need them too. The spec states no Rust representation: no type name, no construction-time validation (empty, duplicate, empty-string), no `DefinitionMember` variant, and the "reachable through the `measurement` Cargo feature" sentence is not extended. Two implementers would build different shapes, and Quoin's follow-up has no stated EA API to read. State the Rust type, its typed refusals and its feature reachability, or state that the definition-change check and Quoin read the list as untyped strings. | spec/functional/FR-021-measurement-plan-decision-rule.md:118-135; spec/functional/FR-021-measurement-plan-decision-rule.md:213-215; spec/functional/FR-021-measurement-plan-decision-rule.md:253 |
| FND-002 | low | The checker requirement ("a checker SHALL require the set of distinct `family` dimension values ... and SHALL return a typed inconclusive reason") is normative SHALL text for a component in another repository, with no AC here and none possible. The following sentences already say the comparison is Quoin's; phrase the checker sentence as the forward note it is (for example "Quoin's checker is expected to ...") so FR-021 holds no unverifiable obligation. | spec/functional/FR-021-measurement-plan-decision-rule.md:125-133 |

## Verdict

Structure and traceability are clean. One medium gap: the Rust side that AC-17 and the Quoin consumer depend on is unspecified.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit b8a5d3a states AnswerFamily/AnswerFamilies, AnswerFamiliesError::{Empty, Duplicate, EmptyFamily}, set compare and sorted serialization, DefinitionMember::AnswerFamilies at statistical_design.answer_families, measurement-feature reachability, and FR-021-AC-19/TC-243. Shape matches ProtectedApparatus (BTreeSet newtype, Empty/Duplicate errors, try_from/into Vec serde) and DefinitionMember::path on origin/main. |
| FND-002 | fixed | Fix commit b8a5d3a rewrites the checker sentence as an explicit non-normative forward note with no SHALL. |
