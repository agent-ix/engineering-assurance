---
id: SR-16401
title: "Code review of answer_families schema and Rust types (engineering-assurance PR 200)"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance@9057c54; engineering_assurance/schemas/measurement-plan-frontmatter.schema.json; src/measurement.rs; tests/measurement.rs; tests/python_port/answer_families.rs; tests/python_port/main.rs; tests/python_port/module.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-021
    type: references
---

# Code review of answer_families schema and Rust types (engineering-assurance PR 200)

## Summary

Ticket: PLAT-1020. Code review with the Rust lane (rust-review, rust-style) over `git diff origin/main...plat-1020-impl` at 9057c54. Examined the schema property and the `allOf` if/then/else, `AnswerFamily`, `AnswerFamilyError`, `AnswerFamilies`, `AnswerFamiliesError`, `DefinitionMember::AnswerFamilies`, the `MeasurementDefinition::changed_members` arm, and the test edits. Contract: FR-021-AC-16..AC-19.

Evidence: the focused `measurement` and `python_port` test binaries pass (37 and 72 tests); `cargo fmt --check` is clean; clippy over the lib and both test targets with `--features full -D warnings` is clean. Schema edge cases checked: constant-predictor with a valid list, a one-entry list, an absent list, `[]`, a duplicate, an empty string, a non-string and a bare string; the three other baselines and a threshold rule, each with and without the list. All behave as FR-021-AC-16 states. A plan with no `decision_rule` also refuses the list through `else`, which matches "required exactly when". The `tests/python_port/module.rs` edit only adds `answer_families` to constant-predictor fixtures. No workstation paths, pins, ledgers or count assertions were added. `git merge-tree` with PR 198 (branch ea-84-traceability) gives no conflict: both PRs touch `spec/tests.md` and `tests/python_port/module.rs`, but in different hunks, and PR 198 adds no constant-predictor fixtures.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The prose that lists the checked definition members is stale. The module doc says the check covers "the objective, estimator, rule, or protected list", and the `definition_change_without_version_bump` rustdoc says "objective, estimator, decision rule, or protected apparatus list". Neither mentions `answer_families`, although `DefinitionMember::ALL` now includes it. The edited module-doc line is also not reflowed (about 120 columns). | src/measurement.rs:10-12; src/measurement.rs:2025-2027 |
| FND-002 | low | `AnswerFamiliesError::Duplicate` carries `family: String`. The sibling `ProtectedApparatusError::Duplicate`, which this type copies, carries the typed `path: ApparatusPath`. Carrying an `AnswerFamily` would keep the validated identity type, as rust-style asks for identity newtypes. | src/measurement.rs:1637-1641 |

## Verdict

The schema rule and the Rust types meet FR-021-AC-16, AC-17 and AC-19. The set-newtype shape matches `ProtectedApparatus`: a BTreeSet newtype with sorted serialization, refusals on both construction and deserialization, and a round trip. The PR deserializes through `Vec<String>` so that an empty entry maps to the distinct `EmptyFamily` error, which is a deliberate and correct deviation from the sibling. Only two low documentation and typing nits.
