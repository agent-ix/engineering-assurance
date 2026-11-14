---
id: SR-20261113192951
title: "Code review — engineering-assurance EA-107 change-record builders"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance; Cargo.toml; Cargo.lock; src/lib.rs; src/change_record/{mod.rs,predicate.rs,bom.rs}; src/change_record_schemas/change-record-v1.schema.json; tests/change_record.rs; FR-072; FR-073; EA-107 acceptance criteria"
review_set: subset
relationships:
  - { target: "ix://agent-ix/engineering-assurance/FR-072", type: references }
  - { target: "ix://agent-ix/engineering-assurance/FR-073", type: references }
---

## Summary

Reviewed the EA-107 delta against its Rust APIs, tests, schema, and FR-072/FR-073 contracts. The focused test and clippy commands pass, but the predicate builder can emit a digest pair that does not bind the private form's BOM.

## Verdict

**FAIL** — the predicate builder accepts an unrelated BOM digest and private form, so the receipt can claim a BOM that is not in the bound private content.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Predicate builder accepts caller-supplied BOM digest independently of any BOM in the private form | src/change_record/predicate.rs:203-208,311 |
| FND-002 | medium | The missing-role test checks the first sorted approval, which is `developer`, rather than asserting the required missing `engineer` entry | tests/change_record.rs:135-149 |

## Review Notes

- The added code has no vendored CycloneDX schema or copied implementation. `Cargo.lock` resolves `cyclonedx-bom` 0.9.0, whose parser/validator supports `SpecVersion::V1_7`; the focused BOM test exercises that validator.
- Public items in the new Rust module are documented. The new test target is feature-gated and tagged to the relevant FR criteria.
- The schema's closed shape and digest/identifier patterns are consistent with the reviewed FR-072 values. `validate_predicate` applies that schema to generated predicates.
- Focused checks: `locked-build.sh /tmp/ea107-pr263-review-target cargo test --locked --no-default-features --features change-record --test change_record` passed (9 tests); `locked-build.sh /tmp/ea107-pr263-review-clippy-target cargo clippy --locked --no-default-features --features change-record --lib --test change_record -- -D warnings` passed. No aggregate gate was run.
- `quoin matrix` reports the EA-107 test bindings described in the gap-analysis artifact. The remaining untracked TC-672..TC-682 tags are on unchanged EA-110 files and are reported separately to the dispatching lead.

## Assurance Context

No applicable `AssuranceProfile` was found for the changed change-record implementation.

