---
id: SR-16421
title: "gap-analysis of FR-043 approval-trail (engineering-assurance PR 220)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@be9a4c9; FR-043; spec/tests.md; tests/approval_trail.rs; tests/approval_trail_cli.rs; tests/approval_trail_gitsign.rs; src/approval_trail.rs; src/approval_trail_host.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-043
    type: references
---

# Gap analysis of FR-043 approval-trail (engineering-assurance PR 220)

## Summary

Ticket: PLAT-1594. Plan completion: not assessed. `quoin matrix --repo . --json` (quoin 0.30.1) binds every Test criterion of FR-043 to at least one `#[trace]`-tagged test: AC-1 to AC-12, CON-1 and CON-3 are `tagged`. AC-13 and CON-2 are Inspection, `method-without-symbol`. All 13 FR-043 tests pass on git 2.34.1. spec/tests.md marks TC-351 to TC-362 as 🚧, so the PR makes no hand-written passing claim for them.

I inspected CON-2 and it holds: the adapter has no signature, certificate, trailer-parsing or decision code of its own. AC-13 holds only as presence. The adapter has the 300 s constant and the deadline check, and the shared host limits of 60 s and 8 MiB apply. Nothing writes to the repository; the only writes are temporary directories outside it, from the reused FR-041 code. The time-limit behaviour itself is never exercised.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The TC-358 canary cannot fail. The test asserts that `repo/pwned` does not exist, but none of the hostile names would create it even if a shell ran them: `v1;touch;pwned` runs `touch` with no operand, and `$(touch,pwned)` and `` a`touch,pwned` `` run a command literally named `touch,pwned`. The "executed nothing" half of AC-7 is therefore unchecked. Fix: use a ref-legal payload that does create the file under a shell, for example `a$(touch${IFS}pwned)` and `` b`touch${IFS}pwned` ``, and assert the file is absent. | tests/approval_trail_cli.rs:468-495 |
| FND-002 | medium | The `time_limit` path (`truncated` true, `truncation` `time_limit`, and `considered`) has no test: AC-13 only inspects that the bound is present. This behaviour can be tested, and an untested bug sits in it (SR-16420 FND-003, the overstated `considered`). Fix: pass the deadline into an inner function and add a test with an already-expired deadline that asserts `time_limit`, no entries, and `considered` 0. Then narrow AC-13 to the bound's value. | src/approval_trail_host.rs:156-167; spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md:164 |
| FND-003 | medium | The Behavior "SHALL count in `skipped` an annotated tag whose name is not valid UTF-8" has no AC and no test. A probe confirms it works today, but nothing would catch a regression. Fix: add an AC and extend TC-359 with an annotated tag whose ref name holds a byte 0xE9. | spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md:126-127; src/signature_verify_host.rs:287-290 |
| FND-004 | low | AC-8 says "a validly signed tag" whose message holds a NUL byte, but TC-359 uses an unsigned forged tag (`forged_tag`), so the test does not show what the AC states. Fix: either sign the NUL tag object (sign the payload with `ssh-keygen -Y sign` and append the signature), or reword AC-8 to "an annotated tag". | tests/approval_trail_cli.rs:509-521; spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md:159 |

## Verdict

The binding coverage is complete and the tests are real. They are process-boundary tests on real repositories, signed with throwaway ed25519 keys generated at test time and fictional identities. TC-355 is a strong CON-1 check: it compares each entry with the `approval-verify` output byte for byte, less the echo members. TC-351 exercises the pure ordering in both input orders.

The gaps are FND-001, a vacuous canary, and FND-002 and FND-003, specified behaviour with no test. Fix all three in the fix round. The public-repo hygiene is clean: fixtures use example.test and example.invalid identities, nothing is committed to fixtures, and there are no workstation paths.
