---
id: SR-090
title: "Rust review of retained evaluation report host"
type: SpecReview
analysis: code-review
scope: "FR-017-AC-1, FR-017-CON-1, FR-017-CON-3, TC-129; Rust report decoder, transcript host, CLI dispatch, and aggregate cutover"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

## Summary

Applied the repository conventions and the exact
`agent-skills/rust-review/SKILL.md` checklist to the additive retained-report
adapter. The pure library accepts a closed `cli-agent-evals.report/v1` shape
without I/O; the binary host owns canonical-root confinement, link and
non-regular-file refusal, bounded reads, transcript hashing, artifact writing,
and current-time rendering. The Make target now calls the Rust CLI while the
Python loader and its consumers remain retained for governed removal.

Four findings found during implementation review are closed. No open Rust
finding remains in this slice. This review does not claim a live agent run,
the still-missing external scenario provider, TC-109 completion, release
evidence, or deletion of the Python implementation.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-158 | high | Closed. Rust initially accepted an explicit `null` where the published TypeScript report contract requires either an omitted optional value or a present nullable value; missing and null transcript identity could therefore be conflated. Required-nullable decoding and focused mutations now refuse the invalid shapes. | `src/evaluation_reports.rs:142`; `tests/evaluation_reports.rs:245` | implementation-bug-despite-evidence |
| FND-159 | medium | Closed. The shared transcript-path validator initially admitted DEL and other ASCII control bytes. It now rejects every ASCII control byte before host path use; focused path mutations prove refusal. | `src/evaluation.rs:794`; `tests/evaluation_reports.rs:245` | implementation-bug-despite-evidence |
| FND-160 | medium | Closed. An empty producer model differed from the retained Python loader, which treats it as the runner default. The decoder now maps missing and empty values to `runner-default`, with a direct assertion. | `src/evaluation_reports.rs:202`; `tests/evaluation_reports.rs:148` | implementation-bug-despite-evidence |
| FND-161 | low | Closed. A prior interrupted invocation could leave the PID-derived aggregate staging name behind and block a later atomic output write after PID reuse. The host now retries bounded, create-new stage names and preserves unowned stale bytes. | `src/evaluation_report_host.rs:277`; `src/evaluation_report_host.rs:474`; `src/evaluation_report_host.rs:916` | implementation-bug-despite-evidence |

## Gate results

| Gate | Result |
| --- | --- |
| Exact Rust 1.98.1 formatting | pass; inherited vendored ix-trace-rs nightly-option warnings remain visible |
| Exact Rust 1.98.1 Clippy | pass; workspace, all targets, all features, locked, `-D warnings`, two build jobs |
| Focused Rust binary tests | pass; 37 tests, including TC-129 root, link, file-kind, digest, resource, ordering, model, and atomic-output cases |
| Focused Rust decoder tests | pass; 6 TC-129 tests covering closed shapes, outcome/retention contradictions, path grammar, source identity, and exact/over report/result limits |
| Direct Rust CLI tests | pass; artifact rendering and same-revision Python/Rust semantic parity for a retained success plus failed attempt |
| Aggregate Make dispatch | pass; dry-run contains the exact Rust 1.98.1 CLI invocation, explicit workspace root, source revision, report paths, and output path |
| Safety scan | pass; no production `unsafe`, panic macro, lint allowance, debug macro, TODO, or unchecked wire conversion introduced |
| Traceability | pass; every new test uses bare `ix_trace_rs::trace` with TC-129 and FR-017 references |
| Live agent evaluation | intentionally not run; user-directed token-bearing evaluation prohibition remains in force |

## Review disposition

**PASS for the TC-129 Rust report/transcript adapter and reversible aggregate
command cutover.** The retained Python loader, integration-evidence consumer,
and legacy-removal work remain outside this reviewed slice and must not be
represented as complete.
