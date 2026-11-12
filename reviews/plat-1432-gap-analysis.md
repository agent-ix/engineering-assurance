---
id: SR-16406
title: "gap-analysis of FR-036 signature-verify (engineering-assurance PR 210)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@c58cb1d; FR-036 (AC-1..AC-12, CON-1, CON-2); spec/tests.md; tests/signature_verify.rs; tests/signature_verify_cli.rs; src/signature_verify.rs; src/signature_verify_host.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-036
    type: references
---

# Gap analysis of FR-036 signature-verify (engineering-assurance PR 210)

## Summary

Ticket: PLAT-1432. Plan completion: not assessed. Computed with `quoin matrix --json` (quoin 0.30.1) at head c58cb1d.

Every FR-036 AC and CON-1 has a `#[trace]`-tagged test that asserts real behaviour and can fail. There are no stubs and no coverage inflation. The CLI tests run the built binary against real repositories with ssh-keygen ed25519 keys generated at test time. The computed matrix still reports FR-036-AC-1..AC-11 and CON-1 as `untagged`: the TC-248..TC-259 definition rows were added to the Functional Requirement Coverage table instead of the Test Case Summary. As a check, the 12 rows were moved after TC-247 in a scratch copy and the matrix rerun: every criterion then reads `tagged`. AC-12 and CON-2 are Inspection. A reviewer inspection confirms no crypto crate was added and only git and ssh-keygen are invoked.

The CON-1 tests cover only the case where the classifier and git agree on the signature format. They miss the format-divergence adverse case, which is where code-review FND-001 and FND-002 reproduce `signed_valid` without SSH allowed-signers acceptance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The TC-248..TC-259 definition rows (6 columns) are in the 4-column Functional Requirement Coverage table, not in the Test Case Summary. As a result `quoin matrix` reports all of FR-036-AC-1..AC-11 and FR-036-CON-1 as `untagged` with no binders. Fix: move the 12 rows into the Test Case Summary after TC-247. This was checked in a scratch copy, and all 12 criteria then read `tagged`. | spec/tests.md:239-250 |
| FND-002 | medium | The FR-036-CON-1 tests (TC-248, TC-249, TC-250) never cover the case where git verifies a different signature format than the classifier detected. Two such cases are missing: a tag with a trailing prefix-only PGP header, and a commit with `gpgsig-sha256` SSH plus `gpgsig` PGP. Each needs a repository-local `gpg.program` reporting a good signature, and must assert a status other than `signed_valid`. Without them CON-1 passes while it is violated (see code-review SR-16405 FND-001/FND-002). | tests/signature_verify_cli.rs:253-270 |

## Verdict

AC coverage is real and every bound test can fail, but the traceability as committed is broken (FND-001) and CON-1 lacks its adverse case (FND-002). Not mergeable until both are fixed together with code-review SR-16405.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit 98e28a1 moves TC-248..TC-259 (plus the new TC-260..TC-263) into the Test Case Summary. `quoin matrix --json` at 98e28a1 reports FR-036-AC-1..AC-11 and CON-1 as `tagged`, with CON-1 at 7 binders. |
| FND-002 | fixed | TC-260 and TC-261 add the format-divergence adverse cases, with a lying repo-local `gpg.program`, asserting `unverifiable`/`unsupported_signature_format`. Both are tagged FR-036-CON-1 and pass. |
