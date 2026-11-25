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
