---
id: SR-011
title: "Code review — assurance onboarding delivery rerun"
type: SpecReview
analysis: code-review
scope: "PLAN-001; engineering_assurance/; scripts/; tests/; evals/; Makefile"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-001"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/TM-001"
    type: references
---

# SR-011: Code review — assurance onboarding delivery rerun

## Summary

Reviewed the complete Engineering Assurance #3 delivery from base
`4e6522fc32f82cac82a3489a4192e17a94f539c3` through
`42677b852096929b09386301b78c449a017dbf79`, including onboarding confinement,
atomic publication, evidence-state classification, workflow resume and human
gates, package boundaries, real-agent report retention, and the integration
verifier. No unresolved correctness, safety, test-quality, or spec-alignment
defect was found.

One reproducibility defect was fixed during this rerun: the canonical Make target
defaulted to an absent generic aggregate even though PLAN-001's revision-pinned
aggregate is retained in the repository. The default now names that retained
aggregate, while callers can still override it for later evaluated revisions.

## Verdict

**PASS** — no unresolved blocking finding remains. The full gate passes through
one stable command with real tools and retained evidence; it does not execute an
opencode session during this review.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Fixed: `make integration-gate` defaulted to absent `evals/reports/aggregate.json` instead of the retained aggregate required by PLAN-001. | `Makefile`; TASK-007; TC-034; TC-039 | implementation-bug-despite-evidence |
| FND-002 | low | The installed TestMatrix contract still emits the known `Status` versus `Coverage Status` diagnostic; the fail-closed verifier independently requires all 101 rows and rejects non-passing states. | TM-001; `scripts/check_integration_evidence.py` | wrong-requirement |

## Review Method

- Inspected every changed production, gate, packaging, workflow, evaluation, and
  test surface for stubs, skips, internal mocks, weak assertions, unsafe paths,
  subprocess fail-open behavior, non-atomic writes, and unowned behavior.
- Traced FR-001..FR-007, NFR-001..NFR-003, StR-001, and TC-001..TC-049 through
  concrete tests and the final evidence verifier.
- Confirmed staged artifact publication uses a confined same-directory temporary
  file, validates before `os.replace`, and removes failed staging files.
- Confirmed retained reports and transcripts are digest checked and confined
  after path resolution, and required Quire/ix-flow executables fail closed.

## Validation Evidence

`make integration-gate` with the repository's pinned Python and Quire environments
passed Ruff, content-rights checks, 120/120 pytest cases, manifest validation,
real wheel/npm package auditing, Quire document validation, 101/101 traceability,
and the retained 28/28 four-host aggregate. No test skip, internal-logic mock,
placeholder body, TODO/FIXME/XXX, or assertion-free target test was found.

## Semantic Review

The optional broad semantic expansion of gap analysis was not run. This code
review did perform the required specification-faithfulness and code-test
alignment checks for the changed implementation.
