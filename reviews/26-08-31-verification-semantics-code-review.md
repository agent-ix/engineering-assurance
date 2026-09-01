---
id: SR-020
title: "Code review — verification-semantics specification gate"
type: SpecReview
analysis: code-review
scope: "9b7e302..f3ded86; ADR-001; StR-002; US-005; FR-008..FR-010; NFR-004; IT-005; PLAN-002; TM-001; CI policy"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-002"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/TM-001"
    type: references
---

# SR-020: Code review — verification-semantics specification gate

## Summary

Reviewed the complete Engineering Assurance #5 specification candidate against
the issue contract, epic architecture, Quire/Quoin ownership ADRs, historical
PGM-01 requirements, package invariants, and manual-only hosted-CI rule. The
review found two documentation integrity defects and one repository-control
defect; all are corrected in the review remediation commit.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Resolved: the inherited CI workflow still ran on `push` and `pull_request`; both runs were cancelled, the workflow now exposes only `workflow_dispatch`, and a regression test parses the event set exactly. | `.github/workflows/ci.yml`; `test_hosted_ci_is_manual_only` | implementation-bug-despite-evidence |
| FND-002 | medium | Resolved: the master specification described onboarding as the only capability; it now includes vocabulary/type fit, compatibility, projections, and execution/persistence exclusions. | `spec/spec.md`; FR-008..FR-010 | missing-requirement |
| FND-003 | medium | Resolved: TM-001 said every row was implemented; it now separates the 51 completed onboarding rows from the 17 planned #5 rows. | TM-001; TC-052..TC-068 | wrong-requirement |

## Boundary Review

- No runner, subprocess execution, generic stdout verdict scraper, evidence
  store, parallel envelope, audit engine, or human-decision inference exists.
- Quire and Quoin remain non-executing.
- Historical PGM-01 compatibility is read-only and cannot synthesize success.
- The eight repository migration tickets remain untouched.

## Verification

Ruff, 128 pytest tests, manifest validation, package audit, Quire document
validation, and `git diff --check` pass locally. GitHub comments, reviews, and
inline review threads were read and contained no finding at review time.
