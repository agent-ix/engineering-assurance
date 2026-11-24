---
id: FR-082
title: "Detect deleted, changed and unreceipted tags by comparing the tag set with the tag receipts"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-081"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-077"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-043"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-066"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-067"
    type: "references"
---

# FR-082: Detect deleted, changed and unreceipted tags by comparing the tag set with the tag receipts

## Description

Tracked by Linear EA-137. The tag receipts of [FR-081](./FR-081-tag-receipt-on-approval-and-release-tag-push.md) are the
only evidence that a tag once existed. This requirement defines the one comparison that turns them into findings, and
where it runs: in `approval-trail` for one record ([FR-043](./FR-043-approval-trail-of-a-change-assurance-record.md)) and
in `receipt-missing` for the whole repository ([FR-077](./FR-077-missing-receipt-check.md)), which the scheduled
`missing-receipt-check.yml` already runs. Both call the same pure function, so they cannot disagree.

The function takes the verified tag receipts and the current set of `refs/tags/*` with their object ids, and returns:

- `missing_tag` with reason `tag_absent` for a receipt whose `refs/tags/<name>` does not exist;
- `missing_tag` with reason `tag_changed`, carrying `current_object`, for a receipt whose ref now names a different object;
- nothing for a receipt whose ref names the receipted object id.

Each receipt is judged on its own object id, so a tag deleted and pushed again at the identical object id passes, a tag
re-signed or re-created differs and is reported as `tag_changed` for the earlier receipt, and a tag with two receipts for
two objects reports the one that is not current. A re-signed tag is therefore a standing finding that the earlier receipt
no longer matches: the tool reports it and does not decide whether the change was legitimate. People read the finding and
accept or reject it; the tool assigns no verdict on it. A receipt that does not verify under FR-081 is not judged; it
is listed as `invalid_receipt` with its reason. Every line of a note under `refs/notes/assurance-tag-receipts` is a
candidate receipt, so a note line that cannot be parsed is an `invalid_receipt` with reason `predicate_invalid`.

`receipt-missing` gains a third finding for the repository as a whole: a current tag whose name starts with a policy
prefix and has no verified receipt for its current object id is listed as `tag_no_receipt`. This covers a tag pushed by a
ruleset bypass or whose receipt run failed, as `no_note` does for commits. A tag counts only when the platform's record
of ref pushes gives a push time at or after the policy's `tag_receipts.adopted_at` and older than `grace_minutes`; a tag
pushed before `adopted_at` is never listed, and a tag whose push time cannot be found is listed with reason
`status_unknown`. The workflow `missing-receipt-check.yml` fails when any of `tag_no_receipt`, `missing_tag` or
`invalid_receipt` is listed, in addition to the commit findings of FR-077.

Relation to the release attestation. For a release tag the attestation workflow ([FR-066](./FR-066-release-attestation-set-and-subjects.md),
[FR-067](./FR-067-release-attestations-bind-to-the-signed-release-tag.md)) attests what the release was when its signed
tag verified; it holds evidence about the release and no list of tags to compare the repository against. The tag receipt
records only that the ref pointed at an object when it was pushed. A release tag whose prefix is in the receipt policy
gets both: the attestation is the evidence about the release, and the receipt is the evidence that lets a later deletion
or move of its tag ref be seen. Neither copies the other's content, and a receipt never stands in for an attestation.

Out of scope: protecting the refs, reading the tag's signature or meaning, and any persistent store written by the
commands (they write nothing; the evidence is the receipt set CI wrote).

## Behavior

- The tag comparison SHALL list a `missing_tag` with reason `tag_absent` for each verified receipt whose `refs/tags/<name>` does not exist.
- The tag comparison SHALL list a `missing_tag` with reason `tag_changed` and `current_object` for each verified receipt whose ref names a different object id.
- The tag comparison SHALL pass a receipt whose ref names the receipted object id.
- The tag comparison SHALL judge each receipt on its own object id.
- The tag comparison SHALL NOT drop an earlier receipt because a later one exists for the same ref.
- The tag comparison SHALL list a receipt that does not verify as `invalid_receipt` with its reason.
- The tag comparison SHALL NOT judge it.
- The tag comparison SHALL be a pure function of the verified receipts and the current tag set, with no I/O.
- The `receipt-missing` command SHALL list a current tag that starts with a policy prefix, was pushed at or after `adopted_at` and older than `grace_minutes`, and has no verified receipt for its current object id, as `tag_no_receipt`.
- The `receipt-missing` command SHALL NOT list a tag pushed before `adopted_at`.
- The `receipt-missing` command SHALL list a tag whose push time cannot be found as `tag_no_receipt` with reason `status_unknown`.
- The missing-receipt workflow SHALL fail when `tag_no_receipt`, `missing_tag` or `invalid_receipt` is listed.
- The commands SHALL NOT write to the repository, keep a log, or mark a listed finding resolved.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-082-AC-1 | A fixture with a verified receipt for `refs/tags/approve-1` and the tag deleted lists one `missing_tag` with reason `tag_absent` and no other finding. | Test |
| FR-082-AC-2 | The same fixture with the tag re-created as a different tag object lists `missing_tag` with reason `tag_changed` and `current_object` equal to the new object id. | Test |
| FR-082-AC-3 | The same fixture with the tag deleted and pushed again at the identical object id lists no finding; a tag with receipts for two objects, only the second current, lists exactly the first as `tag_changed`. | Test |
| FR-082-AC-4 | A receipt with a changed byte, an unparsable note line and a receipt signed by an unlisted identity are each listed as `invalid_receipt` with `signature_invalid`, `predicate_invalid` and `untrusted_signer` and are not judged as missing. | Test |
| FR-082-AC-5 | The comparison is exercised by the pure function alone for absent, equal, different, repeated-ref and empty-receipt inputs, with identical results from `approval-trail` and `receipt-missing` for the same inputs. | Test |
| FR-082-AC-6 | `receipt-missing` lists a current tag with a policy prefix and no receipt, pushed after `adopted_at` and older than `grace_minutes`, as `tag_no_receipt`; a tag pushed before `adopted_at`, a tag with a non-matching prefix and a tag within `grace_minutes` are not listed; a tag absent from the push record is listed with `status_unknown`. | Test |
| FR-082-AC-7 | An empty finding list leaves `missing-receipt-check.yml` green and each of `tag_no_receipt`, `missing_tag` and `invalid_receipt` fails it. | Test |
| FR-082-AC-8 | A release tag in the policy prefixes deleted after its release attestation was written yields `missing_tag` while the attestation is unchanged and still verifies; no receipt content is read by the release verifier and no attestation content is read by the tag comparison. | Test |
| FR-082-AC-9 | Neither command writes to the repository, keeps a log or reports a finding as resolved. | Inspection |

## Dependencies

- **Upstream**: [FR-081](./FR-081-tag-receipt-on-approval-and-release-tag-push.md), [FR-077](./FR-077-missing-receipt-check.md), [FR-066](./FR-066-release-attestation-set-and-subjects.md), [FR-067](./FR-067-release-attestations-bind-to-the-signed-release-tag.md).
- **Downstream**: [FR-043](./FR-043-approval-trail-of-a-change-assurance-record.md).
