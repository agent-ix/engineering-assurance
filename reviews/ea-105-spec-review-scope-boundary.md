---
id: SR-20261113163616
title: "Scope-boundary review of record sinks and approval statements (engineering-assurance PR 231)"
type: SpecReview
analysis: scope-boundary
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-054..FR-065, spec/non-functional/NFR-008..NFR-010, spec/spec.md (PR #231 diff, head dd5a124)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-054
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-059
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-065
    type: references
---

# SR-20261113163616: Scope-boundary review of record sinks and approval statements (engineering-assurance PR 231)

## Summary

Ticket: EA-105 and PLAT-1685. Checked allocation between Engineering Assurance, Quoin, the CI workflows and the later EA-97/EA-98 work. Clean: Engineering Assurance owns sinks, leak check, statements and verification; NFR-010 states that Quoin runs no git and no network call; the role mapping belongs to the adapter as the ruling says; the workflow files are left to the code PRs; release decisions stay with FR-047/NFR-007; the three workflow roles have separate identities and no role both signs statements and publishes records.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The statement writer "stores them where the verifier of FR-064 can read them": no requirement names that place, or says whether approval statements go through the sink interface and the leak check. A statement carries free text (`meaning`), the approver identity and the pull request reference of possibly private repositories. Fix: allocate statement storage (for example, statements publish through FR-054 with the repository's resolved sink, so FR-057 applies), with an AC. | spec/functional/FR-065-workflow-roles-and-relation-to-signed-release-tags.md:28-31 |
| FND-002 | low | The ruling says a GitHub review or label maps to a role; the spec maps roles from accounts only and neither specifies nor excludes labels. Fix: state that labels are not approvals (anyone with triage access can apply one), or specify them. | spec/functional/FR-059-source-adapter-interface-and-github-review-adapter.md:34-35 |
| FND-003 | low | The private store is "a destination ... that only the repository's own principals can read", an access property Engineering Assurance neither enforces nor checks, with no AC; and "exactly three implementations" sits beside "SHALL accept a new implementation". Fix: state that access control of the private store belongs to its operator, and drop "exactly". | spec/functional/FR-054-record-sink-interface-and-implementations.md:19, 25, 54 |

## Verdict

Mergeable on scope once FND-001 is fixed; FND-002 and FND-003 are for the same round.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
