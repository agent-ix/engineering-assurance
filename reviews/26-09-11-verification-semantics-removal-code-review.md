---
id: SR-098
title: "Rust review of the verification-semantics retirement"
type: SpecReview
analysis: code-review
scope: "FR-008, FR-009, FR-010, FR-015, NFR-004, US-005, StR-002; TC-052..TC-068, TC-100..TC-104; tests/semantics_contract.rs, tests/semantics_parity.rs, tests/fixtures/semantics-*.json, src/semantics/fixtures.rs; removal of engineering_assurance/verification_semantics.py and tests/test_verification_semantics.py"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---

## Summary

Applied `agent-skills/rust-review` to the cutover that removes the Python
verification-semantics lane and replaces the four differential `python3`
invocations in `tests/semantics_parity.rs` with reference bytes captured once
from the retained implementation.

The central claim was checked rather than accepted. The retained
implementation was restored from `bd6d2ce` into a scratch directory, run
against the same governed fixtures and the same five adverse inputs, and its
output diffed against the committed references: all four reproduce byte for
byte. The references are therefore genuinely derived from the retired lane and
not Rust output rebadged as an oracle, which was the one substitution that
would have made the whole cutover circular.

Eleven findings were raised. All are closed in this change. The highest and the
most instructive share one shape: the assertion moved to a test that did not
carry the tag, while the matrix row was simultaneously widened to claim the
moved work — the same escape this campaign has now produced three times.

## Verdict

**PASS** — no finding remains open; every one below was closed before this
change landed, and the non-obvious closures were confirmed by mutation rather
than by inspection.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The test carrying TC-066 / FR-010-AC-3 asserted only the invalid expected digest and the non-integer byte counts. Unreadable, unknown-version, malformed, stale and tampered — which the row was widened to claim in this same change — were asserted only under FR-015 tags. Closed: the adverse-outcome test now carries TC-066 / FR-010-AC-3. | tests/semantics_parity.rs:256; spec/tests.md TC-066 | correct-requirement-no-evidence |
| FND-002 | medium | Same shape on TC-067 / FR-010-AC-4: the row claimed a comparison against captured reference bytes while the only test carrying the tag did no byte comparison. Closed: the reference comparison now carries TC-067. | tests/semantics_parity.rs:410 | correct-requirement-no-evidence |
| FND-003 | medium | Same shape on TC-053, TC-058 and TC-060: the rows named four registry refusals, an absent link target, and an absent producer tuple, while the tests carrying those tags exercised one, none, and none of them respectively. Closed: each case is now exercised under its own tag. | tests/semantics_contract.rs:67,178,210 | correct-requirement-no-evidence |
| FND-004 | medium | An assertion that could not fail. The stale-reference guard compared a recorded digest to one the mapper had just produced, after the preceding assertion had already compared the entire produced view to the same reference. Closed: the digest is now recomputed from the fixture bytes on disk and checked first, so it diagnoses a stale reference instead of restating a passing comparison. | tests/semantics_parity.rs:213 | correct-requirement-no-evidence |
| FND-005 | medium | The packaged JSON Schemas were enforced by nothing. The retired Python validated every instance against `semantic-reference-v1`, `verification-semantics-fixture-v1`, `verification-semantics-ownership-v1`, `assurance-report-projection-v1` and `pgm01-compatibility-view-v1` before applying any semantic rule; the Rust port replaced that with typed decoding. Those schema files are still shipped by `setup.cfg` and still named in `compatibility-matrix.json`, so they remain the published description of these records with nothing holding them to it. Closed: a conformance test now checks every governed fixture and four produced views against them. | engineering_assurance/schemas/, tests/semantics_contract.rs | correct-requirement-no-evidence |
| FND-006 | medium | The audit backing FR-015-CON-2 read exactly one file. `scripts/` does not exist, the `.github/workflows` walk was not recursive, and the Makefile — this repository's documented entry point, and where such a call would most plausibly be added — was never opened. The population floor of one was satisfied and meant nothing. Closed: the roots are walked recursively, the four repository-root configuration files are read, and the floor is five. | tests/semantics_parity.rs:469 | correct-requirement-no-evidence |
| FND-007 | medium | A tautology in the generated-fixture key-order test, with a comment claiming the opposite. The cases were decoded into a `BTreeMap` and their keys compared against their own sort, which holds whatever the emitted bytes said. Closed: the keys are now read out of the emitted text in emission order. | src/semantics/fixtures.rs:338 | correct-requirement-no-evidence |
| FND-008 | medium | `validate_report_bytes` is the published entry point for encoded reports and was reached by no test; every test decoded with serde directly and so never met the refusal wrapper an external caller meets. The renderer's "No X declared." branches, including "No decision recorded.", were likewise unreachable from the only report fixture, which populates every field. That branch is where an empty report would silently render as a report with no objections. Closed: both are exercised. | src/semantics/report.rs:227; tests/semantics_contract.rs | correct-requirement-no-evidence |
| FND-009 | low | `#[trace("TC-100", "FR-015-AC-1")]` was duplicated verbatim on three tests, an artifact of how the file was rewritten. Closed. | tests/semantics_parity.rs:209,343,375 | correct-requirement-no-evidence |
| FND-010 | low | The cross-language state comparison agreed over an empty population: an emptied state file makes all three extracted lists empty, and three empty lists agree. The floor preventing it lived in a different test binary, which is a coincidence rather than an invariant of this one. Closed: the floor is repeated locally. | tests/semantics_parity.rs:590 | correct-requirement-no-evidence |
| FND-011 | low | Two comments overstated what their assertions do — one claimed byte comparison catches field reordering, which this encoder normalizes away, and one called a repeated render a statement about timestamp dependence on types that carry no clock. Both now say what they actually hold. A comment that overstates a test is how the next reader stops looking. | tests/semantics_parity.rs:42; tests/semantics_contract.rs | wrong-requirement |

## Mutations Run

Nineteen mutations were applied and reverted, each confirming that a named test
fails when the property is broken. The three most worth recording:

- Emptying `non-success-states.json` leaves the pre-existing state loop green
  and fails only the restored floor. That is direct evidence of the
  empty-population shape, observed rather than argued.
- Narrowing the published `outcome` enum in
  `pgm01-compatibility-view-v1.schema.json` fails the new conformance test,
  which is what makes FND-005 a live gate rather than a one-time check.
- Reordering the expected key shape in the generated-fixture test fails it,
  which the previous `BTreeMap` form could not have done.

## Notes Not Closed Here

Six typed refusal reasons in `src/semantics/mod.rs` — `InvalidIdentity`,
`InvalidSourcePath`, `EmptyRequiredField`, `InvalidDigest`,
`IncompleteOwnershipMetadata` and `InvalidResultStateSet` — are constructed by
the library and asserted by no test, and several encode rules no acceptance
criterion states, such as a 256-byte identifier grammar. `InvalidLegacyField`
and `InvalidLegacyInteger` never reach a caller as typed reasons because
`malformed_view` stringifies them into an unmapped-field reason.
`Pgm01Outcome::Compatible` is declared in FR-010's Outputs and produced by no
path in this mapper. None of these were backed by the retired Python either, so
none is a regression of this cutover; they are pre-existing underspecified code
and belong to their own ticket rather than to this one.
