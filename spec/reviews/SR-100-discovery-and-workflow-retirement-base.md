---
id: SR-100
title: "base review of the canonical-discovery and workflow retirement"
type: SpecReview
analysis: base
scope: "spec/functional/FR-018-staged-runtime-migration.md, spec/tests.md; FR-002-AC-1..AC-5, FR-002-CON-1, FR-002-CON-2, FR-005-AC-1..AC-7, FR-016-AC-3, NFR-001-AC-1..AC-3, StR-001-VC-3, TC-003, TC-009..TC-013, TC-026..TC-030, TC-038, TC-041, TC-042, TC-047, TC-048, TC-107"
review_set: base
---

## Summary

Base checklist review of the FR-018 behavior added for the canonical-discovery
and ix-flow workflow capability, and of the Test Matrix rows whose backing moves
from Python to Rust with it. No acceptance criterion is added and none is
reworded: every criterion under review already existed, and the change is which
implementation carries it.

The review was performed by auditing each assertion in `tests/test_discovery.py`,
`tests/test_workflow_resume.py` and `tests/test_workflow_integration_gate.py`
against the Rust assertion that would fail if the same property were violated,
rather than by reading the Test Matrix. Fourteen test cases were recorded green
while bound only to Python. Seven of the fifteen criteria they carry had no Rust
assertion at all, and one had an assertion that could not fail.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Fourteen test cases — TC-003, TC-009 through TC-013, TC-026 through TC-030, TC-038, TC-041, TC-042, TC-047 and TC-048 — read green while every tag binding them lived in a Python test. Deleting those tests without first restating the criteria in Rust would have unbound all fourteen with no matrix row changing colour. | FR-002, FR-005, NFR-001, StR-001-VC-3 | correct-requirement-no-evidence |
| FND-002 | high | FR-002 had no Rust implementation at all: there is no `src/discovery.rs`. All five of its acceptance criteria and both of its constraints were carried by Python only, so the whole canonical-bundle discovery contract was one deletion away from unbacked. | FR-002-AC-1..AC-5, FR-002-CON-1, FR-002-CON-2 | correct-requirement-no-evidence |
| FND-003 | high | TC-038 asserted that the skill files resolved by the four host surfaces share one digest, but discovery returns the same path for all four surfaces by construction, so the digest set had exactly one member whatever the bundle contained. The assertion could not fail. NFR-001-AC-2, which is about the promoted workflow definitions rather than the skill file, was not checked by it at all. | NFR-001-AC-1, NFR-001-AC-2, TC-038 | correct-requirement-no-evidence |
| FND-004 | medium | TC-027 iterated the canonical workflow definitions found by a glob and asserted inside the loop body only. An empty or renamed `workflows/` directory passed it over an empty population, which is the exact condition FR-005-AC-2 exists to detect. | FR-005-AC-2, TC-027 | correct-requirement-no-evidence |
| FND-005 | medium | FR-005-AC-1 requires that reinvocation preserve completed phases. The Rust lifecycle test resumed a run twice without advancing it between calls, so it demonstrated idempotence but never that a completed phase is retained across resume. Restored as an advance-then-resume assertion on both phase and state version. | FR-005-AC-1, TC-026, TC-107 | correct-requirement-no-evidence |
| FND-006 | medium | FR-005-AC-7 names the binding as a whole; the Python case exercised four fields and the Rust case exercised one. A change to `decision_boundary`, `workflow_version` or `workflow` was refused by code nothing asserted on. Restored to the full field population. | FR-005-AC-7, TC-048 | correct-requirement-no-evidence |
| FND-007 | medium | FR-005-AC-4 requires a run with no choice to remain non-terminal at its gate; the Rust assertion checked the phase but not that no gate had been opened. FR-005-AC-6 requires one attributed acceptance event; the Rust assertion checked the event but not that no acknowledgement preceded it, so a pre-existing acknowledgement would have satisfied it. Both restored. | FR-005-AC-4, FR-005-AC-6, TC-029, TC-047 | correct-requirement-no-evidence |
| FND-008 | low | The retired Python resolved the ix-flow executable from an explicit argument, then `IX_FLOW_BIN`, then `PATH`, and a test asserted that precedence. The Rust boundary takes the executable as a request field, so the precedence rule does not exist to restate; no criterion referenced it and the test carried no trace tag. Recorded here so its disappearance is a decision rather than an omission. | FR-016-AC-3 | wrong-requirement |

## Checklist Results

- **ID formats** — no requirement, criterion, constraint or test-case identifier
  is added, renumbered or reused. `SR-100` is taken rather than `SR-097` because
  three sibling retirement branches are open against this repository at the same
  revision and each would otherwise claim the same next ordinal; the gap is
  deliberate headroom, not a missing review.
- **Duplicates and gaps** — the FR-018 behavior added here names one capability
  and five files, none of which appear in another FR-018 bullet.
- **Validation-link integrity** — every test case named by a criterion under
  review exists in the Test Case table, and each of those rows names its
  criteria in return. The direction that failed was neither: the rows were
  linked correctly and bound to a language about to be deleted.
- **Coverage rules** — the six rules hold, and hold more narrowly than before:
  each criterion under review now has a test that fails when its property is
  violated, in the language that remains after retirement. The non-obvious ones
  were confirmed by mutation rather than by inspection.
- **Measurability** — FR-018's new behavior names the specific properties that
  must be restated rather than requiring "equivalent behavior" in the abstract,
  because the three previous retirements each passed an equivalence claim while
  losing assertions.
- **Unhappy paths** — discovery is refusal-first in Rust: an unknown manifest
  key, a target that is not the canonical skill source, a bundle-escaping or
  absolute target, an opencode manifest declaring other than exactly one source,
  and a host set that is short, long or duplicated are each a typed refusal at
  the pure boundary rather than a condition a test may or may not look for.

## Method Note

FND-001 through FND-004 share the escape cause `correct-requirement-no-evidence`,
and they share a shape with the findings SR-096 recorded for the previous
retirement: the requirement was right, the matrix row was green, and the backing
could not fail. Two of the specific shapes SR-096 named recurred here exactly —
a check that passes over an empty population (FND-004) and an assertion that
cannot fail because of how the subject is constructed rather than because of what
the bundle contains (FND-003). The added FR-018 sentence forbidding a completion
claim under either shape is the finding generalized, so that the next retirement
inherits the rule instead of rediscovering it.
