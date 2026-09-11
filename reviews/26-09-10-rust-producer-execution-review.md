---
id: SR-086
title: "Rust review of bounded producer execution and shared process kernel"
type: SpecReview
analysis: code-review
scope: "FR-014-AC-4, FR-019, NFR-004-AC-2, TC-122 through TC-127; src/producer_execution.rs; src/process_host.rs; src/onboarding_host.rs; tests/producer_execution.rs; tests/source_audit.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/NFR-004"
    type: reviews
---

## Summary

The repository rules and exact `agent-skills/rust-review/SKILL.md` checklist
were applied to the public producer-execution API, retained executable/input
acquisition, process supervision, resource boundaries, typed result states,
caller-owned adapter seam, existing binary-host cutover, and static ownership
gate. The implementation incorporates all seven consumer pre-freeze findings
recorded by Agent B and closes five additional Rust-review findings.

The provider implementation is complete and locally passing. Final issue
acceptance remains conditional only on TC-126, the separately owned real
`quire-verification` consumer integration against the published Engineering
Assurance revision.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-199 | high | Closed: the first implementation added a second spawn, timeout, process-group, and bounded-stream loop beside the existing binary host adapter. Both public producer execution and every existing binary adapter now call one `capture_command` supervision kernel; `process_host.rs` is a compatibility facade with no child-process capability. | `src/producer_execution.rs:1515`; `src/producer_execution.rs:1578`; `src/process_host.rs:11`; FR-019 Behavior; TC-127 |
| FND-200 | high | Closed: onboarding validation still invoked `Command::output` directly, bypassing both the existing host adapter and the new ownership gate with unbounded time and output. It now uses the shared kernel with explicit 30-second and 8-MiB limits while preserving its prior success/diagnostic mapping. | `src/onboarding_host.rs:285`; `src/onboarding_host.rs:300`; FR-014-AC-4; TC-127 |
| FND-201 | medium | Closed: the first kernel extraction narrowed the existing process host from Unix process-group support to Linux-only support. Shared group supervision remains available on Unix; only the stronger public retained-descriptor and descendant-observation profile is Linux-specific, and unsupported hosts still refuse before spawn. | `src/producer_execution.rs:1578`; `src/producer_execution.rs:1631`; `src/producer_execution.rs:1987`; FR-019-CON-4 |
| FND-202 | medium | Closed: TC-127 initially passed under Cargo but Quire classified its function as a non-evidence symbol because the trace macro separated `#[test]` from the function declaration. The bare trace now precedes an adjacent `#[test]`; Quire binds TC-127 and FR-019-AC-5, increasing repository coverage from 239/260 to 240/260. | `tests/source_audit.rs:217`; FR-019-AC-5; TC-127 |
| FND-203 | low | Closed: the first ownership gate used textual substring checks that comments and string literals could trigger. It now parses Rust syntax, visits nested and renamed imports plus direct paths, and kills direct, nested-alias, and `std`-alias child-process mutants while admitting `ExitCode`. | `tests/source_audit.rs:123`; `tests/source_audit.rs:161`; `tests/source_audit.rs:217`; FR-014-AC-4; TC-127 |
| FND-204 | low | No open Rust implementation finding remains. Production adds no unsafe block, panic, unchecked numeric cast, lint suppression, shell invocation, ambient environment in the public executor, unbounded request population, evidence persistence, domain oracle, or automatic decision. New requirement tests use bare `ix_trace_rs::trace` markers. | reviewed source; FR-019; NFR-005 |

## Gate results

| Gate | Result |
| --- | --- |
| QUOIN `/specify` and owner-selected base `/spec-review` | pass after 18 specification and implementation findings were resolved in SR-085 |
| Exact Rust 1.98.1 formatting | pass; inherited vendored ix-trace-rs nightly-option warnings remain visible |
| Exact Rust 1.98.1 Clippy | pass; all targets, all features, locked, `-D warnings`, two build jobs |
| Exact Rust 1.98.1 tests | pass; complete library, binary, integration, and documentation target suite |
| Exact Rust 1.98.1 rustdoc | pass; workspace, all features, no dependencies, locked, `-D warnings` |
| `cargo deny --locked check` | pass; advisories, bans, licenses, and sources; existing duplicate transitive versions remain policy warnings |
| `cargo audit` | pass; 153 locked dependencies scanned against 1,243 advisories |
| `make lint` | pass |
| `make test` | pass; content-rights tree accepted 283 entries, 170 retained Python tests passed, and module validation passed |
| `make package-audit` | pass; wheel 70 files, npm 60 files, and six canonical installed files agreed |
| Local Quire coverage | provider rows TC-122 through TC-125 and TC-127 backed; repository 240/260; TC-126 remains the expected consumer-owned gap |
| Hosted CI | not run; local-only policy preserved |

## Mutation evidence

- Replacing a retained selected-input path after preflight still exposes the
  sealed original bytes to the producer.
- Moving an escaped descendant into a new session produces
  `containment_failure`; removing the observation lets the mutant survive.
- Re-nesting, aliasing, or directly qualifying a child-process import outside
  `producer_execution` fails TC-127.
- Removing either public or legacy invocation of `capture_command` fails the
  exact shared-kernel census.
- Reversing the TC-127 test/trace attribute adjacency causes Quire to report
  FR-019-AC-5 unbacked even though Cargo still executes the test.

## Review disposition

**PASS for Engineering Assurance provider implementation quality. CONDITIONAL
for final #34 acceptance until the real `quire-verification` consumer supplies
TC-126 at the exact reviewed provider revision.** No new crate, repository,
generic CLI runner, evidence store, domain oracle, language grammar, hosted CI,
or public package publication is introduced.
