---
id: SR-16300
title: "gap-analysis of owned npm platform release exemption (engineering-assurance PR 187)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@1b7ff8c3; spec/non-functional/NFR-005-rust-containment-and-traceability.md; src/language_boundary.rs; src/language_boundary_host.rs; tests/language_boundary_cli.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/NFR-005
    type: references
---

# Gap analysis of the owned npm platform release exemption (engineering-assurance PR 187)

## Summary

Ticket: IR-1008. Incremental and planless, so plan completion was not assessed. `quire matrix` (quire 0.36.2, engine 0.50.2) at head 1b7ff8c3 shows NFR-005-AC-6, AC-7 and AC-8 as `tagged`. The two new tests are bound to AC-6/TC-209 and AC-7/TC-210 (tests/language_boundary_cli.rs:1057 and 1080). The other NFR-005 rows are unchanged from base. All new production code has an owning requirement: `classify_delta`, `owned_release_versions` and `release_version` implement the new NFR-005 section "Existing npm package release transitions", and the host change only routes through `classify_delta`. There are no stubs and no coverage inflation. The new tests assert exact refusing evidence strings and the exact empty-findings acceptance. NFR-005 still validates under `quire validate`. No existing AC text was changed. The remaining gap is that the new normative section has no acceptance criterion of its own (FND-001). Test strength over that section is recorded in SR-16299 FND-001 and FND-002.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The new "Existing npm package release transitions" section adds about a dozen SHALL conditions (base authority, mode, owner URL form, bin, private, registry, family membership, SemVer, lockstep, JSON equality, findings that must still be retained, no publication authority). No acceptance criterion row was added or amended for them. The new tests are tagged to the unchanged AC-6 and AC-7, so the computed matrix cannot show that the exemption's conditions are verified, and an exemption-only regression has no criterion to fail. Fix: add an NFR-005 AC for the sanctioned transition, or extend AC-7's text with the narrow exception, bound to TC-210 (no spec/tests.md edit needed if bound to the existing TC). | spec/non-functional/NFR-005-rust-containment-and-traceability.md:27-55; spec/non-functional/NFR-005-rust-containment-and-traceability.md:79 | missing-requirement |

## Verdict

Not mergeable as is, because of FND-001 (medium). The code has full requirement ownership and real, non-tautological tests. The new normative behaviour needs an acceptance criterion so that the matrix tracks it.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-002 | low | NFR-005 now names TC-210 as the verification for the new AC-9, but the TC-210 row in `spec/tests.md` still lists only NFR-005-AC-7 in its requirement column. The tests.md trace table disagrees with the NFR. `quire matrix` (AC-9 tagged) and TC-118 reconciliation still pass, so nothing fails. The PR's brief excludes shared spec/tests.md edits, so this belongs in a follow-up. | spec/tests.md:432; spec/non-functional/NFR-005-rust-containment-and-traceability.md:81 | missing-requirement |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Head 56bed14d adds NFR-005-AC-9, which covers the sanctioned transition's conditions, verified by TC-210. Four platform-release tests are tagged to AC-9, and `quire matrix` reports NFR-005-AC-9 as `tagged`. |
