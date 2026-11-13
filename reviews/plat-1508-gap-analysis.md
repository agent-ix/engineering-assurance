---
id: SR-16414
title: "gap-analysis of FR-040 gitsign verification (engineering-assurance PR 215)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@aac6677; FR-040 (AC-1..AC-19, CON-1, CON-2); spec/tests.md; tests/signature_verify.rs; tests/signature_verify_gitsign.rs; src/signature_verify.rs; src/signature_verify_host.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-040
    type: references
---

# Gap analysis of FR-040 gitsign verification (engineering-assurance PR 215)

## Summary

Ticket: PLAT-1508. Plan completion: not assessed. Coverage was computed with `quoin matrix --repo . --json` (quoin 0.30.1) at head aac6677.

Every Test-method criterion is bound to a `#[trace]` test function, with no run evidence:

| Criterion | Bound tests |
| --- | --- |
| AC-1 | TC-278 |
| AC-2 | TC-279 |
| AC-3 | TC-280 |
| AC-4 | TC-281 |
| AC-5 | TC-282 |
| AC-6 | TC-283 |
| AC-7 | TC-295, TC-284 |
| AC-8 | TC-285 |
| AC-9 | TC-294 |
| AC-10 | TC-293 |
| AC-11 | TC-296, TC-286 |
| AC-12 | TC-287 |
| AC-13 | TC-288 |
| AC-15 | TC-292, ignored live |
| AC-17 | TC-289 |
| AC-18 | TC-290 |
| AC-19 | TC-291 |
| CON-1 | TC-293, TC-278, TC-289 |

Each bound test asserts the criterion and can fail. All of them pass on git 2.34.1.

Inspection criteria:

- AC-14 and CON-2 pass. No dependency was added, `Cargo.lock` is unchanged, and the only programs invoked are git, `ssh-keygen` and `gitsign`.
- AC-16 passes against the gitsign v0.18.0 source for the subcommands, the four flag names and the `.*` wildcard values. The missing end-of-options separator is code-review SR-16413 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-040-AC-15's third clause has no test: `unverifiable` with `gitsign_verification_failed` when the network is unavailable. TC-292 asserts only `signed_valid` and the `issuer` mismatch, and its spec/tests.md row drops the clause. Fix: add a network-less leg to the ignored live test (for example under `unshare -rn` when available), or remove the clause from AC-15. | tests/signature_verify_gitsign.rs:653-685; spec/tests.md:524 |
| FND-002 | high | The FR-040 Behavior bullet prescribes the scoped `git config --get-regexp '^gitsign\.'` check. gitsign reads `git config --get-regexp .*` line by line, so a newline inside another key's value injects a `gitsign.*` key the scoped check never sees. The spec as written cannot meet CON-1's purpose. Fix: amend the bullet and FR-040-AC-17 to mirror gitsign's own read: the unscoped `.*` query, refusing any output line whose first token case-insensitively starts with `gitsign.`. Add an AC-17 case for the injected value. This pairs with SR-16413 FND-002. | spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md |
| FND-003 | medium | FR-040 Inputs defines the tag name as the part after `refs/tags/`, and does not exclude a name that begins with `-`. git accepts such names (`refs/tags/--help`), and gitsign parses one as an option. Fix: state in Inputs that a tag name beginning with `-` is `revision_unresolved`, and that the target is passed after `--`. Extend AC-19 with that case. This pairs with SR-16413 FND-001. | spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md |

## Verdict

The tag binding is complete and honest. Every Test criterion has a real test that can fail. The stand-in limits are declared in the spec, and the one live test is opt-in and fails loudly without its variables.

The gaps are one untested AC-15 clause and two spec holes that SR-16413 reproduced as exploitable. The spec and its tests should be fixed in the same round as the code.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | The amended FR-040 Behavior bullet says a line's first token is matched against `gitsign.` "ASCII case-insensitive". gitsign uses Go `strings.EqualFold`, which also folds U+017F to `s`, so the spec rule still lets a repository inject a key gitsign applies (reproduced, SR-16413 FND-007). Fix: state the match as Go EqualFold, that is Unicode simple case folding, which for `gitsign.` means ASCII case-insensitive with U+017F counted as `s`. Extend AC-17 with that case. | spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-040-AC-15 now states that an unreachable network is covered by the AC-4 stand-in tests, because gitsign reports it as exit 1 with every matcher. That matches the gitsign v0.18.0 source (every `Execute` error exits 1). The live TC-292 asserts the two remaining clauses. |
| FND-002 | fixed | The Behavior bullet now prescribes gitsign's own `git config --get-regexp .*` query, read line by line for a first token beginning `gitsign.`. AC-17 adds the newline-injection case and TC-289 tests it. The case-folding gap left in the new wording is FND-004. |
| FND-003 | fixed | FR-040 Inputs now makes a tag name beginning with `-` `revision_unresolved`, Behavior passes the target after `--`, and AC-19 and AC-2 name both. TC-291 (`refs/tags/--help`) and TC-279 (`--` in argv) test them. |
