---
id: SR-041
title: "Gap analysis — Rust evidence classification slice"
type: SpecReview
analysis: gap-analysis
scope: "PR #27; FR-015-AC-1..AC-3; spec/tests.md TC-100, TC-102, TC-103; src/evidence.rs; tests/evidence_parity.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: "references"
---

# SR-041: Gap analysis — Rust evidence classification slice

## Summary

Verification gate over the FR-015 evidence-classification slice landed by PR
#27. The three acceptance criteria the PR claims (AC-1, AC-2, AC-3) each
resolve to a tracking-tagged test that the engine counts as backing, and the
206→207 traceability movement is real. Two gaps remain: the Rust migration is
being landed with **no plan bundle at all**, and the Test Matrix still reports
every row it now backs as pending.

## Verdict

**FAIL** — no plan bundle exists for the FR-014..FR-018 migration, so step 1 of
this gate has no target, and the matrix status markers contradict the engine.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-090 | high | No plan bundle exists for the Rust migration. `plan/` holds only `PLAN-001-assurance-onboarding` and `PLAN-002-verification-semantics`; PRs #22–#27 implement FR-014..FR-018 with no `Plan`/`Task` artifacts, so plan-completion cannot be asserted for this slice or any sibling, and the remaining migration work is tracked only in PR prose. | `plan/`; PR #27 "Remaining #59 work" |
| FND-091 | medium | The Test Matrix understates what is now backed. `make integration-traceability` counts FR-015-AC-1..AC-3 and TC-100/102/103 as backed, while `spec/tests.md` still marks all six rows `🚧 pending implementation` — the gate reports this directly as "test matrix retains non-passing status markers". A reader cannot tell which subcases landed. | `spec/tests.md:110`; `spec/tests.md:272` |
| FND-092 | medium | FR-015-AC-1 requires byte-identity "over the accepted corpus and focused fictional cases". TC-100's evidence half runs fictional cases only; the accepted corpus is not exercised for the evidence identity domain, so the row is backed by a strictly narrower test than the criterion states. | `tests/evidence_parity.rs:265`; `spec/functional/FR-015-semantic-and-identity-parity.md` |
| FND-093 | medium | Reverse gap: `AvailabilityState::as_str` and `EvidenceEnvelope::is_valid` are public surface with no owning requirement text, and `EvidenceValidationError` exposes no accessor for its message, so a caller can only match on the rendered string. `validate_state_labels` is public and untraced from `classify_producer`, unlike the reference where it gates every classification. | `src/evidence.rs:36`; `src/evidence.rs:57`; `src/evidence.rs:330` |
| FND-094 | low | The traceability scanner walks `corpus/` fixtures and `vendor/ix-trace-rs`, so `TC-999`, `TC-1`, `TC-1598`, `FR-001-INV-1`, and `FR-003-CON-1` are permanently reported as untracked symbols. Pre-existing, not introduced by #27, but it keeps the gate's output noisy enough to hide a real untracked tag. | `scripts/check_integration_evidence.py`; `corpus/cases/join/`; `vendor/ix-trace-rs/src/lib.rs` |

## Coverage

- Plan completion: **not assessable** — no plan bundle for FR-014..FR-018 (FND-090).
- Matrix verification: FR-015-AC-1, AC-2, AC-3 backed by `TC-100`, `TC-102`,
  `TC-103` in `tests/evidence_parity.rs`. AC-4..AC-7 (`TC-104`, `TC-119`,
  `TC-120`, `TC-122`) remain unbacked, as the PR declares.
- Repository rollup: 207/262 backed; test-case traceability 103/128. Both moved
  in the right direction and neither regressed.
- Underspecified code: FND-093.
- Semantic review: **performed**. Intent↔test↔code was checked for AC-1..AC-3
  against the retained Python reference line by line and by differential probe;
  the one intent violation found is recorded as FND-084 in SR-040, not repeated
  here.

## Relation to SR-040

SR-040 covers the implementation defects in the same slice. This document covers
only plan, matrix, and specification coverage. The high finding in each is
independent: SR-040 FND-084 is a wrong identity; SR-041 FND-090 is missing
process evidence.
