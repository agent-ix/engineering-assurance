---
id: SR-121
title: "Review of condition-specific, located integration-evidence coverage refusals"
type: SpecReview
analysis: code-review
scope: "FR-017-AC-10, TC-137; integration-evidence coverage refusal reporting"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

## Summary

`make integration-traceability` refused with
`integration_evidence_coverage_invalid` and the message
`quire coverage returned an invalid document`. The document was not invalid.
Capturing the exact bytes the host received showed `quire` exiting `0` with a
valid 197,900-byte document whose every field decoded against the host's own
structs. The repository had unbacked rows; the gate said the tool had
malfunctioned.

### One code for six conditions, and the wrong component named

`validate_coverage` returns `CoverageInvalid` from four sites covering six
conditions — incomplete totals, unbacked rows, status lies, first-party
untracked symbols, the pinned test-case population, and fatal local
diagnostics. `run_coverage` returns the same code for a genuinely unparseable
document. Only that last one is a defect in the coverage tool.

Because the variant's `Display` string was written for the parse case, every
repository-side gap was reported as a fault in another repository. That is not
a wording problem. It sends a reader to the wrong codebase, and it makes a real
`quire` malfunction indistinguishable from routine unbacked rows.

### Located evidence was read and discarded

`quire` supplies `document`, `row_id`, `line`, `path`, `symbol`, and `trace_id`
on its findings. `CoverageReference` decoded only `path`; `CoverageDiagnostic`
decoded only `reason` and `path`. The host therefore could not name a single
offending row even though the locations were already in hand. An error about
files must say where in those files the error is.

### The regression is measurable against what it replaced

`scripts/check_integration_evidence.py`, removed by #49 (`5ef3a24`), returned a
tuple of located findings and kept the parse case in its own branch:
`traceability is {backed}/{total}, expected complete backing`;
`{label}: {names}`; `test-case traceability is {b}/{t}`;
`repository traceability census is partial: {reason}: {path}`; and
`quire coverage returned invalid JSON: {error}`. The port collapsed all of it
into one opaque code and reused the parse branch's wording for the rest. This
change restores the lost behaviour rather than inventing a new one.

### A skipped check was reading as a pass

`quire` reports `status-column-matches-nothing` at `spec/tests.md:67`: the
configured status column does not exist in the authored table, so status
classification was **skipped**. That reason was absent from the fatal set, so
`validate_coverage` went on to read `status_lies`, found `[]`, and accepted it.
The list was empty because the check never ran.

A gate whose subject is status honesty must not certify a property it did not
measure. An unmeasured classification is now a refusal in its own right, not a
silent pass. This is the most serious of the findings: the other four make a
failure hard to diagnose, this one makes a failure invisible.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-221 | high | A skipped status classification was accepted as an empty `status_lies` list, certifying an unmeasured property | FR-017-AC-10, TC-137 | missing-requirement |
| FND-222 | high | Six distinct refusal conditions shared one code whose message attributed repository gaps to the coverage tool | FR-017-AC-10, TC-137 | missing-requirement |
| FND-223 | medium | Authored `document`, `line`, `row_id`, `symbol`, and `trace_id` were decoded away, so no offending row could be named | FR-017-AC-10, TC-137 | missing-requirement |
| FND-224 | medium | Every condition short-circuited, so only the first failing condition of six was ever reported | FR-017-AC-10, TC-137 | missing-requirement |

## Disposition

FR-017-AC-10 is added because no requirement pinned this capability's refusal
taxonomy. FR-016 already establishes the one-code-per-failure-mode convention
for the onboarding and workflow hosts, and `map_quire_process` already honours
it for this capability's process failures; the content failures were the half
that never received it. The criterion states the convention for
integration-evidence so TC-137 binds to a minted target rather than reconciling
against nothing.

The envelope itself is deliberately unchanged. `MachineError` is shared by
roughly twenty capabilities, so located rows are carried in error-variant
fields rendered by `#[error(...)]`, following
`PackageMembershipError::ExpectedPathDuplicate { path }`, and `code()` remains
a `const fn` returning a `&'static str`.
