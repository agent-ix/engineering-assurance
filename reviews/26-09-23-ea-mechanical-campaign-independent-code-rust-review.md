---
id: SR-129
title: "Independent code and Rust review of mechanical campaign candidate"
type: SpecReview
analysis: code-review
scope: "EA PR #135 at 951c397; handwritten campaign resolver, source projection, executor, and generated FCD contract"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/TM-001"
    type: references
---

## Summary

Independent Sol review of the candidate against FR-019 and the campaign value
objects. The generated records carry FCD provenance at de00ad3; this review
inspected their provenance and use, not a fresh byte-for-byte regeneration.
The candidate 0.5 matrix is unaccepted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-353 | high | Required dynamic input prefixes are checked against explicit inputs before verified source-tree inputs are appended. A procedure declaring required `source/` with only a selected Git source tree returns `inputRolePrefixes.required`, even though the verified tree supplies `source/<path>`; the source-tree route cannot satisfy its own required declaration. Validate after combining inputs and keep the source-role authority explicit. | `src/campaign.rs:577`, `src/campaign.rs:600`, `src/campaign.rs:713`; FR-019-AC-10, VO-001, VO-011 |
| FND-354 | low | Omitted tracked symlink targets are verified during resolution but have no executor preflight recheck. Replacing a tracked link after resolution leaves the minted request valid. Since omitted links never reach the staged tree, this does not change executed bytes or create a false process observation; it limits the source-link provenance claim to resolution time. State that timing explicitly or recheck at launch if launch-time provenance is required. | `src/producer_execution.rs:2734`, `src/producer_execution.rs:2810`; FR-019-AC-11, VO-005 |
| FND-355 | medium | `verify_source_file` and `verify_source_link` use path-based metadata checks followed by path-based opens/reads. A concurrent replacement of a parent directory with a symlink after the check can make the resolver read outside the capability root. Git OID verification and the executor's later `openat2` prevent that read from silently staging different content; the issue is the resolver's own no-following source boundary. Use descriptor-relative opens with no-symlink resolution for both file and link verification. | `src/producer_execution.rs:2811`, `src/producer_execution.rs:2861`; FR-019-AC-11, VO-005 |

## Review coverage and gates

- Reviewed the PR diff, FR-019, campaign value objects, TC-177 through TC-183, source projection, and generated provenance. No CI workflow changed.
- The seven focused EA campaign tests pass with Rust 1.98.1.
- A minimal source-only required-prefix test was run in an isolated Git archive at `/private/tmp/ea-review-repro` and failed: one verified `Cargo.toml` source file yields `source/Cargo.toml`, but `resolve_procedure` returns `inputRolePrefixes.required` before adding it. The source worktree was not modified.
- The tests do not cover a source-only required prefix, a link changed between resolution and execution, or a parent-directory replacement during source verification.
- Full `make lint`, `make test`, `make package-audit`, generation replay, and Linux executor gates were not run in this scoped review. The local host is macOS.
- The 0.5 compatibility matrix remains a candidate; this review does not accept it.

## Disposition

**FAIL** for the reviewed candidate because FND-353 blocks an authored source-tree procedure.

## Remediation recheck — 2026-09-23

The follow-up working-tree diff resolves the three findings above:

- **FND-353 closed.** `resolve_procedure` now extends verified source inputs before checking required input prefixes. TC-182 contains a source-only `source/` regression assertion. All seven campaign tests pass with Rust 1.98.1 on macOS.
- **FND-354 closed as a provenance clarification.** `SourceTreeBinding` and VO-005 explicitly state that omitted links are checked at resolution and do not make a launch-time claim. The links remain absent from the staged execution tree.
- **FND-355 closed by code inspection and a path-swap regression.** The resolver now opens root and parent directory components through no-follow descriptors, reads a link with `readlinkat`, and opens a regular file with `openat(O_NOFOLLOW)` from the retained parent. TC-182 refuses a substituted parent symlink. A concurrent-race harness was not run.

`cargo fmt --check` and the seven focused campaign tests pass. The 0.5 matrix's 33 EA artifact digests match the current files; its acceptance state remains `pending_human_acceptance`. The generated targets name FCD commit `9ef44b34dc80c825f625e04e5264d6c0c0952e31`. Fresh byte-for-byte generation and Linux executor tests remain outside this scoped recheck.

**Recheck verdict: PASS for the three reviewed findings**, subject to the candidate matrix's separate acceptance gate.
