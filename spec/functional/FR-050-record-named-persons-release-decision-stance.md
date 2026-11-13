---
id: FR-050
title: "State that Engineering Assurance records a named person's release decision"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-047"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-048"
    type: "references"
---

# FR-050: State that Engineering Assurance records a named person's release decision

## Description

When the release capability of
[FR-047](./FR-047-release-record-and-signed-release-tag.md) is implemented,
the repository README SHALL state that Engineering Assurance records and
verifies a named person's signed release decision and never decides a release.
The README sentence that today declines to approve a release changes to: it
does not calculate a trust score or import external rules, and it never decides
a release; it records a named person's signed release decision and verifies
it. The README change lands with that implementation, not with this
specification, so the README never describes a command that does not exist.
`spec/spec.md` states the same stance and its Out of Scope bullet on
automating terminal human decisions stays: only the person decides.

## Behavior

- The README SHALL list `release-verify` in its command table.
- The README SHALL keep the statement that maintainer-only commands do not make
  a release decision.
- The README SHALL NOT claim that Engineering Assurance approves, certifies or
  grants a release.
- The README SHALL name the person as the decision maker.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-050-AC-1 | The README states that Engineering Assurance records and verifies a named person's signed release decision. | Inspection |
| FR-050-AC-2 | The README says Engineering Assurance never decides a release and names the person as the decision maker. | Inspection |
| FR-050-AC-3 | The README command table has a `release-verify` row consistent with FR-047, and that row and the stance sentence of this requirement are added to the README in the pull request that implements `release-verify`. | Inspection |
| FR-050-AC-4 | `spec/spec.md` lists FR-046 to FR-050 and NFR-007 and keeps the Out of Scope bullet that automated terminal decisions are excluded. | Inspection |

## Dependencies

- **Upstream**: [FR-047](./FR-047-release-record-and-signed-release-tag.md),
  [FR-048](./FR-048-human-release-decision-workflow.md).
