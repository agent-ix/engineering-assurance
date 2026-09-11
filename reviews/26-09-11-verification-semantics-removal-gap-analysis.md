---
id: SR-099
title: "Gap analysis of the verification-semantics retirement"
type: SpecReview
analysis: gap-analysis
scope: "FR-008, FR-009, FR-010, FR-015, NFR-004, US-005, StR-002; spec/tests.md TC-052..TC-068 and TC-100..TC-104; src/semantics/; tests/semantics_contract.rs, tests/semantics_parity.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-018"
    type: reviews
---

## Summary

Audited whether every acceptance criterion the deleted
`tests/test_verification_semantics.py` backed is now backed by a Rust test that
fails when the property is violated, rather than merely tagged. All fourteen
deleted tests were walked one by one against the Rust suite.

Every criterion has a counterpart, and several are strictly stronger than what
was deleted: the authority allocation is restated in the test instead of read
back from the library, population floors replace unguarded loops, all four
forbidden aggregate report fields are refused instead of one, and the
negative-capability detector is itself proven against capability mutants. No
criterion lost its backing.

`quire coverage` reports no unbacked row and no status lie under FR-008,
FR-009, FR-010 or FR-015, and the repository-wide backed-criterion total is
unchanged by the removal of fourteen Python tests — which is the measurement
that distinguishes "the Rust took up the criteria" from "the matrix lost sight
of them".

## Verdict

**PASS** — no unbacked row in scope and no criterion left without a failing
test. The gaps found were attribution and residue; each is closed below or
recorded as pre-existing and out of this ticket's scope.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The one behaviour of the retired module with no Rust counterpart at all: it validated every instance against the packaged Draft-7 schemas before applying a semantic rule. The Rust port replaced that with typed decoding, which is stricter about structure and says nothing about the schema files — files this repository still ships and still names in its compatibility matrix. Nothing would have reported a drift between the published contract and the types, and the first party to notice would have been an external consumer. Closed: conformance is now asserted for every governed fixture and four produced views. | engineering_assurance/schemas/, tests/semantics_contract.rs | correct-requirement-no-evidence |
| FND-002 | medium | Four Test Matrix rows described refusal families their own tagged tests did not reach — the work lived under FR-015 tags instead. A regression would still have been caught by the suite, but not by the criterion the row names, which is what a matrix row is for. Closed for TC-053, TC-058, TC-060 and TC-066. | spec/tests.md TC-053, TC-058, TC-060, TC-066 | correct-requirement-no-evidence |
| FND-003 | medium | US-005-AC-2's clause forbidding an inferred human decision had no test, and the only report fixture populates every field including its decision reference. Every "No X declared." branch of the renderer, and the "No decision recorded." branch in particular, was unreachable from the test suite — the branch where an empty report renders as a report with no objections. Closed. | spec/usecase/US-005; src/semantics/report.rs:227 | correct-requirement-no-evidence |
| FND-004 | medium | The captured reference fixtures carried no in-band provenance, so their authenticity rested entirely on a commit message. Both reviewers reproduced them from the retired implementation, but a later reader could not have known how. Closed: the module doc names the revision the captures came from and the exact procedure to re-derive them. | tests/fixtures/semantics-*.json; tests/semantics_parity.rs | missing-requirement |
| FND-005 | low | Six typed refusal reasons are constructed by the library and asserted by no test: `InvalidIdentity`, `InvalidSourcePath`, `EmptyRequiredField`, `InvalidDigest`, `IncompleteOwnershipMetadata`, `InvalidResultStateSet`. Several encode rules no acceptance criterion states — a 256-byte identifier grammar and a snake-case result-state grammar among them — so this is underspecified code with no owning requirement. Not a regression of this cutover: the retired Python backed none of them either. Recorded for its own ticket. | src/semantics/mod.rs:504,531,550,564,778,804 | missing-requirement |
| FND-006 | low | `InvalidLegacyField` and `InvalidLegacyInteger` never reach a caller as typed reasons; `malformed_view` stringifies them into an unmapped-field reason, leaving prose as the only discriminator for a malformed PGM-01 record. Defensible, because such a record is classified rather than refused, but FR-015-AC-3 asks for typed reasons and nothing asserts either kind. Pre-existing; recorded. | src/semantics/pgm01.rs:281,310,684 | missing-requirement |
| FND-007 | low | `Pgm01Outcome::Compatible` is declared in FR-010's Outputs and produced by no path in this mapper, which only ever assigns lossy, incompatible or unreadable. It has an owning requirement, so it is not untraced code, but it is an unreachable variant. Pre-existing; recorded. | src/semantics/pgm01.rs:40 | wrong-requirement |
| FND-008 | low | The FR-008-AC-4 / NFR-004-AC-2 detector is a lexical identifier scan over `src/semantics/` alone, while NFR-004-AC-2 says "the package". It cannot see a capability reached through a helper in another module. Partly closed — the vocabulary now also names `io`, `net` and `libc`, each with a mutant — and otherwise mitigated by TC-101's parsed audit over every library module, which the matrix already records. | tests/semantics_parity.rs:88 | correct-requirement-no-evidence |

## Checks That Came Back Clean

- No dangling reference to the deleted module. Nothing under `src/`, `tests/`,
  `engineering_assurance/`, `setup.cfg` or CI imports or executes it; the
  remaining mentions are spec prose, historical reviews, and one module comment
  recording what the replacement inherited.
- The Rust link-target rules match the retired Python's exactly, including
  `decision_subject` targeting a report.
- The state vocabulary is pinned on both sides: the schema enum, the typed
  vocabulary, and the declared non-success list agree, now with a floor.
- TC-100..TC-104 statuses understate rather than overstate what is backed.
