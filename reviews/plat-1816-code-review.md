---
id: SR-2026112016223475846355053
title: "Code review — Engineering Assurance producer identity"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance; build.rs, src/producer_identity.rs, src/*_host.rs, tests/*_cli.rs, SECURITY.md, four changed FRs"
review_set: subset
---

## Summary

Reviewed PR #314 at `0402dcb87b2ef77fd59280dd420edfd007950820`, including the Rust lane, the changed producer output paths, and their CLI tests. The producer fields are wired through the result paths, but repeated test logic and stale build-script inputs remain defects.

## Verdict

**FAIL** — a duplicated test helper is a high-severity P0 under the code-review rules; a dirty-source rebuild can also report a stale source revision.

## Assurance Context

No applicable `AssuranceProfile` was found under `spec/`. Repository instructions require `make lint`, `make test`, and `make package-audit`; the dispatch brief explicitly excludes whole suites and package audits. Coder evidence reports compile check, 134 focused CLI tests, rights scan and focused Clippy passed. Direct `cargo fmt --check` reports formatting differences only in unchanged files (`src/approval_rekor.rs`, `src/approval_tag_host.rs`, `tests/approval_verify_gitsign.rs`); changed files pass the direct formatting check by diff inspection. No full suite, package audit, mutation run or manual workflow was run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Four CLI test targets duplicate the same `assert_producer` helper. | tests/signature_verify_cli.rs:247 |
| FND-002 | medium | Build-script rerun inputs omit tracked source files, so a dirty rebuild can retain a clean-tree commit identity. | build.rs:9 |

## Review Scope

- {id: FR-036-AC-16, path: "spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md", role: examined, excerpt: "Every verdict for both commit and tag requests, including `unsigned` and `unverifiable`, carries the running package version, the build-embedded Engineering Assurance source commit or `null` when unavailable, and one UTC `generated_at` value with source `host_clock`; all verdicts produced by one invocation share that value, which is not used as signing or trusted time."}
- {id: FR-041-AC-37, path: "spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md", role: examined, excerpt: "Every `approval-verify` verdict, including one whose trust inputs cannot be read and one whose status is not `approved`, carries the running `ea_version`, the build's `ea_source_revision` (or `null` when absent), and a UTC host-clock `generated_at` sampled once for the invocation with source `host_clock`; when `trust` is present, top-level `ea_version` equals `trust.ea_version`; the nested `signature` omits duplicate producer fields, and no producer field is presented as signing or trusted time."}
- {id: FR-043-AC-26, path: "spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md", role: examined, excerpt: "The trail result and every entry carry `ea_version`, `ea_source_revision`, `generated_at` and `generated_at_source`; all entries share the trail's `generated_at`, including an empty or incomplete trail. When result `trust` is present, top-level `ea_version` equals `trust.ea_version`; `trust` carries `allowed_signers_sha256`, `ea_version` and `git_version`, and `allowed_signers_commit` when the request's `allowed_signers` is a `<rev>:<path>` that resolved."}
- {id: FR-047-AC-17, path: "spec/functional/FR-047-release-record-and-signed-release-tag.md", role: examined, excerpt: "Every `released` or `not_released` verdict, including a verdict without `approval`, carries the executable's `ea_version`, build-embedded `ea_source_revision` (or `null` when unavailable), and one UTC `generated_at` value with source `host_clock`; when `trust` is present top-level `ea_version` equals `trust.ea_version`, and when `approval` is present its producer fields are identical; none of these fields is treated as signing or trusted time."}
- {id: build.rs::main, path: "build.rs", role: examined, excerpt: "cargo:rerun-if-changed watches only the git HEAD and branch ref paths; the build script also derives EA_SOURCE_REVISION from the working-tree cleanliness check."}
- {id: ProducerIdentity::sample, path: "src/producer_identity.rs", role: examined, excerpt: "Samples the producer build identity and UTC host clock once; serializes package version, optional source revision, generated_at and host_clock source."}
- {id: result construction, path: "src/signature_verify_host.rs; src/approval_verify_host.rs; src/approval_trail_host.rs; src/release_host.rs", role: examined, excerpt: "Each command samples producer identity at execute entry and carries it through verdict, trail-entry and nested approval result construction."}
- {id: test helper duplication, path: "tests/signature_verify_cli.rs; tests/approval_verify_cli.rs; tests/approval_trail_cli.rs; tests/release_verify_cli.rs", role: examined, excerpt: "Each file defines an equivalent assert_producer helper that compares the same fields and checks the same timestamp shape."}
- {id: CLI producer tests, path: "tests/signature_verify_cli.rs; tests/approval_verify_cli.rs; tests/approval_trail_cli.rs; tests/release_verify_cli.rs", role: examined, excerpt: "The new traced tests exercise unsigned/unverifiable, no-trust, empty-trail and not-released results; shared result helpers assert producer fields on other exercised cases."}
- {id: SECURITY.md policy route, path: "SECURITY.md", role: examined, excerpt: "Use the repository Security tab and choose Report a vulnerability."}

## Findings Detail

### FND-001 — high

The same `assert_producer` implementation is added in all four changed CLI test files. I searched the changed test targets and `tests/support/`; no pre-existing shared producer assertion helper is present. The copies assert identical version, source-revision, source-label and timestamp-format conditions, so the behavior has four owners and can drift. Consolidate the assertion in one shared test support module and import it from each target.

### FND-002 — medium

`build.rs` emits `rerun-if-changed` only for Git's `HEAD` and its current branch ref, then computes cleanliness with `git status`. After a clean build, editing a tracked source file without changing `HEAD` does not invalidate those watched paths. Cargo can reuse the previous build-script output while compiling the dirty source, leaving `EA_SOURCE_REVISION` set to the clean commit instead of empty. That makes the emitted producer identity inaccurate on this rebuild path.

## Verification Notes

- `quoin matrix --repo . --json`: all four new ACs are statically tagged.
- `cargo fmt --check`: failed on formatting differences in three unchanged files; no changed file was among those diffs.
- Coder-reported focused evidence: compile check passed; four CLI targets 134/134; focused Clippy passed with six baseline lint classes allowed; content-rights-tree passed with 841 entries and no findings.
- Strict Clippy was reported as exit 101 with six warnings attributed to existing lint classes, including an unchanged warning site in `src/approval_verify_host.rs`; changed lines add producer threading only. No full Clippy/test/package suite was run by this reviewer.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The repeated assertions now use the single helper in `tests/support/producer_identity.rs`. |
| FND-002 | fixed | `build.rs` now watches every currently tracked path in addition to Git HEAD and its ref. |
