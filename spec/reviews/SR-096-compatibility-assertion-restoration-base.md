---
id: SR-096
title: "base review of the restored compatibility and corpus assertions"
type: SpecReview
analysis: base
scope: "spec/tests.md, spec/functional/FR-018-staged-runtime-migration.md; FR-011-AC-7, FR-011-AC-9, FR-012-AC-1..AC-6, FR-012-CON-2, FR-015-AC-1, TC-075, TC-077, TC-079..TC-084, TC-100"
review_set: base
---

## Summary

Base checklist review of the Test Matrix and FR-018 changes that accompany the
restoration of assertions lost when the Python compatibility lane was retired in
#62. The change restores test backing rather than adding requirements: no
acceptance criterion is added, and every row touched already existed. The review
confirms that each row's recorded status now matches what its backing test can
actually catch, which was the defect the restoration addresses.

## Findings

| ID      | Severity | Summary                                                                                                                                                     | Refs                                        | Escape Cause                    |
| ------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- | ------------------------------- |
| FND-001 | high     | TC-075 was recorded as backed by the native corpus index while its only test ran against a synthetic index the test itself wrote, so the cross-language and concept assertions could not fail for any real corpus. Restored against the pinned corpus, and the concept vocabulary and non-empty producer/path rules moved into parse-time refusal. | FR-011-AC-7, TC-075                         | correct-requirement-no-evidence |
| FND-002 | high     | FR-012-CON-2 was recorded as passing with nothing checking it. An agent name in `accepted_by` and a non-date `accepted_at` both opened the gate — the exact failure the constraint exists to prevent. Restored and confirmed by mutation. | FR-012-AC-4, FR-012-CON-2, TC-082           | correct-requirement-no-evidence |
| FND-003 | medium   | TC-083 could pass having hashed nothing: absent artifacts are skipped, and no assertion required a non-empty population. A path rename would have turned the digest gate green and silent. Restored as a floor on the artifacts actually found. | FR-012-AC-5, TC-083                         | correct-requirement-no-evidence |
| FND-004 | medium   | TC-079's "no pin is a branch, latest, or HEAD" and the non-empty `release` rule were unenforced; the matrix validator checked only `released` and non-blank name/version. Moved into parse-time refusal so a bad matrix is rejected rather than merely untested. | FR-012-AC-1, TC-079                         | correct-requirement-no-evidence |
| FND-005 | medium   | TC-077 was downgraded to out-of-scope by #62, which contradicted the FR-018 cutover rule added in that same change. The criterion is restored by invoking the pinned corpus repository's own builder, and FR-018 now states the relocate-don't-delete rule it was missing. | FR-011-AC-9, FR-018, TC-077                 | wrong-requirement               |
| FND-006 | medium   | The TC-100 row claimed no Python is executed as a qualification oracle. That holds for the compatibility slice only; the PGM-01 and report slices still execute the retained Python differentially. The row overstated its own population and now says which part of it holds. | FR-015-AC-1, TC-100                         | wrong-requirement               |
| FND-007 | low      | TC-081 recorded "one unobserved component withholds the gate" while the test dropped one component of four; TC-080 recorded the incompatible vocabulary while the tagged-but-never-published version was observed by nothing. Both restored to the full population. | FR-012-AC-2, FR-012-AC-3, TC-080, TC-081    | correct-requirement-no-evidence |

## Checklist Results

- **ID formats** — no identifier was added, renumbered, or reused. `SR-096` is
  the next unused review ordinal across every remote branch.
- **Duplicates and gaps** — `quire coverage` reports no unbacked row across
  FR-011, FR-012 and FR-015, including FR-011-AC-9, which was unbacked before
  this change.
- **Validation-link integrity** — every TC named by a touched criterion exists
  in the Test Case table, and each of those rows names its criteria in return.
- **Coverage rules** — the six rules hold. The substantive result is narrower
  than "every criterion has a test": every criterion now has a test that fails
  when the property is violated. Two were confirmed by mutation rather than by
  inspection — an agent attribution and a moving version pin, which fail one and
  seven tests respectively.
- **Measurability** — the restored rows name the exact property each test can
  catch rather than asserting that a capability is "covered".
- **Unhappy paths** — the restoration is refusal-first: the matrix validator
  gained four structural refusals, and the corpus index gained a closed concept
  vocabulary and an empty-member refusal, so malformed input fails at parse time
  rather than reaching a test that may or may not look for it.

## Method Note

The three highest-severity findings share one escape cause:
`correct-requirement-no-evidence`. In each case the requirement was right, the
Test Matrix row read green, and the test could not fail. That pattern is what a
green matrix is least able to report on itself, and it is worth recording that
it was found by auditing deleted assertions against their replacements — not by
reading the matrix, which looked healthy throughout.
