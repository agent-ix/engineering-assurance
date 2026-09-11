---
id: SR-097
title: "base review of the verification-semantics retirement and its assertion audit"
type: SpecReview
analysis: base
scope: "spec/tests.md, spec/functional/FR-018-staged-runtime-migration.md; FR-008-AC-1..AC-4, FR-009-AC-1..AC-4, FR-010-AC-1..AC-4, FR-015-AC-1, FR-015-AC-4, NFR-004-AC-1, NFR-004-AC-2, US-005-AC-1..AC-3, StR-002-VC-1, TC-052..TC-068, TC-100, TC-102, TC-103, TC-104"
review_set: base
---

## Summary

Base checklist review of the FR-018 and Test Matrix changes that accompany
retiring `engineering_assurance/verification_semantics.py` and
`tests/test_verification_semantics.py`, and replacing the four differential
`python3` invocations in `tests/semantics_parity.rs` with reference bytes
captured once from the retained implementation.

The change adds no acceptance criterion. Its substance is an assertion audit:
every test in the retired Python suite was matched, assertion by assertion,
against the Rust assertion that would fail if the same property were violated.
The Python suite carried seventeen Test Matrix rows, TC-052..TC-068. Two of
them, TC-052 and TC-061, were also carried by a Rust test; the remaining fifteen
were not, and retiring the suite would have left them green and backed by
nothing. Within those rows, the findings below enumerate the individual
assertions that had no Rust equivalent at all. The rows now state what their
backing tests can actually catch, and FR-018 states the retirement rule this
capability was missing.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Fifteen rows — TC-053..TC-060 and TC-062..TC-068 — recorded criteria whose only tracking-tagged backing lived in `tests/test_verification_semantics.py`. Deleting that file would have left every one of them green and backed by nothing. Each criterion is now carried by a tracking-tagged Rust test. | US-005-AC-1..AC-3, FR-008-AC-1..AC-4, FR-009-AC-1..AC-4, FR-010-AC-1..AC-4, NFR-004-AC-1, NFR-004-AC-2, TC-053..TC-068 | correct-requirement-no-evidence |
| FND-002 | high | TC-061 was recorded as backed by a Rust loop over the declared non-success states. The loop asserts nothing about the population it iterates, so an empty or truncated state file passes it vacuously — the exact shape #62 lost. The Python test guarded the population by naming three states it must contain; that guard is restored, together with a floor on the state count. | FR-009-AC-2, TC-061 | correct-requirement-no-evidence |
| FND-003 | high | TC-100's PGM-01 and report slices asserted only that Rust agrees with a Python process. With the Python deleted there is no assertion left, and the recorded values — the lossy outcome, the preserved legacy identities, the seven bounded report sections, the absence of a trust score or overall verdict in the rendered Markdown — were asserted nowhere else. Reference bytes are now committed and the field-level assertions are restored natively. | FR-010-AC-2, FR-010-AC-4, FR-015-AC-1, TC-065, TC-067, TC-100 | correct-requirement-no-evidence |
| FND-004 | medium | TC-066 recorded the refusal population as ambiguous, unreadable, malformed, stale and tampered. Two refusals the Python exercised had no Rust assertion: an invalid caller-supplied expected digest, and a retained-stream byte count that is a string rather than an integer. Both are restored, and the row now names them. | FR-010-AC-3, TC-066 | correct-requirement-no-evidence |
| FND-005 | medium | TC-058's refusal family was narrower in Rust than the criterion states. A reference that links to itself and a reference that omits a required relationship are both refused by the library with distinct typed reasons, and neither was exercised by any test. Both are restored. | FR-008-AC-3, TC-058 | correct-requirement-no-evidence |
| FND-006 | medium | TC-060 was recorded as confirming that the complete producer tuple survives projection. The Rust side only validated a fixture that happens to contain producer tuples; nothing asserted that the fixture's producer-owned population is non-empty, that the tuple carries exactly its six declared fields, or that an absent field or an empty environment refuses. Restored with a population floor. | FR-009-AC-1, TC-060 | correct-requirement-no-evidence |
| FND-007 | medium | TC-053's registry audit exercised three of the four registry refusals. An ownership registry that simply omits a concept — the one failure a shrinking vocabulary produces — was refused by the library and asserted by no test. Restored. | US-005-AC-1, FR-008-AC-1, TC-053 | correct-requirement-no-evidence |
| FND-008 | medium | TC-064 stated that PGM-01 source bytes are unchanged after mapping. The Python test read the fixture before and after the call and compared. No Rust test did, and no Rust test recomputed the recorded digest independently of the code that produced it. Both are restored; the static library-capability audit in TC-059 remains the structural half of the same claim. | FR-010-AC-1, TC-064 | correct-requirement-no-evidence |
| FND-009 | medium | TC-068 and TC-104 audit contract and schema populations by reading a directory and asserting that nothing forbidden appears in the concatenation. An empty or renamed directory passes. Counted population floors were added to both, matching the one the executable-path audit already carried. | NFR-004-AC-1, FR-015-AC-4, TC-068, TC-104 | correct-requirement-no-evidence |
| FND-010 | medium | TC-054 recorded a bounded report with no trust score. Only one of the four forbidden aggregate fields was exercised in Rust; the Python test named all four. The report's deterministic rendering and its unchanged round trip were likewise asserted only through the Python oracle. All are restored. | US-005-AC-2, FR-010-AC-4, TC-054, TC-067 | correct-requirement-no-evidence |
| FND-011 | low | FR-018 named the retirement rule for the content-rights, package-audit and compatibility capabilities but not for verification semantics, so the removal in this change had no requirement to satisfy. The clause is added, including the obligation to carry every criterion the retired tests backed. | FR-018, TC-113 | missing-requirement |
| FND-012 | low | The TC-100 row carried a population qualifier added by #64 because the PGM-01 and report slices still executed Python. That qualifier is now false rather than true, and is removed. | FR-015-AC-1, TC-100 | wrong-requirement |

