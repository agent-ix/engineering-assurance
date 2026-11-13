---
id: SR-16424
title: "gap-analysis of the document converter producer requests (engineering-assurance PR 222)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@406604ec; spec/functional/FR-044-document-converter-producer-requests.md; spec/tests.md; tests/document_converter_producer_cli.rs; Makefile"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-045
    type: references
---

# Gap analysis of the document converter producer requests (engineering-assurance PR 222)

## Summary

Ticket: PLAT-1602. Plan completion: not assessed, because the analysis was planless.

The tool was `quoin matrix --json` (quoin 0.30.1). Every FR-044 criterion is tagged:
- AC-1 to AC-5 and CON-1 are bound to non-ignored tests: TC-368 to TC-373 and TC-376.
- AC-6 is `tagged-by-ignored-test`, through TC-374 and TC-375.

No production code is added, so there is no unowned code.

The ticket ACs map as follows:

| Ticket AC | Coverage |
| --- | --- |
| 1. Runs through `producer-execute` with no new execution path | Met. Every test drives the CLI binary, and the diff has no `src/` change. |
| 2. Fixture converts to PDF and DOCX | Specified and tested, but not yet executed. See FND-001. |
| 3. Missing converter named; failure non-zero with stderr | The state half is met (TC-370 to TC-372). The naming half is the spec-review finding in SR-16425 FND-001. |
| 4. Nothing vendored | Met. The repository tree holds no converter binary, template or reference document. The stand-ins are generated in a tempdir. |

The same pattern as the Syft producer was checked against the cross-repo contract, quire-cli `origin/main` at dc1876e. The request fields, arguments, environment, budget and response in the test builder equal the FR-032 field table. The two-request PDF flow over one capability root matches FR-032 A. The `unavailable` and `failed` states map to the FR-033 missing-converter and converter-failure rows. NFR-010 is not violated: there is no converter other than pandoc and typst, and no launch site outside producer-execute.

The two `#[ignore]` real-converter tests are acceptable and well specified, for these reasons:
- FR-044-CON-1 and TC-376 make `make converter-integration` fail, naming the variable, when a tool is absent. The tests also panic rather than skip.
- `spec/tests.md` honestly marks TC-374 and TC-375 as 🚧 "NOT yet executed".
- This matches the accepted FR-038 Syft precedent.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Ticket AC 2 ("a fixture Markdown document converts to PDF and to DOCX") and FR-044-AC-6 have no execution evidence. TC-374 and TC-375 have never run against real pandoc and typst, so nobody has observed real converters working inside the executor: `--sandbox`, `HOME` under `out/home`, the typst font lookup under `process-group-v1`, and the `maxDescendants` 64 limit. The PR can merge as specified. The ticket AC must stay unchecked until `make converter-integration` has passed once on a host with both tools, and the TC-374 and TC-375 status has been updated. | tests/document_converter_producer_cli.rs:354-388; spec/tests.md:605-606 |

## Verdict

Trace coverage is complete, and the stand-in tests pass. One medium gap remains: the real-converter run has not been executed. It does not block the merge, but it blocks ticking ticket AC 2.

## Dispositions

Round 1, at head 4be67958. "FR-044" in this file means the converter FR, now FR-045 after renumber 4be6795. The relationship target was repointed to FR-045. `quoin matrix` still tags FR-045-AC-1 to AC-5 and CON-1, and AC-6 is `tagged-by-ignored-test`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | The fix round did not execute TC-374 or TC-375, and none is available here because pandoc and typst are not installed on this host. As stated in the review, this does not block the merge. Ticket PLAT-1602 AC 2 must stay unchecked, and the ticket must not be closed, until `make converter-integration` has passed once on a host with both tools. |
