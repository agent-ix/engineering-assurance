---
id: SR-20261113085806
title: "EARS conformance review of the release evidence snapshot requirements (engineering-assurance PR 226)"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-047, FR-048 (edited lines), FR-051, FR-052, FR-053 (PR #226 diff, head f8669b66)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-053
    type: references
---

# SR-20261113085806: EARS conformance of the release evidence snapshot requirements (engineering-assurance PR 226)

## Summary

Ticket: EA-101. Examined every Description and Behavior statement that this PR adds or edits in
FR-047, FR-048, FR-051, FR-052 and FR-053 against EARS grammar. `quire validate --scope .
"spec/**/*.md"` reports zero grammar warnings on these files. Each Behavior bullet carries one
SHALL, and the unwanted-behaviour bullets use the `If ..., then ... SHALL` form. A manual read
found one subject problem that quire does not flag.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Three FR-053 statements have a subject that is not the system or one of its components: "An attestation that carries the snapshot SHALL take ...", "An attestation ... SHALL NOT alter ..." and "The repository SHALL NOT record a digest ...". An attestation is a document and the repository is a store, so neither statement allocates the obligation to a component. Fix: name the emitter (the EA-97 attestation producer) or the release module as the subject. | spec/functional/FR-053-snapshot-and-attestations-never-replace-the-decision.md:34-38 |

## Verdict

PASS with one low finding. Apart from FR-053, the new and edited statements conform: one SHALL
per bullet, a named subject (`release-verify`, the command, the workflow), and conditional
refusals in unwanted-behaviour form.
