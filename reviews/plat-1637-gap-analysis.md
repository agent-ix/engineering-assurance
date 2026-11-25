---
id: SR-20261115040622
title: "Gap analysis — feature risk rating"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/engineering-assurance; feature-risk-rating implementation and trace matrix"
review_set: subset
---

## Summary

Compared the changed workflow and verifier against PLAT-1637, FR-021, linked FR-027 and computed repository trace evidence at PR #315 frozen head `437d3a0372a1ef3f62496048a744786c0ef4d79f`. The main acceptance path is implemented, but linked FR-027 obligations are not cleanly traceable and the verifier process-failure behavior lacks direct test evidence.

## Verdict

**CONDITIONAL** — the implementation is fail-closed on inspected verifier errors, but trace integrity and process-boundary evidence remain incomplete.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new FR-027 trace ids alias this repo's unrelated atomic-publish FR-027, causing the matrix to claim unrelated tests cover atomic-publish AC-1/2 while the linked signer-acceptance criteria have incorrect or absent bindings. | tests/feature_risk_rating_cli.rs:173-285 |
| FND-002 | medium | FR-027's request and fail-closed process/output obligations have no direct test of request serialization or verifier start/timeout/nonzero/malformed-output handling; current classifier tests bypass the process boundary. | src/feature_risk_rating_host.rs:868-918 |

## Matrix and Reverse-Gap Evidence

- `quire matrix --scope . --format json`: exit 0. The local `FR-027-atomic-publish.md` resolves `FR-027-AC-1/2`; rating tests bind to those IDs, generating false coverage. Negative tests for missing meaning, unmapped signer, and principal mismatch all claim AC-2 rather than linked FR-027 AC-5/3/4 respectively.
- `quoin matrix --repo . --json`: exit 0; 926 criteria total, 746 tagged, 88 untagged, 18 tagged only by ignored tests; evidence statuses 907 undischarged and 19 stale. These are repository-wide baseline counts, not attributed wholesale to this feature.
- `quire coverage --scope . --json`: exit 0; reports 39 untracked symbols overall, including external `FR-027-AC-3` and `FR-027-AC-10` tags that have no local criteria. Duplicate module and symbol-id-collision diagnostics were emitted by the current repository index.
- Reverse-gap inspection of the changed verifier found fail-closed code branches for unavailable verifier, process error/nonzero, invalid JSON, protocol/kind mismatch, and invalid response fields. Test coverage does not enter the process error/nonzero/invalid-JSON branches.
- Workflow human gate is configured for both terminal outcomes. Acceptance produces a draft and signed-commit command; the separate CLI acceptance command verifies the resulting commit.

## Method Limits

No semantic review or whole-suite test was run. `cargo check`, focused tests, and workflow parity results are taken from the PR's recorded evidence. Plan completion was not assessed because no plan artifact was supplied.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The exact-request test is tagged to FR-036-AC-8, which requires malformed or unreadable inbound requests to return a machine error; it instead asserts outgoing request shape, falsely crediting AC-8. | src/feature_risk_rating_host.rs:1297-1349 |
| FND-004 | medium | The uncommitted rating-file test is tagged to FR-036-AC-15, which requires signature-verifier commit-id results; it only asserts the `not_committed` acceptance reason. | tests/feature_risk_rating_cli.rs:316-324 |
| FND-005 | medium | FR-037-AC-1 requires successful SSH verification to report the mapped principal; mapped-principal-mismatch and rated_by-validation tests instead exercise acceptance policy/input validation, falsely crediting AC-1. | src/feature_risk_rating_host.rs:1153-1209; tests/feature_risk_rating_cli.rs:300-313 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | New rating tests no longer bind to local `FR-027-AC-*`; computed matrix no longer credits atomic-publish criteria with rating tests. |
| FND-002 | fixed | New focused tests exercise exact request members and unavailable, timeout, nonzero, and malformed-output verifier outcomes. |
| FND-003 | still-open | The exact-request test remains tagged to FR-036-AC-8, which does not specify outgoing request shape. |
| FND-004 | still-open | The uncommitted-file acceptance test remains tagged to FR-036-AC-15 without asserting that criterion's commit-id result. |
| FND-005 | still-open | The two acceptance/input-policy tests remain tagged to FR-037-AC-1 without asserting mapped signature output. |
| FND-003 | fixed | The outgoing request-shape test no longer has the wrong FR-036-AC-8 trace tag. |
| FND-004 | fixed | The uncommitted-file test no longer has the wrong FR-036-AC-15 trace tag. |
| FND-005 | fixed | The acceptance-policy tests no longer carry the wrong FR-037-AC-1 trace tag. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The terminal gate is attributed to `decision_owner`, while signature acceptance is bound to draft `rated_by`; no check requires the same identity, so one person can approve a rating signed by another. This conflicts with the ticket's same-person acceptance/signing flow. | src/feature_risk_rating_host.rs:360-379; src/feature_risk_rating_host.rs:963-965 |
| FND-006 | fixed | Present and decide both require the bound decision owner to match the rating's signed principal. |
