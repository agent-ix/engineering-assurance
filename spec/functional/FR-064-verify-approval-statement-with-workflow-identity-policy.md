---
id: FR-064
title: "Verify approval statements against the trusted CI workflow identity and the role policy"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-055"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-058"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-061"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-010"
    type: "references"
---

# FR-064: Verify approval statements against the trusted CI workflow identity and the role policy

## Description

Tracked by Linear PLAT-1685. Engineering Assurance owns the post-merge verification of approval statements
([FR-058](./FR-058-approval-statement-and-predicate.md)). A statement verifies when its signature is valid, the
signing identity matches the workflow-identity policy, the predicate type is the approval predicate, the statement
schema is satisfied, and the statement names the commit and the pull request that the caller asks about. The verifier
then evaluates the verified statements with the role evaluation function of
[FR-061](./FR-061-role-policy-and-distinct-individuals.md) against the policy of the protected base branch at the
commit the pull request merged onto, so the post-merge record is checked against the same policy that the pre-merge
review gate used.

Which workflow identity is trusted is configuration, not code. The workflow-identity policy is a closed-schema document
read from the protected base branch ([FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md)),
`.engineering-assurance/workflow-identities.json`, with `protocol`
`engineering-assurance.workflow-identities/v1` and the member `identities`: a list of entries whose `role` is one of
`statement_writer`, `record_publisher`, `release_attester` or `receipt_writer`. Every entry has `role`, `issuer`,
`repository`, `workflow_path` and `ref`, matched against the signing certificate's signer workflow (the workflow whose
code signed), its repository, path and ref. For `statement_writer` and `record_publisher` the workflow runs in the
repository itself and `ref` is the protected base branch's ref (`refs/heads/` followed by the branch name). For the
reusable workflows `release_attester` and `receipt_writer` the signer workflow is an Engineering Assurance workflow
called from another repository: `repository` and `workflow_path` name the Engineering Assurance reusable workflow, and
`ref` is the `refs/heads/` or `refs/tags/` ref at which callers pin it. These two entries also carry `source_repository`
(the repository that triggered the run, exact `owner/name`), `source_workflow_path` (the path of the caller workflow in that
repository), `event` (the literal `push`, the trigger event the certificate records) and exactly one of `source_ref` (an exact ref, the default
branch's `refs/heads/` ref for `receipt_writer`) and `source_ref_prefix` (a literal prefix of the form `refs/tags/`
followed by the tag prefix, for `release_attester`, whose release runs are triggered by a tag push). Matching is exact
on every member, with no wildcard; a prefix is compared as literal text. A signing certificate whose signer ref or
source ref is a pull request ref, or any ref other than the listed one, matches no entry, so a workflow edited on a
pull request branch cannot sign a trusted statement. A statement signed by any other identity is
`untrusted_signer`.

The verification command reports one line of JSON, `engineering-assurance.approval-verify-statement-result/v1`, with
`status` one of `verified`, `untrusted_signer`, `signature_invalid`, `wrong_subject`, `predicate_invalid` or
`unverifiable`, and, only when `status` is `unverifiable`, `reason` (one of `statements_not_retained`,
`publish_refused`, `input_unreadable`); the schema is closed and has no other member besides `protocol`, `status` and
`reason`; for a set of statements it also reports the role verdict of FR-061. It adds no cryptography
([NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md)).

The verifier reads the full statements from the private store ([FR-054](./FR-054-record-sink-interface-and-implementations.md)),
where the full private form is always kept for the `public_log` and `private_store` sinks. For a repository whose sink is
`none` nothing is retained, so post-merge verification returns `unverifiable` with the reason
`statements_not_retained`; the pre-merge review gate is unaffected because it reads the platform reviews.

Out of scope: the git-signature `approval-verify` command of
[FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md), which this command does not replace.

## Behavior

- The statement verifier SHALL return `verified` only for a statement with a valid signature.
- The statement verifier SHALL return `verified` only for a signing identity that matches a listed entry on every member.
- The statement verifier SHALL return `untrusted_signer` for a valid signature by an identity the policy does not list.
- The statement verifier SHALL return `untrusted_signer` for a listed workflow signed at a ref other than the listed ref.
- The statement verifier SHALL return `wrong_subject` when the statement's subject, pull request or merged commit
  differs from the caller's request.
