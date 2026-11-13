---
id: FR-064
title: "Verify an approval statement against the trusted CI workflow identity"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-058"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-061"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-010"
    type: "references"
---

# FR-064: Verify an approval statement against the trusted CI workflow identity

## Description

Tracked by Linear PLAT-1685. Engineering Assurance owns the verification of an approval statement
([FR-058](./FR-058-approval-statement-and-predicate.md)). A statement verifies when its signature is valid, the
signing identity matches the workflow-identity policy, the predicate type is the approval predicate, the statement
schema is satisfied, and the statement names the commit and the pull request that the caller asks about. Which
workflow identity is trusted is configuration, not code: the policy lists the accepted certificate identity patterns
and the OIDC issuer, and a statement signed by any other identity is `untrusted_signer`.

The verification command reports one line of JSON, `engineering-assurance.approval-verify-statement-result/v1`, with
`status` one of `verified`, `untrusted_signer`, `signature_invalid`, `wrong_subject`, `predicate_invalid` or
`unverifiable`. It adds no cryptography ([NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md)). The
workflow-identity policy is a closed-schema document read from the protected base branch together with the role policy
of [FR-061](./FR-061-role-policy-and-distinct-individuals.md).

Out of scope: the role policy's evaluation (FR-061) and the git-signature `approval-verify` command of
[FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md), which this command does not replace.

## Behavior

- The statement verifier SHALL return `verified` only for a statement with a valid signature.
- The statement verifier SHALL return `verified` only for a signing identity matching the workflow-identity policy.
- The statement verifier SHALL return `untrusted_signer` for a valid signature by an identity the policy does not list.
- The statement verifier SHALL return `wrong_subject` when the statement's subject, pull request or merged commit
  differs from the caller's request.
- The statement verifier SHALL return `predicate_invalid` for a predicate that breaks the closed schema.
- The statement verifier SHALL return `unverifiable` when the verification inputs cannot be read.
- The statement verifier SHALL read the workflow-identity policy from the protected base branch.
- The statement verifier SHALL NOT trust an identity named only inside the statement.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-064-AC-1 | A statement signed by the fixture workflow identity listed in the policy, for the requested merged commit and pull request, yields `verified`. | Test |
| FR-064-AC-2 | The same statement checked under a policy that lists a different workflow identity yields `untrusted_signer`. | Test |
| FR-064-AC-3 | A statement with one changed byte yields `signature_invalid`. | Test |
| FR-064-AC-4 | A statement verified against a different merged commit or pull request yields `wrong_subject`. | Test |
| FR-064-AC-5 | A correctly signed statement with an extra predicate member or a wrong predicate type yields `predicate_invalid`. | Test |
| FR-064-AC-6 | A statement whose predicate names a workflow identity as `approver` and is signed by an untrusted identity yields `untrusted_signer`. | Test |
| FR-064-AC-7 | A missing statement file or unreadable trust root yields `unverifiable`. | Test |
| FR-064-AC-8 | A workflow-identity policy changed on the pull request branch does not change the policy used. | Test |

## Dependencies

- **Upstream**: [FR-058](./FR-058-approval-statement-and-predicate.md),
  [FR-061](./FR-061-role-policy-and-distinct-individuals.md),
  [NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md).
- **Downstream**: the required-check and change-record requirements of EA-98.
