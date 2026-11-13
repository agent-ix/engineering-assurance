---
id: FR-072
title: "Define the change-record/v1 predicate as identifiers, hashes, verdicts and per-role approval references"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-056"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-058"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-061"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-063"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-064"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-008"
    type: "references"
---

# FR-072: Define the change-record/v1 predicate as identifiers, hashes, verdicts and per-role approval references

## Description

Tracked by Linear EA-98. A per-change receipt carries one custom predicate, `change-record/v1`. Its type is the URI with scheme `https`, host `schemas.agent-ix.ai` and path `/attestation/change-record/v1`; the URI is an identity string and no document needs to be hosted there. The statement is a closed schema ([NFR-008](../non-functional/NFR-008-closed-schemas.md)) with one subject: the repository, the merged commit as a git commit digest and the merged tree as a git tree digest, so the receipt survives a squash merge.

The predicate is written to be stored beside the code (git notes, [FR-076](./FR-076-receipt-storage-in-git-notes.md)), so it holds no ticket text, criteria text or model name. Those go to the private form ([FR-056](./FR-056-public-and-private-record-split.md)) and the predicate carries the hash that binds it. Every digest in this specification is `sha256:` followed by 64 lowercase hexadecimal characters computed over the RFC 8785 canonical bytes of the named JSON document; for the members below the named documents are the approval statement, the test-result statement, the agent BOM and the full private form. The private form's `change_record` is the text-bearing change record without the predicate's `private_form` member, so no digest contains itself. The predicate members are:

- `record_id`: the lowercase UUID of the record, equal in the private form.
- `status`: `passed` or `failed`.
- `ticket`: one reference `{kind, id}` of kind `linear_issue`, with the grammar of [FR-056](./FR-056-public-and-private-record-split.md); absent when the contract is missing.
- `criteria`: the acceptance-criterion ids of the ticket, each of the form `AC-N` with N of 1 to 3 digits.
- `spec_criteria`: the spec criterion ids those criteria trace to, each of the form `FR-N-AC-N` or `NFR-N-AC-N`.
- `reviews`: a list of `{kind, verdict, artifact_digest}`; `kind` is one of `spec_review`, `code_review`, `rust_review`, `gap_analysis`; `verdict` is `pass`, `findings` or `fail`.
- `approvals`: one entry per role of the role policy ([FR-061](./FR-061-role-policy-and-distinct-individuals.md)): `{role, verdict}` with `verdict` `covered` or `missing`, plus `statement_digest` if and only if `verdict` is `covered`.
- `test_results`: the digest of the test-result statement ([FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md)).
- `agent_bom`: the digest of the agent BOM ([FR-073](./FR-073-agent-bom-in-cyclonedx-1-7.md)).
- `private_form`: the digest of the full private form, so the text and the BOM are bound without being copied.
- `failures`: a list from the closed set `gate_failed`, `approvals_incomplete`, `approval_role_unstaffed`, `approvals_unavailable`, `contract_missing`, `reviews_missing`, `leak_check_refused`; empty when `status` is `passed`.

The predicate lists one `approvals` entry for every required role of the policy, so an incomplete role set is visible and decides the status. A role evaluation that is `unavailable` ([FR-063](./FR-063-unstaffed-required-role-fails-visibly.md)) or statements that are `unverifiable` ([FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md)) are recorded as `approvals_unavailable`, and an unstaffed role as `approval_role_unstaffed`. The predicate carries no approver name, reviewer name or meaning: those are in the approval statements and the private form.

## Behavior

- The receipt writer SHALL write the change-record predicate with exactly the members this requirement lists.
- The receipt writer SHALL give the statement the merged commit and the merged tree as its subject.
- The receipt writer SHALL list one approvals entry for each role the role policy requires.
- The receipt writer SHALL write `statement_digest` in an approvals entry if and only if its verdict is `covered`.
- The receipt writer SHALL set `status` to `failed` when any approvals entry is `missing`.
- The receipt writer SHALL record `approvals_unavailable` when the role evaluation is unavailable or the statements are unverifiable.
- The receipt writer SHALL set `status` to `passed` only when `failures` is empty.
- The receipt writer SHALL take every value of the predicate from its member's grammar.
- The receipt writer SHALL compute every digest as SHA-256 over the RFC 8785 canonical bytes of the named document.
- The receipt writer SHALL omit the `private_form` member from the change record it stores in the private form.
- The receipt writer SHALL NOT write ticket text, criteria text, a model name, an approver name or a meaning into the predicate.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-072-AC-1 | A fixture change with a ticket, two criteria, two reviews and three satisfied roles yields a predicate with exactly the listed members, status `passed`, three `covered` approvals each with a statement digest and an empty failure list. | Test |
| FR-072-AC-2 | A predicate with an unknown member, a value outside its grammar or a ticket id of kind `repo` fails schema validation. | Test |
| FR-072-AC-3 | A change with the `engineer` role unsigned yields status `failed`, an approvals entry `missing` for `engineer` without a `statement_digest` and the failure `approvals_incomplete`; a policy with an unstaffed role adds `approval_role_unstaffed`; an unavailable role evaluation adds `approvals_unavailable`. | Test |
| FR-072-AC-4 | The serialized predicate contains none of the fixture's ticket title, criterion text, model name, approver name or meaning. | Test |
| FR-072-AC-5 | The statement's subject carries the squash commit and its tree for a squash-merged fixture. | Test |
| FR-072-AC-6 | The predicate type is the URI with scheme `https`, host `schemas.agent-ix.ai` and path `/attestation/change-record/v1`. | Test |
| FR-072-AC-7 | Recomputing each digest from the canonical bytes of the named document reproduces the predicate's value, and the stored change record has no `private_form` member. | Test |

## Dependencies

- **Upstream**: [FR-056](./FR-056-public-and-private-record-split.md), [FR-058](./FR-058-approval-statement-and-predicate.md), [FR-061](./FR-061-role-policy-and-distinct-individuals.md), [NFR-008](../non-functional/NFR-008-closed-schemas.md).
- **Downstream**: [FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md), [FR-075](./FR-075-push-to-main-receipt-workflow.md), [FR-078](./FR-078-receipt-verify.md).
