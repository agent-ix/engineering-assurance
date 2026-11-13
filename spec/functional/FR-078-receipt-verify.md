---
id: FR-078
title: "Verify a receipt against the pinned workflow identity, the commit and the role policy"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-074"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-064"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-061"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-072"
    type: "references"
---

# FR-078: Verify a receipt against the pinned workflow identity, the commit and the role policy

## Description

Tracked by Linear EA-98.. The `receipt-verify` command verifies the receipt of one commit and reports a status. It reads the note, verifies each bundle's signature and checks that the signer is the change-receipt workflow identity at the ref the workflow-identity policy lists ([FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md)); it checks that the change-record subject is the requested commit and tree; it validates the predicate against its closed schema ([FR-072](./FR-072-change-record-predicate.md)); it checks that the summary's input digests match the other two statements; and it re-evaluates the approvals entries of the change record against the approval statements and the role policy of the protected base branch ([FR-061](./FR-061-role-policy-and-distinct-individuals.md)), so a record whose `approvals` claim a covered role that the statements do not support fails. The result is one JSON line `engineering-assurance.receipt-verify-result/v1` with `status` one of `verified`, `verified_failed` (a genuine failed receipt), `untrusted_signer`, `signature_invalid`, `wrong_subject`, `predicate_invalid`, `approvals_mismatch` or `unverifiable`, plus `reason` only on `unverifiable`. It adds no cryptography ([NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md)).

## Behavior

- The receipt verifier SHALL verify the signature of each bundle through the delegated stack.
- The receipt verifier SHALL return `untrusted_signer` when the signer is not the listed change-receipt identity at the listed ref.
- The receipt verifier SHALL return `wrong_subject` when the subject commit or tree differs from the requested commit.
- The receipt verifier SHALL return `predicate_invalid` when the predicate breaks its closed schema.
- The receipt verifier SHALL return `approvals_mismatch` when an approvals entry claims a role that the verified statements do not cover.
- The receipt verifier SHALL return `verified_failed` for a valid receipt whose status is `failed`.
- The receipt verifier SHALL return `unverifiable` with a reason when the note or the trust inputs cannot be read.
- The receipt verifier SHALL NOT report `verified` for a receipt whose status is `failed`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-078-AC-1 | The fixture receipt of a passing change yields `verified`, and of a failing change yields `verified_failed`. | Test |
| FR-078-AC-2 | A receipt signed by another workflow, or by the change-receipt workflow at a pull request ref, yields `untrusted_signer`. | Test |
| FR-078-AC-3 | A receipt copied to another commit yields `wrong_subject`; one with a changed byte yields `signature_invalid`; one with an extra predicate member yields `predicate_invalid`. | Test |
| FR-078-AC-4 | A change record that claims all roles `covered` while one approval statement is missing yields `approvals_mismatch`. | Test |
| FR-078-AC-5 | A missing note or an unreadable trust root yields `unverifiable` with a named reason. | Test |
| FR-078-AC-6 | A role policy changed on the pushed branch does not change the policy used. | Test |

## Dependencies

- **Upstream**: [FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md), [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md), [FR-061](./FR-061-role-policy-and-distinct-individuals.md), [FR-072](./FR-072-change-record-predicate.md), [NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md).
- **Downstream**: [FR-077](./FR-077-missing-receipt-check.md).
