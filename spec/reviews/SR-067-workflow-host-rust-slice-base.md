---
id: SR-067
title: "Base review of the Rust workflow-host slice"
type: SpecReview
analysis: base
scope: "FR-016-AC-3, FR-016-CON-1, FR-016-CON-5..7, TC-107"
review_set: base
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-016"
    type: reviews
---

# SR-067: Base review of the Rust workflow-host slice

## Summary

This owner-selected base review covers only the additive Rust adapter that
starts, resumes, and records explicit human terminal decisions through ix-flow.
It does not approve an invariant-provider bridge, canonical or pilot invocation
cutover, evaluation qualification, consumer migration, or legacy deletion.

The result is **PASS after fixes for specification quality**. The changed
requirement now defines a closed request/result boundary, exact binding and
version preconditions, ix-flow-owned recovery at each mutation window, human
gate invariants, bounded host I/O, and the adverse population required by
TC-107. Acceptance evidence remains gated on a separately human-accepted
compatibility matrix that pins the verified ix-flow 0.2.3 candidate.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | **Closed through `/specify`.** The prior criterion claimed every host refusal left history unchanged, which is unknowable when ix-flow commits before its response times out or becomes unreadable. The contract now distinguishes pre-mutation refusal from an indeterminate post-mutation outcome and requires status-first reconciliation without editing or rolling back ix-flow files. | FR-016 Behavior; FR-016-AC-3; TC-107 | wrong-requirement |
| FND-002 | high | **Closed through `/specify`.** “Interruption/resume” did not define recovery from the two multi-command sequences. The requirement now admits an unbound run only when the ix-flow response proves it pristine, and makes retries after gate deferral, acknowledgement, or terminal advance converge on one bound result and one attributed event. | FR-016 Behavior; FR-016-AC-3; TC-107 | missing-requirement |
| FND-003 | high | **Closed through `/specify`.** ix-flow 0.2.3 adds run-level `full-auto` and per-transition gate overrides; merely omitting an `automatic` request Boolean did not prevent a preconfigured terminal transition from bypassing human review. Rust must pass no override and must verify the selected terminal transition remains `hitl` before advancing. | FR-016 Behavior; FR-016-CON-1; TC-107 | missing-requirement |
| FND-004 | high | **Dependency corrected but not waived.** The accepted matrix pinned ix-flow 0.0.4 while the current public release and installed executable are 0.2.3. Public npm metadata binds 0.2.3 to source revision `8b6cf8287db828b4db2df814bc7c1ef10362db24` and integrity `sha512-tmFIMgOhGS4Tova901cm/yRUANBKhRT3L9Sz/SNxZig1XXR8BGtuXXoETcU6kXqynVLg8uHMyoZ7+AM74Ig/BQ==`; the relevant result-envelope, resume, optimistic-concurrency, and human-gate contracts remain present. The candidate matrix now pins 0.2.3 and honestly returns to unattributed `pending_human_acceptance`. Additive implementation may proceed, but TC-107 cannot pass and enforcement cannot begin until a human accepts it. | FR-012; FR-016 Dependencies; FR-016-AC-3 | correct-requirement-no-evidence |
| FND-005 | medium | **Closed through `/specify`.** The lifecycle adapter had no versioned request/result shape, leaving operation fields, snapshot fields, decision fields, and optional-choice semantics implicit in the old Python dataclasses. Both wire discriminators and their complete typed payloads are now named. | FR-016 Inputs; FR-016 Outputs; TC-107 | missing-requirement |
| FND-006 | medium | **Closed through `/specify`.** The prior requirement bounded neither process duration nor captured output and did not prohibit treating ix-flow next-action strings as commands. Each stream and duration now has a ceiling, arguments bypass a shell, and returned command text remains display-only data. | FR-016 Behavior; FR-016-CON-7; TC-107 | missing-requirement |
| FND-007 | medium | **Closed through `/specify`.** Unavailable, wrong-version, well-formed command failure, malformed read response, and indeterminate mutation were collapsed into two codes. The stable refusal vocabulary now keeps those conditions distinguishable without parsing diagnostic prose. | FR-016 Error Conditions; TC-107 | missing-requirement |
| FND-008 | medium | **Closed through `/specify`.** TC-107 named only interruption, choices, transition failure, and binding mismatch. The matrix now covers pristine recovery, each decision interruption window, exact-version failure, malformed/oversized/timed-out hosts, automatic gates, repeated/opposite choices, and hostile next-action text. | TC-107; Rust Port Permutation Matrix; State Transition Coverage | correct-requirement-no-evidence |
| FND-009 | high | **Closed through the repeated `/specify` cycle.** The initial adapter design trusted a structurally valid status response without asking ix-flow to verify its own hash chain, so tampered history could drive a Rust decision. The requirement now delegates integrity verification to ix-flow before mutation and before every successful result, while preserving the prohibition on reading or reimplementing its state format. | FR-016 Behavior; FR-016-AC-3; FR-016-CON-1; FR-016-CON-6; TC-107 | missing-requirement |

The base checklist finds complete IDs, inputs, outputs, error families,
dependencies, constraint boundaries, transition states, and adverse cases for
this slice. The open matrix acceptance is an explicit upstream decision gate,
not permission to weaken TC-107 or claim the migration has begun.
