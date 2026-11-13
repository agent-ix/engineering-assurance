---
id: SR-16416
title: "gap-analysis of FR-041 approval-verify (engineering-assurance PR 217)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@1993257; FR-041 (AC-1..AC-15, CON-1, CON-2); FR-014-AC-2; spec/tests.md; tests/approval_verify.rs; tests/approval_verify_cli.rs; tests/approval_verify_gitsign.rs; tests/package_boundary.rs; src/approval_verify.rs; src/approval_verify_host.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-041
    type: references
---

# Gap analysis of FR-041 approval-verify (engineering-assurance PR 217)

## Summary

Ticket: PLAT-1510. Plan completion: not assessed (planless). The matrix comes from `quoin matrix --repo . --json` (quoin 0.30.1) at head 1993257.

All 13 Test ACs, the Property AC (AC-9) and FR-041-CON-1 have tag-bound test functions (TC-310..TC-329), and all of those tests pass. Each one can fail, for these reasons:
- TC-310 enumerates all 255 status, binding and meaning combinations and asserts that exactly one is `approved`.
- TC-313 catches trimming or case folding in `bind`.
- TC-314 catches Unicode trimming in `trailer_values`.
- The CLI tests run real ed25519-signed tags generated in each test and assert concrete statuses, bindings and members.
- TC-327 proves through a marker shim that no git is started on a refused request.

AC-12 and CON-2 (Inspection) have no binder (`method-without-symbol`), which is honest. I inspected them: `src/approval_verify.rs` holds no signature, certificate or trailer-parsing code and restates no FR-036, FR-037 or FR-040 rule, and the host takes trailer values only from `signature_verify::trailer_values`. This passes.

No production code in the diff lacks an owning requirement. `run_request_line` refactors the shared CLI request handling for FR-036 and FR-041.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-041-CON-1 has no test with trailer bytes that git or the decoder does not read byte-exactly. A signed tag whose trailer holds a NUL or invalid UTF-8 is reported `approved` for a different value (see SR-16415 FND-001). Add CLI tests for a NUL-truncated digest and a non-UTF-8 id, each expecting `unverifiable` / `trailer_unreadable` and never `approved`. | tests/approval_verify_cli.rs:580-608 |
| FND-002 | low | Wrong trace. `tc_314` is tagged `FR-041-AC-10` but tests trailer reading, not request validation; spec/tests.md maps TC-314 to FR-041-AC-3 only. Drop `FR-041-AC-10` from the `#[trace]` of tc_314. | tests/approval_verify.rs:198 |
| FND-003 | low | FR-014-AC-2 ("every machine-facing command emits exactly one declared-version JSON result") is tested by TC-098 over an explicit 13-command list that omits `approval-verify`, so the AC cannot fail for the new command. Add an `approval-verify --request '{not json'` case (array length 14). `make package-audit` is accepted and FR-014 states no command count, so no other boundary change is needed. | tests/package_boundary.rs:59-135 |

## Verdict

Traceability is complete for FR-041 and every bound test can fail. CON-1 has an untested and failing edge (FND-001, which pairs with the high code-review finding). Two low trace and coverage fixes remain. Not mergeable until FND-001 is covered by a test that passes against the fixed code.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit 6600bdf adds tc_330 (TC-330, traced to FR-041-AC-15 and FR-041-CON-1). It hand-signs a NUL-truncated digest and a 0xFF id and expects `unverifiable`/`trailer_unreadable` with no binding and no approver. It fails against the pre-fix code (both cases reproduced `approved` there) and passes now; quoin matrix binds it to AC-15 and CON-1. |
| FND-002 | fixed | Fix commit 6600bdf drops FR-041-AC-10 from the tc_314 trace; quoin matrix binds AC-10 only to tc_315 and tc_327. |
| FND-003 | fixed | Fix commit 6600bdf adds an `approval-verify --request '{not json'` case to TC-098 (14 commands) and updates the spec/tests.md rows; package_boundary passes. |
