---
id: SR-088
title: "Rust review of producer-execution consumer remediation"
type: SpecReview
analysis: code-review
scope: "FR-019, IT-006, TC-122 through TC-128; Cargo.toml; src/lib.rs; src/producer_execution.rs; tests/producer_execution.rs; tests/fixtures/producer_execution_fixture.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/IT-006"
    type: reviews
---

## Summary

The repository rules and exact `agent-skills/rust-review/SKILL.md` checklist
were applied after Agent B exercised the first real consumer boundary. The
review covered canonical result retention, output descriptor lifetime, staged
working-input closure, first-party fixture ownership, Cargo feature isolation,
process supervision, resource bounds, path topology, panic/unsafe surface,
trace markers and the default package surface.

All five consumer findings and one additional path-topology finding are closed.
No open Rust implementation finding remains. Provider acceptance is
conditional only on TC-126, the separately owned real `quire-verification`
consumer integration against the published revision.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-205 | high | Closed: the public result had no canonical bytes or result identity, so a consumer would have needed a local mapping before retention. Every portable result field and all eight states now serialize through RFC 8785 and `sha256-jcs`; a caller observation that refuses serialization returns a typed encoding error and no identity. Mutation scenario: skipping `timing` from serialization made TC-122 accept identical identities and was killed. | `src/producer_execution.rs:595`; `src/producer_execution.rs:638`; `src/producer_execution.rs:662`; `tests/producer_execution.rs:437`; FR-019-AC-1; TC-122 |
| FND-206 | high | Closed: output observation hashed a pathname and discarded its descriptor, so a replacement could be read as if it were the hashed artifact. Observation now copies the opened file into a sealed memfd, derives length and digest from that snapshot, retains it behind the artifact, and gives both adapter and caller a fresh reader for the same inode. Mutation scenario: returning `/dev/null` from the reader made the adapter reject the valid fixture and TC-122 failed. | `src/producer_execution.rs:486`; `src/producer_execution.rs:503`; `src/producer_execution.rs:1848`; `tests/producer_execution.rs:376`; FR-019-AC-1; TC-122 |
| FND-207 | high | Closed: inputs were retained but the producer still ran in the original capability root, exposing undeclared and later-mutated files through its working directory. Preflight now creates an invocation-owned tree containing only retained declared inputs and output parents, and launch uses that tree. Mutation scenario: restoring the original root as cwd made the extra-file case report `present` and TC-123 failed. | `src/producer_execution.rs:1268`; `src/producer_execution.rs:1466`; `src/producer_execution.rs:1562`; `tests/producer_execution.rs:658`; FR-019-AC-2; TC-123 |
| FND-208 | high | Closed: TC-122 through TC-125 executed Python and shell programs as first-party fixtures. One bounded Rust fixture target now supplies every success, failure, timeout, environment, cancellation, descendant and containment case; the provider tests contain no foreign-runtime executable binding. | `Cargo.toml:20`; `tests/fixtures/producer_execution_fixture.rs:1`; `tests/producer_execution.rs:109`; FR-019-AC-1 through FR-019-AC-4; TC-122 through TC-125 |
| FND-209 | medium | Closed: the first consumer activated the entire monolithic package dependency closure. The existing crate now exposes `producer-execution` with defaults disabled while `full` remains the default and preserves the library/CLI surface. An offline downstream compile plus activated direct-dependency census proves the lightweight seam. Mutation scenario: adding `regex` to `producer-execution` compiled but failed TC-128 on the resolved EA node. | `Cargo.toml:27`; `src/lib.rs:12`; `tests/producer_execution.rs:1024`; FR-019-AC-7; TC-128 |
| FND-210 | medium | Closed during Rust review: output paths `result` and `result/nested` were accepted or refused according to declaration order because leaf absence was checked while parents were still being created. Parent construction and leaf validation are now separate passes; both permutations refuse before launch. | `src/producer_execution.rs:1502`; `tests/producer_execution.rs:770`; FR-019-AC-2; TC-123 |
| FND-211 | low | No open Rust implementation finding remains. Production adds no unsafe block, panic, unchecked numeric cast, lint suppression, shell invocation, unbounded request population, evidence persistence, domain oracle or hosted CI. Every new requirement test uses bare `ix_trace_rs::trace` markers. | reviewed source; FR-019; NFR-005 |

## Gate results

| Gate | Result |
| --- | --- |
| QUOIN `/specify` | pass; FR-019, IT-006 and TC-122 through TC-128 incorporate FND-043 through FND-047 |
| Owner-selected base `/spec-review` | pass after remediation in SR-087 |
| Exact Rust 1.98.1 formatting | pass; inherited vendored ix-trace-rs nightly-option warnings remain visible |
| Exact Rust 1.98.1 Clippy | pass; all targets, all features, locked, `-D warnings`, two build jobs |
| Exact Rust 1.98.1 tests | pass; 145 library, binary and integration tests plus documentation targets; producer contract 12 of 12 |
| Lightweight downstream compile | pass offline with defaults disabled and only `producer-execution`; activated EA direct dependencies are rustix, serde, serde_json, serde_json_canonicalizer, sha2, tempfile and thiserror |
| Exact Rust 1.98.1 rustdoc | pass; workspace, all features, no dependencies, locked, `-D warnings` |
| `cargo deny --locked check` | pass; advisories, bans, licenses and sources; existing duplicate transitive versions remain policy warnings |
| `cargo audit` | pass; 153 locked dependencies scanned against 1,243 advisories |
| `make lint` | pass |
| `make test` | pass; content-rights tree accepted 286 entries, 170 retained Python tests passed and module validation passed |
| `make package-audit` | pass; wheel 70 files, npm 60 files and six canonical installed files agreed |
| Local Quire coverage | 242 of 262 repository rows backed; TC-122 through TC-125, TC-127 and TC-128 backed; TC-126 is the sole #34 consumer-owned gap |
| Hosted CI | not run; local-only policy preserved |

## Mutation evidence

- Omitting a complete-result field from canonical serialization fails the
  TC-122 field mutation census.
- Replacing the staged cwd with the source capability root fails the TC-123
  extra and changed-unbound file cases.
- Discarding the retained output reader fails both adapter observation and the
  post-result pathname-replacement check.
- Activating `regex` in the lightweight feature fails the resolved direct
  dependency census even though both crates still compile.
- Reintroducing order-dependent output leaf validation fails the two
  parent/leaf permutations.

## Review disposition

**PASS for Engineering Assurance provider implementation quality. CONDITIONAL
for final #34 acceptance until the real `quire-verification` consumer supplies
TC-126 at the exact reviewed provider revision.** The consumer findings change
the existing crate boundary only; they introduce no new crate, repository,
generic CLI runner, evidence store, domain oracle, language grammar, hosted CI
or public package publication.
