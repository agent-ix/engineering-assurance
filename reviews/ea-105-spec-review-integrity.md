---
id: SR-20261113163613
title: "Integrity review of the record sink and approval statement requirements (engineering-assurance PR 231)"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-054..FR-065, spec/non-functional/NFR-008..NFR-010, spec/spec.md (PR #231 diff, head dd5a124)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-055
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-061
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-065
    type: references
---

# SR-20261113163613: Integrity review of the record sink and approval statement requirements (engineering-assurance PR 231)

## Summary

Ticket: EA-105 and PLAT-1685. Checked internal consistency of FR-054 to FR-065 and NFR-008 to NFR-010, and consistency with merged FR-036, FR-037, FR-041, FR-042, FR-043, FR-046 to FR-053 and NFR-007. The release path is consistent: FR-065 keeps release decisions as person-signed tags, its ACs mirror FR-053-AC-1, and NFR-007 is untouched. Both tickets' acceptance criteria are covered by at least one AC each. Five consistency defects, two of them contradictions.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-065 runs the approval gate "on the pull request as a required check" that "verifies the statements", but the statement writer runs only "when a pull request is merged" and FR-058/FR-060 make the merged commit the subject. Before merge no statement exists, so the required check can never be satisfied (only the unstaffed check could run). Fix: say what the pre-merge gate evaluates (the adapter approvals against the base policy, unsigned) and what is verified after merge (statements, recorded FAILED when incomplete, as EA-108 plans), or move the gate to the merged commit. | spec/functional/FR-065-workflow-roles-and-relation-to-signed-release-tags.md:28-33 |
| FND-002 | high | FR-061 says identities are equated only when "the policy's `roles` lists both under a shared `individual` key", but the closed member list of `roles` ("the `source` and the list of account identifiers") defines no `individual` key; under NFR-008 such a policy is rejected as having an unknown member, so FR-061-AC-8 cannot pass. Fix: define the member and its shape (see the security review, FND-006, for a recommended shape). | spec/functional/FR-061-role-policy-and-distinct-individuals.md:28-34, 69 |
| FND-003 | medium | FR-059 maps an approval's role from the policy's account lists and FR-058 writes one statement with one `role` per approval, but nothing says which role an account listed under two roles gets. FR-061-AC-7 assumes one identity holds statements for two roles, which needs either one approval per listed role or an explicit role choice by the approver. Fix: state the rule (for example, one approval per role whose list contains the account, or a required `Signed-off-role:` line). | spec/functional/FR-061-role-policy-and-distinct-individuals.md:68 |
| FND-004 | medium | FR-055 reads the sink configuration from the "protected default branch" while FR-061 and FR-064 read their policies from the "protected base branch"; for a pull request into a non-default branch these differ. No requirement says what happens when that branch is not protected. Fix: use one term (base branch of the change, or default branch, for all three documents), and state the result when the branch is unprotected (refuse, fail closed). | spec/functional/FR-055-per-repo-sink-configuration-and-visibility-default.md:22,52 |
| FND-005 | medium | FR-065 says "A change approval is a platform-recorded statement", but merged FR-041, FR-042 and FR-043 remain the signed-tag approval of a change-assurance record, and FR-065 lists them only as the release path. Two change-approval mechanisms now exist and nothing says which the role gate counts or whether a signed approval tag on a change record is still an approval. Fix: state the relation explicitly (FR-041 stays the approval object of a release decision and is not counted by the role gate, or it is), in FR-065. | spec/functional/FR-065-workflow-roles-and-relation-to-signed-release-tags.md:40-45 |

## Verdict

Not mergeable: FND-001 (the pre-merge gate cannot see statements that are only written at merge) and FND-002 (the `individual` key is used but is not a member of the closed policy schema) are contradictions. FND-003 to FND-005 are for the same round.
