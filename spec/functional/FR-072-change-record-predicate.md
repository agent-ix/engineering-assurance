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
  - target: "ix://agent-ix/engineering-assurance/NFR-008"
    type: "references"
---

# FR-072: Define the change-record/v1 predicate as identifiers, hashes, verdicts and per-role approval references

## Description

Tracked by Linear EA-98.. Tracked by Linear EA-98 and the change-record work of its follow-up tickets. A per-change receipt carries one custom predicate, `change-record/v1`. Its type is the URI with scheme `https`, host `schemas.agent-ix.ai` and path `/attestation/change-record/v1`; the URI is an identity string and no document needs to be hosted there. The statement is a closed schema ([NFR-008](../non-functional/NFR-008-closed-schemas.md)) with one subject: the repository, the merged commit as a git commit digest and the merged tree as a git tree digest, so the receipt survives a squash merge.

The predicate is written to be stored beside the code (git notes, [FR-076](./FR-076-receipt-storage-in-git-notes.md)), so it holds no ticket text, criteria text or model name. Those go to the private form ([FR-056](./FR-056-public-and-private-record-split.md)) and the predicate carries the hash that binds it. The predicate members are:

- `record_id`: the lowercase UUID of the record, equal in the private form.
- `status`: `passed` or `failed`.
- `ticket`: one reference `{kind, id}` of kind `linear_issue`, with the grammar of [FR-056](./FR-056-public-and-private-record-split.md); absent when the contract is missing.
- `criteria`: the acceptance-criterion ids of the ticket, each of the form `AC-N` with N of 1 to 3 digits.
- `spec_criteria`: the spec criterion ids those criteria trace to, each of the form `FR-N-AC-N` or `NFR-N-AC-N`.
- `reviews`: a list of `{kind, verdict, artifact_digest}`; `kind` is one of `spec_review`, `code_review`, `rust_review`, `gap_analysis`; `verdict` is `pass`, `findings` or `fail`; the digest is `sha256:` plus 64 lowercase hex.
- `approvals`: one entry `{role, statement_digest, verdict}` per role of the role policy ([FR-061](./FR-061-role-policy-and-distinct-individuals.md)); `statement_digest` is the digest of the approval statement ([FR-058](./FR-058-approval-statement-and-predicate.md)); `verdict` is `covered` or `missing`.
- `test_results`: the digest of the test-result statement ([FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md)).
- `agent_bom`: the digest of the agent BOM ([FR-073](./FR-073-agent-bom-in-cyclonedx-1-7.md)).
- `private_form`: the digest of the full private form, so the text and the BOM are bound without being copied.
- `failures`: a list of failure codes from the closed set `gate_failed`, `approvals_incomplete`, `approval_role_unstaffed`, `contract_missing`, `reviews_missing`, `leak_check_refused`; empty when `status` is `passed`.

The predicate lists one `approvals` entry for every required role of the policy, including a role with no statement (`missing`), so an incomplete role set is visible and decides the status. It carries no approver name, reviewer name or meaning: those are in the approval statements and the private form.

## Behavior

- The receipt writer SHALL write the change-record predicate with exactly the members this requirement lists.
- The receipt writer SHALL give the statement the merged commit and the merged tree as its subject.
- The receipt writer SHALL list one approvals entry for each role the role policy requires.
- The receipt writer SHALL set `status` to `failed` when any approvals entry is `missing`.
- The receipt writer SHALL set `status` to `passed` only when `failures` is empty.
- The receipt writer SHALL take every value of the predicate from its member's grammar.
- The receipt writer SHALL NOT write ticket text, criteria text, a model name, an approver name or a meaning into the predicate.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-072-AC-1 | A fixture change with a ticket, two criteria, two reviews and three satisfied roles yields a predicate with exactly the listed members, status `passed`, three `covered` approvals and an empty failure list. | Test |
| FR-072-AC-2 | A predicate with an unknown member, a value outside its grammar or a ticket id of kind `repo` fails schema validation. | Test |
| FR-072-AC-3 | A change with the `engineer` role unsigned yields status `failed`, an approvals entry `missing` for `engineer` and the failure `approvals_incomplete`; a policy with an unstaffed role adds `approval_role_unstaffed`. | Test |
| FR-072-AC-4 | The serialized predicate contains none of the fixture's ticket title, criterion text, model name, approver name or meaning. | Test |
| FR-072-AC-5 | The statement's subject carries the squash commit and its tree for a squash-merged fixture, and the tree differs from the reviewed head's tree only when the base moved. | Test |
| FR-072-AC-6 | The predicate type is the URI with scheme `https`, host `schemas.agent-ix.ai` and path `/attestation/change-record/v1`. | Test |

## Dependencies

- **Upstream**: [FR-056](./FR-056-public-and-private-record-split.md), [FR-058](./FR-058-approval-statement-and-predicate.md), [FR-061](./FR-061-role-policy-and-distinct-individuals.md), [NFR-008](../non-functional/NFR-008-closed-schemas.md).
- **Downstream**: [FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md), [FR-075](./FR-075-push-to-main-receipt-workflow.md), [FR-078](./FR-078-receipt-verify.md).
