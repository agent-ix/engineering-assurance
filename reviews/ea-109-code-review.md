---
id: SR-20261113192943
title: "Code and Rust review of EA-109 review gate PR #252"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance; PR #252 at 07a3d6b; EA-109; review-gate workflow, policy adapter, Rust evaluator, and ruleset installer"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-059"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-061"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-062"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-063"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-065"
    type: reviews
---

# SR-20261113192943: Code and Rust review of EA-109 review gate PR #252

## Summary

Reviewed PR #252 against `origin/main` at `07a3d6b`, including the EA-109
ruleset scope, FR-059/061/062/063/065 behavior, workflow permissions and
opt-in behavior, edited-review metadata handling, and the sandbox-only
installer. The Rust evaluator's matching, human classification, stale-head
filter, review-state selection, and closed policy/request decoding were
inspected. The sandbox execution evidence from the lead also confirms the
workflow cannot execute its request-preparation script. `cargo fmt --check`
passed; focused Rust tests passed (19 library tests and 4 CLI tests), both
through `locked-build.sh`. `actionlint` is unavailable. The
checked-in ruleset payload does not enforce several required protections.

## Verdict

**FAIL** — the request-preparation script is not executable although the
workflow invokes it directly, so the gate fails before evaluation. The
ruleset data also does not prevent direct pushes or force-pushes and does not
require pull requests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The checked-in ruleset only requires a strict status check; it does not require PRs or prohibit force-push and deletion, so direct changes can bypass the review gate. | `examples/main-review-gate-ruleset.json:12-25`; EA-109 Scope |
| FND-002 | high | The workflow invokes `prepare-review-gate-request.sh` directly, but the file is mode 100644; the sandbox run exits 126 with `Permission denied` before policy evaluation. | `scripts/prepare-review-gate-request.sh:1`; `.github/workflows/review-gate.yml:69-73` |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | The pull-request rule omitted required schema fields and GitHub rejected the ruleset payload with HTTP 422. | `examples/main-review-gate-ruleset.json:13-20` |
| FND-004 | high | A successful empty REST review list was treated as a source outage, so the gate returned unavailable instead of reporting missing roles. | `scripts/prepare-review-gate-request.sh:35-49` |
| FND-005 | medium | `cargo fmt --check` failed on new Rust assertions and import ordering. | `src/approval_gate.rs:942-955`; `tests/approval_gate_cli.rs:10` |

## Coverage

Examined FR-059-AC-1..10, FR-061-AC-1..12, FR-062-AC-1..5,
FR-063-AC-1..6, FR-065-AC-1..10, and EA-109's ruleset requirements. This
includes edited-review metadata completeness and fail-closed behavior,
agent-authored PRs, distinct eligible human matching, protected-base policy
loading, required pull request and status-check settings, strict/up-to-date
branch behavior, stale approvals, last-pusher approval, bypass mode, merge
queue, force-push and deletion restrictions, opt-in workflow execution,
permissions, signing/id-token absence, sandbox installer scope, and deferred
active-repository application. Matrix and trace-tag findings are recorded in
the separate gap-analysis review.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The ruleset now requires pull requests, a strict up-to-date review-gate check, stale-review dismissal, last-push approval, PR-only organization-admin bypass, no force-push, and no deletion. The corrected payload was accepted in the authorized sandbox. |
| FND-002 | fixed | The workflow now invokes the mode-100644 helper with `bash`, and the sandbox run reached policy evaluation. |
| FND-003 | fixed | The ruleset adds the required `require_code_owner_review` and `required_review_thread_resolution` fields; the installer successfully applied it in the authorized sandbox. |
| FND-004 | fixed | An empty review list skips GraphQL edit-metadata lookup; a sandbox run now returns unsatisfied with `no_approval`, as required. |
| FND-005 | fixed | `cargo fmt --check` passes at `af26c50`. |
| FND-001 | fixed | Round 2 re-read confirms the complete ruleset remains applied only to the private sandbox. |
| FND-002 | fixed | Round 2 re-read confirms the workflow still invokes the helper with bash. |
| FND-003 | fixed | Round 2 re-read confirms GitHub accepted the ruleset with required pull-request fields. |
| FND-004 | fixed | Round 2 evidence confirms zero reviews produce `unsatisfied` with `no_approval`. |
| FND-005 | fixed | `cargo fmt --check` passes at the reviewed head. |

