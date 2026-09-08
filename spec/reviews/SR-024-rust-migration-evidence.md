---
id: SR-024
title: "Evidence review of the Rust-native Engineering Assurance migration"
type: SpecReview
analysis: evidence
scope: "StR-003, ADR-002, FR-014..FR-018, NFR-005, TC-096..TC-127"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/NFR-005"
    type: reviews
---

## Summary

This is correctly a specification-only candidate: every Rust migration TC is
pending and no implementation evidence is claimed. Human review independently
confirmed the AssuranceProfile frontmatter rejection and correctly challenged
the earlier ambiguous MeasurementPlan wording. Exact Quire commands at the
pinned consumer revisions now show that MeasurementPlan frontmatter passes and
the required body-section contract rejects those documents. The deterministic
Quoin advisor could not execute in this sandbox and GitHub reports no branch
checks.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-057 | medium | `quoin advise --repo . --json` is unavailable in the review environment because the sandbox denies Quoin's nested Node→`quire` spawn with `EPERM`. Direct `quire --version` reports 0.31.0 and direct validation succeeds, so this is an evidence limitation—not a version or #59 implementation defect—and advisor output must not be invented. | Quoin 0.23.1 `src/quire/exec.ts`; Quire 0.31.0; review run 2026-09-08 | correct-requirement-no-evidence |
| FND-058 | medium | PR #22 has no configured status checks. The reported 181-test baseline and validation results are local observations, not branch-reproduced evidence; implementation PRs need an immutable CI gate before they can discharge any pending TC. | engineering-assurance PR #22 statusCheckRollup; spec/tests.md:464-475 | correct-requirement-no-evidence |

## Evidence disposition

The specification may be reviewed without pretending pending implementation
tests pass. Acceptance must distinguish direct validation, human review,
unavailable advisor output, and future implementation evidence.

The direct Quire evidence is retained in ADR-002 with exact revisions and
commands. It resolves the human-review request without broadening a body
contract failure into a frontmatter-schema claim.

## Repeat-review dispositions

| ID | Disposition |
| --- | --- |
| FND-057 | Open environment limitation, explicitly retained. Direct Quire results are reported separately; no advisor verdict is claimed. |
| FND-058 | Specification gap fixed by NFR-005-AC-6/TC-127. Evidence remains pending until implementation PRs expose the required hosted statuses. |

The amended-branch `make integration-gate` run passed Ruff, content rights, 181
tests (2 skipped), module validation, package audit, and document validation.
Its final traceability leg correctly refused 191/260 with TC-096..TC-127 and
their criteria unbacked; that non-zero result is the expected pre-implementation
state and is not reported as a passing integration gate.
