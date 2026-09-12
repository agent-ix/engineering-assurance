---
id: SR-123
title: "Review of constraint validation bindings after the single-Status collapse"
type: SpecReview
analysis: code-review
scope: "FR-011-CON-3, FR-012-CON-2, FR-012-CON-4, FR-012-CON-5, FR-013-CON-2, FR-014-CON-3, FR-015-CON-2, FR-015-CON-3; constraint coverage bindings"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-012"
    type: reviews
---

## Summary

Installing the module that mints constraint ids made forty `FR-0NN-CON-N`
targets real, and twelve of them reported unbacked. Every one was unbacked for
the same proximate reason: its `Validation` cell read a bare `Test`,
`Inspection` or `Review` with no `TC-` token for the `constraint-validation`
reference to scrape. FR-019 was the only document already using the scrapeable
form.

So the question per row was never "does a test exist" but "does the row say
which test". For eight of the twelve the test already existed and already
asserted the property.

### What was already proven, and only undeclared

- `FR-012-CON-2`, `FR-012-CON-4`, `FR-012-CON-5` — `tc_082_reports_attributed_acceptance_separately`
  already folds the attribution and rejects `agent`, `claude` and `bot` as the
  accepting party, requires the note to name a human, and requires a pending
  matrix to carry null acceptance fields. Three constraints, one test, all
  three asserted before this change.
- `FR-013-CON-2` — `test_migration_waits_on_acceptance_and_claims_no_qualification`
  carries the assertions verbatim (`"stays manual-dispatch only"`,
  `"dispatches nothing and changes no trigger"`). The id was simply missing from
  its `Trace:` list while its siblings CON-1 and CON-3 were present.
- `FR-014-CON-3` — `tc_096_root_package_exports_library_and_native_cli`.
- `FR-015-CON-2`, `FR-015-CON-3` — the inert-fixture audit and the
  parallel-record-family audit in `semantics_parity.rs`, both written for those
  constraints and both naming them in comments while binding neither.

None of these rows needed new evidence. They needed the evidence they had to be
declared, which is the difference between a gate that measures and a gate that
is merely quiet.

### What needed a real assertion

`FR-011-CON-3` says the corpus makes no claim about the live state of its source
repositories. The corpus index already carries the disclaimer
(*"It does not re-observe the source repositories"*), and `tc_070` already read
`limitations` — but asserted only `derived` and `not-computed`, so the sentence
that discharges the constraint was present and unchecked. An assertion that
reads a field and ignores the part that matters is indistinguishable from one
that never ran. The assertion was verified red by mutating the expected
substring before it was accepted green.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-228 | medium | Eight constraints were discharged by existing tests but named no test case in their `Validation` cell, so the coverage reference scraped nothing | FR-012, FR-013, FR-014, FR-015 | correct-requirement-no-evidence |
| FND-229 | low | `tc_070` read the corpus limitations and asserted two tokens while ignoring the sentence that discharges FR-011-CON-3 | FR-011-CON-3 | correct-requirement-no-evidence |

## Disposition

Constraint coverage moves from twelve unbacked to four; repository backing moves
`277/312` to `281/312` with zero status lies.

The four that remain are not declaration gaps and are deliberately left open:
`FR-012-CON-3` (the self-pin rollback question), `FR-016-CON-2` (the withheld
foreign-language bridge disposition), and `FR-018-CON-2`/`FR-018-CON-3` (the
cutover revert evidence and the coexistence-reporting audit). Each needs
evidence or a ruling, not a cell edit, and none is closed here.

No status was promoted to reach these numbers.
