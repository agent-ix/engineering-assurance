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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | The greedy evaluation can report a role missing when a valid assignment exists. An individual can hold approvals for two roles (two accounts, FR-061-AC-8). Roles A, B; individual X approves A and B, Y approves A. Read as fixed assignment, A may take X (the text does not say which candidate), then B has nobody, although A->Y, B->X is valid; the choice of candidate is also unspecified, so the verdict is not order-independent as the Behavior requires. The second clause hints at re-assignment but does not say it, and FR-061-AC-6 checks only one direction (satisfied implies an assignment exists), so no AC catches the wrong `unsatisfied`. Fix: state that a role is covered if and only if the roles already covered plus this role have an assignment of distinct individuals, re-assigning freely (greedy over a maximum bipartite matching, which is exact); make AC-6 "if and only if"; add the X{A,B}, Y{A} case as an AC; and say which assignment the verdict reports (for example the lexicographically first). | spec/functional/FR-061-role-policy-and-distinct-individuals.md:54-58 |
| FND-007 | high | Statements now go through the sinks, but FR-054 selects exactly one implementation per publish with no fallback and the public-log sink sends only the trimmed form. For a repository whose sink is `public_log`, nothing stores the full statement, yet FR-058-AC-8 expects "the private store holds them", and the FR-064 verifier needs the full statement to verify a signature over it and run the role evaluation. With `none`, no statement is kept at all, so post-merge verification is always `unverifiable`. Fix: state that the private form (with the statement) always goes to a private store, alongside the public log when that is selected (the ticket: ticket text, criteria text and model names go to a separate private store), or say where a public repository's statements are kept for the verifier, and state the result for `none`. | spec/functional/FR-058-approval-statement-and-predicate.md:AC-8 |
| FND-008 | medium | `hashes.subject_commit` must be `sha256:` plus 64 hex characters, but the subject commit is a git commit id, which is SHA-1 (40 hex) in ordinary repositories and is what the in-toto subject carries as its git commit digest. The grammar cannot hold the commit id, and a SHA-256 of something else is undefined. Fix: give `subject_commit` the git object id grammar (40 or 64 lowercase hex, `gitCommit`), or say what is hashed. | spec/functional/FR-056-public-and-private-record-split.md:34-35 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | The satisfied/unsatisfied verdict is now exact and order-independent, but "lexicographically smallest" maximum matching has two readings that give different `missing` sets. Roles A, B, C; X approves A and B; Y approves A and C. The maximum matchings are {A-Y, B-X}, {A-X, C-Y} and {B-X, C-Y}. Compared as sorted (role, individual) pair lists, {A-X, C-Y} is smallest (A-X < A-Y), so `missing` is [B]; compared by covered role set first (cover A, then B if still possible), {A-Y, B-X} wins and `missing` is [C]. The AC-10 and AC-11 cases do not tell the two apart. Only the reported assignment and `missing` differ, never the status. Fix: define the order in one sentence (for example "matchings are compared as their (role, individual) pairs sorted by role name, element by element, by byte order of the names") and add this case as an AC. | spec/functional/FR-061-role-policy-and-distinct-individuals.md:57-59, 78-79 |
| FND-010 | medium | Requiring a private store for every `public_log` publish is coherent with the ticket (ticket text, criteria text and model names go to a separate private store, and the public repository still defaults to the public sink), and it fails closed rather than making the system unusable. But FR-055 no longer says what an absent `sinks.json` yields. The Description still promises a default with no explicit choice, the old "no configuration" AC was replaced by one that needs a destination, and no error is named for a private repository whose default `private_store` has no destination (`sink_config_incomplete` speaks of a configuration that "selects" the sink). Fix: state that a private store destination is required configuration for every repository, that a missing document or missing destination is refused with one named code, and add that case to an AC. | spec/functional/FR-055-per-repo-sink-configuration-and-visibility-default.md:18-19, 51-52, 75 |
| FND-011 | low | Because the private form is written only after the leak check passes, a `public_log` publish that the leak check refuses keeps the full private form nowhere, so an approval statement refused this way is lost and FR-064 has no stated result for it. Writing to the private store discloses nothing publicly. Fix: write the private form whatever the leak check decides, or state the verifier's result for a refused statement. | spec/functional/FR-054-record-sink-interface-and-implementations.md:50 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | fixed | - |
