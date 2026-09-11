---
id: SR-094
title: "base review of the native compatibility/corpus/fixture-generation port"
type: SpecReview
analysis: base
scope: "spec/functional/FR-014-versioned-rust-boundary.md, spec/functional/FR-015-semantic-and-identity-parity.md, spec/functional/FR-018-staged-runtime-migration.md, spec/tests.md; FR-015-AC-5, TC-069..TC-078, TC-100, TC-103"
review_set: base
---

## Summary

Base checklist review of the specification changes that move compatibility
classification, accepted-corpus access, corpus identity and integrity checking,
bounded root and path handling, and deterministic fixture generation from the
retained Python lane to native Rust, and that record the removal of
`engineering_assurance/compatibility.py`,
`engineering_assurance/compatibility_corpus.py`, and
`engineering_assurance/fixture_codegen.py` under FR-018. ID formats, the six
coverage rules, and validation-link integrity hold; three traceability defects
were found during the review and fixed in the same change.

## Findings

| ID      | Severity | Summary                                                                                                                          | Refs                        | Escape Cause                   |
| ------- | -------- | -------------------------------------------------------------------------------------------------------------------------------- | --------------------------- | ------------------------------ |
| FND-001 | medium   | TC-077 was demoted to out-of-scope in the Test Case table while the FR-011 coverage table still recorded FR-011-AC-9 as passing; the two tables disagreed about the same criterion. Corrected: FR-011-AC-9 now carries the same out-of-scope disposition and names the owning `agent-ix/qa-corpus` submodule builder. | FR-011-AC-9, TC-077         | correct-requirement-no-evidence |
| FND-002 | medium   | TC-100 and TC-103 are the acceptance criteria named by the driving ticket, but neither row mentioned native accepted-corpus reading or the corpus refusal families, so the ported behavior had no evidence row of its own. Corrected: both rows now state native corpus access, the absence of a Python qualification oracle, and the explicit corpus refusal classes. | FR-015-AC-1, FR-015-AC-3, TC-100, TC-103 | correct-requirement-no-evidence |
| FND-003 | low      | FR-015 described accepted-corpus access only in its Inputs list, with no normative Behavior statement and no acceptance criterion, so TC-069..TC-078 had no FR-015 anchor after their Python backing was retired. Corrected: FR-015 gains four Behavior statements and FR-015-AC-5, verified by TC-069..TC-076 and TC-078. | FR-015, FR-015-AC-5, TC-069..TC-078 | missing-requirement            |
| FND-004 | low      | FR-014 required reusable-library behavior to stay free of filesystem access but did not say where corpus filesystem and Git access belongs, leaving the new host module's placement implicit. Corrected: FR-014 now states that corpus and repository filesystem and Git access lives in a binary-side host adapter. | FR-014                      | missing-requirement            |

## Checklist Results

- **ID formats** — every added identifier matches its pattern: `FR-015-AC-5`,
  `SR-094`, `FND-001..FND-004`. No TC identifier was added, renumbered, or
  reused.
- **Duplicates and gaps** — `FR-015-AC-5` is the next unused acceptance-criterion
  ordinal in FR-015. `SR-094` is unused across every remote branch of this
  repository.
- **Validation-link integrity** — every TC referenced by FR-015-AC-5
  (TC-069..TC-076, TC-078) exists in the Test Case table, and each of those rows
  now names FR-015-AC-5 in return.
- **Coverage rules** — every acceptance criterion added has at least one test
  case, every test case touched names at least one criterion, and no criterion
  is claimed as passing without a stated backing. TC-077 is the single recorded
  exception and is marked out of scope rather than claimed.
- **Measurability** — FR-015-AC-5 and the new FR-015 Behavior statements
  enumerate the exact refusal classes (referenced retention, absolute path,
  parent traversal, symlink, non-regular entry, missing entry, duplicate
  identity, unknown corpus version, malformed index, tampered bytes) rather than
  asserting that unsafe input is "handled".
- **Unhappy paths** — the added behavior is refusal-first: nine of the ten named
  conditions are negative cases, and FR-018's removal behavior is gated on the
  native replacement passing at one candidate revision.

## Out of Scope

TC-077 verifies that the committed corpus reproduces from its recorded source
repositories. It is owned by the pinned `agent-ix/qa-corpus` submodule and its
own builder, and it requires checkouts of `quire-contract-ir`, `quire-code-rs`,
and `quoin`. Cross-repository checkouts are excluded from this slice, so TC-077
is recorded as an open disposition rather than claimed as natively backed.