- The statement verifier SHALL return `predicate_invalid` for a predicate that breaks the closed schema.
- The statement verifier SHALL return `unverifiable` with the reason `input_unreadable` when the verification inputs cannot be read.
- The statement verifier SHALL return `unverifiable` with the reason `publish_refused` for a statement the leak check refused, which the private store retains.
- The statement verifier SHALL read statements from the private store.
- The statement verifier SHALL return `unverifiable` with the reason `statements_not_retained` for a repository whose sink is `none`.
- The statement verifier SHALL evaluate the verified statements with the role evaluation function of FR-061.
- The statement verifier SHALL read the workflow-identity policy and the role policy from the protected base branch.
- The statement verifier SHALL NOT trust an identity named only inside the statement.
- The statement verifier SHALL NOT count a statement that is not `verified`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-064-AC-1 | A statement signed by the fixture workflow identity listed in the policy at the listed ref, for the requested merged commit and pull request, yields `verified`. | Test |
| FR-064-AC-2 | The same statement checked under a policy that lists a different workflow identity yields `untrusted_signer`. | Test |
| FR-064-AC-3 | A statement with one changed byte yields `signature_invalid`. | Test |
| FR-064-AC-4 | A statement verified against a different merged commit or pull request yields `wrong_subject`. | Test |
| FR-064-AC-5 | A correctly signed statement with an extra predicate member or a wrong predicate type yields `predicate_invalid`. | Test |
| FR-064-AC-6 | A statement whose predicate names a workflow identity as `approver` and is signed by an untrusted identity yields `untrusted_signer`. | Test |
| FR-064-AC-7 | A missing statement file or unreadable trust root yields `unverifiable`. | Test |
| FR-064-AC-8 | A workflow-identity policy or role policy changed on the pull request branch does not change the document used. | Test |
| FR-064-AC-9 | The same workflow path signed at a pull request ref (`refs/pull/N/merge`) yields `untrusted_signer`, and a policy entry with a wildcard or unknown member is refused as invalid. | Test |
| FR-064-AC-10 | Statements for three roles from three individuals, all `verified`, yield the role verdict `satisfied`; with one statement `untrusted_signer`, the verdict is `unsatisfied` naming that role. | Test |
| FR-064-AC-11 | Statements published for a repository on the public sink are read from the private store and verify; for a repository on `none`, verification returns `unverifiable` with `statements_not_retained` while the review gate still returns its verdict. | Test |
| FR-064-AC-12 | A statement that the leak check refused is read from the private store and yields `unverifiable` with `reason` `publish_refused`; a result with a `reason` member on any other status, or with an unknown member, fails the result schema. | Test |
| FR-064-AC-13 | A release statement signed by the `release_attester` workflow called from a tag push `refs/tags/v1.2.0` of the listed source repository verifies against an entry with `source_ref_prefix` `refs/tags/v`; the same signature from a pull request ref, a branch ref, another source repository or an unlisted signer ref yields `untrusted_signer`. | Test |
| FR-064-AC-14 | A receipt signed by the `receipt_writer` workflow called from the listed source repository's default branch ref verifies against an entry with `source_ref`; the same signature from a tag ref yields `untrusted_signer`; an entry for these roles without `source_repository` or with both `source_ref` and `source_ref_prefix` is refused as invalid. | Test |
| FR-064-AC-15 | A release or receipt signed through the listed reusable workflow by a caller workflow at another path in the source repository, or from a run whose trigger event is `schedule`, `workflow_dispatch`, `issue_comment` or `pull_request_target`, yields `untrusted_signer`; an entry for these roles without `source_workflow_path` or `event`, or with an `event` other than `push`, is refused as invalid. | Test |

## Dependencies

- **Upstream**: [FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md),
  [FR-058](./FR-058-approval-statement-and-predicate.md),
  [FR-061](./FR-061-role-policy-and-distinct-individuals.md),
  [NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md).
- **Downstream**: the required-check and change-record requirements of EA-98.
