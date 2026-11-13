---
id: SR-16418
title: "gap-analysis of FR-041 signing time (engineering-assurance PR 218)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@fbb6e2f; FR-041 (AC-16..AC-21, CON-3); spec/tests.md TC-331..TC-337; tests/approval_verify.rs; tests/approval_verify_cli.rs; tests/approval_verify_gitsign.rs; src/approval_verify.rs; src/signature_verify.rs; src/signature_verify_host.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-041
    type: references
---

# Gap analysis of FR-041 signing time (engineering-assurance PR 218)

## Summary

Ticket: PLAT-1592. Plan completion: not assessed (planless). The matrix comes from `quoin matrix --repo . --json` (quoin 0.30.1) at head fbb6e2f.

AC-16..AC-21 are each `tagged`. FR-041-CON-3 is `untagged`. Evidence status for all of them is "no run evidence" (the matrix carries no run records). The bound tests were run and all pass, and they can fail:
- TC-331 compares exact UTC strings, including epoch 0, 2000-02-29 and 9999-12-31T23:59:59Z, plus 13 rejected lines.
- TC-334 creates real SSH-signed tags with `GIT_COMMITTER_DATE` at +0530 and -0800 (across a UTC day boundary) and asserts the UTC `signed_at`. It also asserts that `signature-verify` has no time member and equals the embedded `signature`.
- TC-335 checks that three non-approved statuses carry none of the three members.
- TC-336 and TC-337 run gitsign stand-in approvals, with a time, with no tagger line and with an unparseable tagger line.

No production code without an owning requirement was found: `tagger_signed_at`, `format_utc` and `SigningTime` are owned by FR-041 Behavior.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-041-CON-3 is untagged in the computed matrix, but spec/tests.md maps TC-332 and TC-337 to it. The `#[trace]` attributes on `tc_332_...` (AC-17 only) and `tc_337_...` (AC-18 only) omit `FR-041-CON-3`, so the safety constraint has no bound test. Fix: add `"FR-041-CON-3"` to both trace attributes. | tests/approval_verify.rs:295; tests/approval_verify_gitsign.rs:284; spec/tests.md:563,568 |
| FND-002 | medium | No assertion backs the AC-20 clause "the time read starts no `gitsign` run". The stand-in records nothing, and TC-336 checks only the status and time, so an implementation that ran `gitsign` a second time to get the time would still pass. Fix: have the stand-in append one line per invocation to a log file in its directory, and assert in TC-336 that one approval request ran it exactly as many times as the same request run through `signature-verify` does. | tests/approval_verify_gitsign.rs:28-42,261-279 |
| FND-003 | low | The AC-17 "non-UTF-8 text yields no time" case is vacuous. `tagger_signed_at(b"\xff\xfe")` has no `tagger` header at all, so it returns `None` whether or not the UTF-8 check exists. Fix: add a header tagger line whose text after `>` is not UTF-8, for example `tagger A <a@x.test> 17\xff0000000 +0000`, and assert `None`. | tests/approval_verify.rs:317 |
| FND-004 | low | Trace gap: TC-310 asserts the AC-19 clause that supplying a time to the pure decision changes no status or reason for any combination, but it is traced only to FR-041-AC-9 and FR-041-CON-1. Neither its trace tag nor its spec/tests.md row names AC-19, and TC-333 (which is mapped to AC-19) checks only one wrong-record case. Fix: add `"FR-041-AC-19"` to TC-310's trace and row. | tests/approval_verify.rs:77,103-110; spec/tests.md:541 |

## Verdict

AC-16..AC-21 have real, failing-capable tests that pass, and code ownership is complete. Gaps: CON-3 is unbound (FND-001), the AC-20 "no gitsign run" clause has no assertion (FND-002), and two binding weaknesses are low (FND-003, FND-004). Not clean. All four belong in the same fix round.