## Checklist Results

- **ID formats** — no identifier was added, renumbered, or reused. `SR-097` is
  the next unused review ordinal across every remote branch at review time.
- **Duplicates and gaps** — `quire coverage` reports no unbacked row across
  FR-008, FR-009, FR-010 and FR-015 after the retirement.
- **Validation-link integrity** — every TC named by a touched criterion exists
  in the Test Case table, and each of those rows names its criteria in return.
- **Coverage rules** — the six rules hold. The stronger result this review
  claims is narrower: every criterion the retired Python backed is now carried
  by a Rust test that fails when its property is violated, confirmed by
  mutation for the non-obvious ones rather than by reading the matrix.
- **Measurability** — the rewritten rows name the exact property each test can
  catch. Rows that previously read as capability labels ("legacy identity and
  limitations are preserved") now name the fields and the refusals.
- **Unhappy paths** — the restored assertions are refusal-first. Nine typed
  refusals the library already produced and no test exercised are now exercised
  — incomplete ownership concept set, self reference, missing required link,
  incomplete producer tuple, empty producer environment, invalid expected
  digest, non-integer and negative retained byte counts, and duplicate source
  premise — each asserted on its typed reason rather than on diagnostic prose,
  alongside the three forbidden aggregate report fields only one test covered.

## Method Note

The audit was performed against the deleted assertions, not against the Test
Matrix. Every row in scope read green before the audit, and fifteen of them were
green only because a Python test carried them. That is the same escape the two
previous rounds of this campaign produced, and it is not visible from the matrix
side: a row backed by a file that is about to be deleted looks identical to a
row backed by a file that is staying.

A second recurring shape appeared once more. TC-061's state loop and the TC-068
and TC-104 directory audits all pass over an empty population. None of the three
would have reported anything if the data they read had disappeared.
