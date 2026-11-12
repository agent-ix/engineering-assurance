---
id: SR-16408
title: "gap-analysis of FR-037 signer mapping and Signed-off-meaning (engineering-assurance PR 211)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@cb6e620; FR-037 (AC-1..AC-9, CON-1, CON-2); spec/tests.md; tests/signature_verify.rs; tests/signature_verify_cli.rs; src/signature_verify.rs; src/signature_verify_host.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-037
    type: references
---

# Gap analysis of FR-037 signer mapping and Signed-off-meaning (engineering-assurance PR 211)

## Summary

Ticket: PLAT-1433. Plan completion: not assessed. The coverage was computed with `quoin matrix --repo . --json` (quoin 0.30.1) at head cb6e620.

All 11 FR-037 criteria read `static_status: tagged`, each bound to a `#[trace]` test function:

| Criterion | Bound tests |
| --- | --- |
| AC-1 | TC-273, TC-264 |
| AC-2 | TC-273, TC-265 |
| AC-3 | TC-266 |
| AC-4 | TC-267 |
| AC-5 | TC-274, TC-269 |
| AC-6 | TC-274, TC-270 |
| AC-7 | TC-271 |
| AC-8 | TC-275, TC-272 |
| AC-9 | TC-268 |
| CON-1 | TC-273, TC-264, TC-265 |
| CON-2 | TC-274, TC-270 |

Each bound test asserts concrete output, and each was checked for a way to fail:

- TC-265 sets the committer email to the principal and asserts no principal.
- TC-266 uses a git shim that writes a marker if started, and also checks the exact 1 MiB boundary.
- TC-268 covers both commit and tag with an expired `valid-before`.
- TC-272 would read `missing` if the signature block were not stripped. It also tags a commit carrying a different trailer value.
- TC-273 drives `G` with no principal and a rejected tag lookup to `unverifiable`.

No stubs and no coverage inflation. The spec/tests.md rows TC-264..TC-275 are in the Test Case Summary and match the bound functions.

The FR-037 behaviour statements that are wrong in the code have no test. Their tests are requested in code-review SR-16407:

- FND-001: the FIFO case for "not a readable regular file".
- FND-002: the `---` divider.
- FND-003: repository-local trailer config.

They are not repeated here as separate gap findings.

`quoin:spec-review` sub-analyses were not run: the only spec change is test-case rows in spec/tests.md, and no requirement statement changed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. Every FR-037 AC and CON has a real, tag-bound test that can fail. The missing adverse-case tests are tracked with their code fixes in SR-16407.
