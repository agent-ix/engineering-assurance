---
id: SR-122
title: "Review of the acceptance-criterion verification vocabulary and one unbound trace tag"
type: SpecReview
analysis: code-review
scope: "FR-008-AC-4, FR-012-AC-3, FR-013-AC-2, FR-014-AC-2, FR-014-AC-3, FR-016-AC-1, FR-016-AC-4, FR-017-AC-1, FR-017-AC-2, FR-017-AC-3, FR-017-AC-5, FR-017-AC-6, FR-017-AC-7, FR-018-AC-2, FR-019-AC-2, FR-019-AC-6, NFR-004, NFR-005; verification method vocabulary"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/NFR-005"
    type: reviews
---

## Summary

Once #86 made coverage refusals legible, thirteen
`uncatalogued-verification-method` diagnostics became readable. Each said the
same thing: *"neither a declared verification_catalog method id nor a declared
class, so nothing can say what discharging it means."*

The `Verification` cell of an acceptance criterion carries a declared
verification class. The catalogue's `evidence_kind` — `Property`, `Integration`, `E2E`,
`Compile`, `Static` — belongs in the Test Matrix `Type` column. One vocabulary,
several uses. This repository wrote the evidence kind in both places, so the
column that is supposed to say *how* a criterion is discharged repeated what
the `Type` column already said, in a spelling the catalogue does not define.

### The mapping was read, not chosen

Each catalogue entry declares its own class beside its evidence kind, so the
rewrite is a lookup rather than a judgement:

| written | catalogue method | class |
| --- | --- | --- |
| `Property` | `property-based-testing` | Test |
| `Integration` | `integration-testing` | Test |
| `E2E` | `e2e-testing` | Test |
| `Compile` | `compile-time-check` | Analysis |
| `Static` | `sast` / `static-quality` / `architecture-conformance` | Analysis |

Nothing is lost. Every affected test case already carries its evidence kind in
the `Type` column of `spec/tests.md` — `TC-110 Property`, `TC-109 E2E`,
`TC-116 Compile`, `TC-117 Static` — so the information moved to the column that
owns it instead of being duplicated into one that does not.

`Compile` and `Static` becoming `Analysis` rather than `Test` is the part worth
checking rather than assuming: `compile-time-check` is classed `Analysis`
because its evidence is that the build succeeded, and the static audits are
classed `Analysis` because they reason over source without executing it.

### NFR metric methods were free text

Seven `Measurement and Evaluation` `Method` cells named an activity rather than
a catalogue method, including one malformed value — `` `#![forbid `` — where a
backtick-quoted attribute had been written into a method column. The three
NFR-004 metrics and NFR-005's executable-path audit all check an implemented
structure against a declared one, which is `architecture-conformance`.
NFR-005's unsafe-block and pinned-version metrics are discharged by the build
succeeding, which is `compile-time-check`. The marker-discipline percentage is
a structural measure over source, which is `static-quality`.

### One tag that reached no channel

`tests/test_migration_contract.py:1` opened its module docstring with
`FR-013 —`, which matches the `python-docstring-id` form. A module container
does not bind trace ids, so the tag reached no channel and FR-013 read as a
test nobody wrote — indistinguishable from an unwritten test.

The criterion was never unbacked: `test_every_family_carries_exactly_one_decision`
already carries `Trace: FR-013-AC-1, TC-087`. The docstring was prose that
happened to be written in tag form. It now names the requirement in a sentence
instead, so the reference survives and the false binding does not.

The five remaining `tag-on-non-binding-symbol` diagnostics are qa-corpus
detection fixtures whose purpose is to carry ids that bind to nothing.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-225 | medium | Twenty acceptance criteria named an evidence kind where an IADT class belongs, so no catalogue entry defined what discharging them meant | NFR-005 | wrong-requirement |
| FND-226 | medium | Seven NFR metric methods were free text outside the catalogue, one of them a malformed backtick-quoted attribute | NFR-004, NFR-005 | wrong-requirement |
| FND-227 | low | A module docstring written in tag form bound nothing, making a backed criterion read as unwritten | NFR-005-AC-3 | correct-requirement-no-evidence |

## Disposition

No acceptance criterion changes meaning and no test changes behaviour, so this
adds no test case. The measurable outcome is the diagnostic count:
`uncatalogued-verification-method` 13 to 0, and first-party
`tag-on-non-binding-symbol` 1 to 0.

Neither reason is in the fatal census set, so neither blocked
`integration-traceability` and neither is newly enforced here. Making the
catalogue check fatal would be a separate decision: it would fail the gate on
advisory vocabulary drift, which is a different contract from the census
completeness the fatal set exists to protect.
