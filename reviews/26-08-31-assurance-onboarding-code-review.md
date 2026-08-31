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
`4711d85cb94fb75c773bc4ce018e1ca16b556821`, including onboarding confinement,
atomic publication, evidence-state classification, workflow resume and human
gates, package boundaries, real-agent report retention, and the integration
verifier. No unresolved correctness, safety, test-quality, or spec-alignment
defect was found.

A follow-up reproducibility audit found that the canonical Make target defaulted
to an aggregate under the intentionally ignored `evals/reports/` directory. The
report was available only in the review worktree and contained operational
workstation data; it was never retained in tracked repository content. The fix
separates the tracked-content `integration-gate` from the real-agent
`release-gate`, requires an explicit aggregate for the latter, and uses the
portable `python3` command by default.

## Verdict

**PASS for tracked-content delivery** — no unresolved implementation blocker
remains. The reproducible integration gate passes from tracked content. Release
evidence remains fail-closed and must be supplied explicitly from the authorized
operational evidence store; no agent session is executed by either verification
target.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Fixed: `make integration-gate` defaulted to an ignored, workstation-local aggregate and the review incorrectly called it repository-retained. Tracked-content and operational release gates are now distinct, and release evidence requires an explicit path. | `Makefile`; `README.md`; TASK-007; TC-034; TC-038; TC-039 | implementation-bug-despite-evidence |
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
passed Ruff, content-rights checks, pytest, manifest validation, real wheel/npm
package auditing, Quire document validation, and 101/101 traceability without
reading `evals/reports/`. A separate `make release-gate
EVAL_AGGREGATE_REPORT=...` run revalidated the available 28/28 operational
aggregate, but that workstation-local evidence is intentionally not claimed as
tracked repository content. No test skip, internal-logic mock, placeholder body,
TODO/FIXME/XXX, or assertion-free target test was found.

## Semantic Review

The optional broad semantic expansion of gap analysis was not run. This code
review did perform the required specification-faithfulness and code-test
alignment checks for the changed implementation.
