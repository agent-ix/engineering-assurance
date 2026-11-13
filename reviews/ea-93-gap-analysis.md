---
id: SR-20261113092143
title: "Gap analysis of the Baseline and Release archetypes (engineering-assurance PR 227)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/engineering-assurance@4110edf; PR 227 (EA-93); spec FR-046, FR-047, FR-050, FR-051, NFR-007, FR-003; tests/semantic_exports.rs; engineering_assurance/"
review_set: subset
---

# Gap analysis of the Baseline and Release archetypes (engineering-assurance PR 227)

## Summary

Ticket: EA-93. Planless gap analysis of PR agent-ix/engineering-assurance#227 at 4110edf over the archetype criteria the ticket delivers (FR-046-AC-1, FR-046-AC-2, and the FR-046/FR-047 closed-schema statements). FR-046-AC-3..AC-6, FR-047's ACs and FR-051's ACs are release-verify and resolution behavior owned by EA-100 and are context only. Matrix from `quoin matrix --json` (quoin 0.30.1) and `quire coverage --json` (quire 0.44.0).

## Verdict

**FAIL** - the two archetype criteria this ticket delivers are untagged in the computed matrix: the new tags do not bind under this repo's TC-row model.

Reverse gap: none. Every schema member and manifest entry has an owning statement (FR-046 Inputs, FR-047 Inputs); the skeleton body headings are structure only. No stubs, no placeholder code and no coverage inflation beyond the trace findings recorded in the code-review artifact.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-046-AC-1 is untagged in the computed matrix (quoin matrix --json: static_status untagged, no binders). quire coverage reports 'criterion-tagged-outside-row ... This tag does not provide backing' for every new tag: this repo binds a criterion through a TC row (every other test uses #[trace("TC-NNN", "FR-...-AC-...")]), and the new tests carry the bare AC id only. Add TC rows/ids for the archetype tests so AC-1 binds. | tests/semantic_exports.rs:505 |
| FND-002 | high | FR-046-AC-2 is likewise untagged in the computed matrix (same cause as FND-001), and its test is also hollow on the duplicate-path clause: a same-path/different-kind Baseline validates (code-review SR-20261113092142 FND-002). | tests/semantic_exports.rs:520 |
| FND-003 | low | Spec drift: FR-003 Behavior still says the manifest exports 'its five artifact types' (and TC-194 at spec/tests.md:432 says 'five'); after this PR there are seven. Update the count or drop it. | spec/functional/FR-003-package-installable-bundle.md:43 |

## Coverage

- Criteria examined: FR-046-AC-1 (untagged), FR-046-AC-2 (untagged). Context only: FR-046-AC-3..AC-6, FR-047-AC-1..AC-12, FR-051-AC-1..AC-9 (EA-100), FR-050-AC-1..AC-4 (README, see code-review FND-005).
- Semantic review: skipped (reviewer subagent, not opted in).
- Plan completion: not assessed
