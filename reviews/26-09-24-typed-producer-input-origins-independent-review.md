---
id: SR-131
title: "Independent review of typed producer input origins"
type: SpecReview
analysis: code-review
scope: "EA draft PR #135, PLAT-1061 at f11fa94; initial 854c5ad review against 4e9a5de"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/TM-001"
    type: references
---

## Summary

This scoped `/code-review`, `/rust-review`, and `/gap-analysis` pass inspected
the authored input-origin values, generated FCD targets, EA validation and
resolution, candidate matrix, and tests. At `f11fa94`, all four findings from
the initial `854c5ad` review are closed within EA's structural boundary.
Quoin retains the separate responsibility to verify source and dependency
bytes independently at replay. No product code was edited for this review.

## Verdict

**CONDITIONAL** at `f11fa94`: the reviewed EA structural gaps are closed, while
the 0.5 matrix awaits human acceptance and Quoin's independent byte replay is
not established by the EA test suite. The initial `854c5ad` verdict was FAIL;
its findings and failing reproductions are preserved below as review history.

## Findings

The table records the initial `854c5ad` findings; the remediation recheck
below records their closure at `f11fa94`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `source_file` checks only field shape. A procedure may declare role `fixture` from `fictional/source:tracked/different.txt` while its selected `InputBinding` is arbitrary `input.txt` bytes; `resolve_procedure` still returns `Ok`. Bind the claimed repository/path to the exact selected file and campaign source graph before minting the request identity. | `src/campaign.rs:385`, `src/campaign.rs:651`; EN-006, VO-012 |
| FND-002 | high | `dependency` checks only nonempty strings. A procedure can name nonexistent member `ghost` and artifact role `report`; `validate_definition` still returns `Ok`, as does request resolution with unrelated input bytes. Check the declared dependency edge and upstream output role, and bind selected retained dependency bytes and attempt identity at resolution. | `src/campaign.rs:426`, `src/campaign.rs:533`; EN-006, VO-012 |
| FND-003 | medium | A present `inputOrigins` inventory covers only `procedure.inputs`, while a dynamic input admitted by `inputRolePrefixes` needs no origin. A required `dependency/` prefix can therefore select arbitrary bytes with no dependency declaration. The migration note explicitly limits coverage to explicit roles; determine whether dynamic producer inputs are in PLAT-1061 scope and state that boundary in the normative contract. | `src/campaign.rs:389`, `src/campaign.rs:855`; `engineering_assurance_v05/generated/campaign/GENERATION.md` |
| FND-004 | medium | The new origin tests sit inside TC-177 but carry no `EN-006` or `VO-012` tracking tag, and the TC-177 matrix row still describes only unknown fields, argument kinds, bounds, and roles. The matrix therefore records no test obligation for provenance binding, dependency membership, or migration behavior. Add tracked cases and matrix rows matching the new contract. | `tests/campaign.rs:148`, `spec/tests.md:397`; EN-006, VO-012 |

## Initial coverage and gates at 854c5ad

- `cargo test --locked --features campaign --test campaign`: seven of seven committed tests pass on macOS with Rust 1.98.1.
- Isolated tests in `/private/tmp/ea-origin-review-repro` fail: a mismatched `source_file` path is accepted by `resolve_procedure`, and an unknown `dependencyMember` is accepted by `validate_definition`.
- `cargo fmt --check` passes.
- The candidate matrix lists 35 EA artifacts; every listed SHA-256 matches the current file. Acceptance remains `pending_human_acceptance`.
- The generated Rust, TypeScript, JSON Schema, and Python surfaces contain the three origin-kind variants and the optional `MeasurementProcedure.inputOrigins` field. This review did not rerun FCD generation byte for byte or Linux executor tests.
- No campaign plan bundle exists, so task-completion verification is unavailable. Existing TC-177 through TC-183 tags are present, but the new origin obligations are not represented in the matrix. Optional gap-analysis semantic review was not run as a separate stage; the code review above checks the claimed behavior directly.

## Remediation recheck — 2026-09-24

The historical FAIL verdict above applies to `854c5ad`. I independently
rechecked commits `49a8a7e` and `f11fa94` against each finding:

- **FND-001 closed for EA's declared boundary.** A source origin now names a repository in the campaign source graph, and `resolve_procedure` requires an exact role-by-role match between authored and caller-selected origin records. EA treats the selected record as a claim. It does not prove its bytes came from the tracked path; the candidate contract assigns that independent byte comparison to Quoin at replay.
- **FND-002 closed for campaign graph and role validation.** A dependency origin must name a direct `dependsOn` member. The new second pass resolves its artifact role against the upstream procedure's fixed output roles or a component-safe child role under a declared output tree. An undeclared `output` role is refused when the upstream procedure only declares `report`; `mutants/case.txt` is admitted under a declared tree and `mutants/../escape` is refused. Runtime origin records must match the authored records. Quoin still owns the retained attempt and byte comparison.
- **FND-003 closed by an explicit migration boundary and guard.** The generation note says opted-in procedures cannot add dynamic prefix inputs without an exact authored role and origin. The updated TC-184 negative declares the `dependency/` prefix, then verifies the origin guard rejects the extra selected input. Implicit inputs from the verified Git source-tree projection remain a separate path.
- **FND-004 closed.** `spec/tests.md` now has TC-184 with FR-019-AC-10, VO-012 and EN-006; the new tests carry those tracking tags, and TC-177 also traces the two new objects.

At `f11fa94`, the ten focused campaign tests pass with Rust 1.98.1 and `cargo fmt --check` passes. No remaining EA structural finding was identified in this scoped recheck. The 0.5 matrix remains `pending_human_acceptance`, and Quoin's independent retained-byte replay has not been shown by this EA test suite. **Recheck verdict: CONDITIONAL** until those separate acceptance and replay gates are satisfied.
