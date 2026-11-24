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
The commands read and verify stored tag receipt notes only when the receipt policy contains `tag_receipts`; without
that member, they do not inspect existing tag receipt notes or emit tag findings.

The function takes the verified tag receipts, the current set of `refs/tags/*` with their object ids, and the policy's
`tag_receipts` member, and returns:

- `missing_tag` with reason `tag_absent` for a receipt whose `refs/tags/<name>` does not exist;
- `missing_tag` with reason `tag_changed`, carrying `current_object`, for a receipt whose ref now names a different object;
- `tag_no_receipt` for a current tag whose name starts with a policy prefix and for which no verified receipt names its current object id;
- `invalid_receipt`, with its reason, for a receipt that does not verify under FR-081;
- nothing for a receipt whose ref names the receipted object id.

Each receipt is judged on its own object id, so a tag deleted and pushed again at the identical object id passes, a tag
re-signed or re-created differs and is reported as `tag_changed` for the earlier receipt, and a tag with two receipts for
two objects reports the one that is not current. Every line of a note under `refs/notes/assurance-tag-receipts` is a
candidate receipt, so a note line that cannot be parsed is an `invalid_receipt` with reason `predicate_invalid`. The
notes ref being absent is an empty receipt set, not an error.

There is no push time and no grace period for tags: GitHub keeps no push record for tag refs. A tag whose receipt run
has not finished appears as `tag_no_receipt` until the run ends; this is expected, the scheduled check runs on its
own schedule, and a later run no longer lists it.

Accepting a finding. A person records acceptance of a specific finding in the optional `accepted_tags` list of the
`tag_receipts` policy member, which is committed on the protected base branch like the rest of the policy
([FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md)). An entry `{ref, tag_object, accepted_by, reason}`
matches a `missing_tag` or `tag_no_receipt` finding when its `ref` equals the finding's ref and its `tag_object` equals the
receipted object id of a `missing_tag` finding or the current object id of a `tag_no_receipt` finding. A null `tag_object` is
allowed only for a `tag_absent` finding, where the tag does not exist; it never matches a `tag_changed` or `tag_no_receipt`
finding, so an accepted unreceipted tag that is later moved or re-created is a new finding. The match is exact on both
members. A matched finding is reported with the status `accepted`, `finding` (the replaced status, `missing_tag` or
`tag_no_receipt`), the members the finding had, and the entry's `accepted_by` and `accepted_reason` (the entry's `reason`), and does not fail the
scheduled workflow or make `complete` false. The tool never decides that a finding is acceptable: it only reads the list,
and it reads each entry as data, never as an instruction. An `invalid_receipt` cannot be accepted, because its content
is not trusted. The list also covers tags that predate adoption, such as existing release tags that never had a receipt.

Shape of the `receipt-missing` output for tags. Every finding of `receipt-missing` carries `kind`, the string `change` for
a commit finding of FR-077 and `tag` for a finding of this requirement, so a tag `invalid_receipt` is told from a commit
`invalid_receipt`. A tag finding has `kind` `tag` and `status` (`missing_tag`, `tag_no_receipt`, `invalid_receipt` or
`accepted`). A `missing_tag` or `tag_no_receipt` finding carries `ref`; an `invalid_receipt` carries only its note
`commit` and `reason` from the untrusted candidate and has no `ref` or tag name. Findings carry, as the status allows,
`tag_object`, `current_object`, `commit`, `finding`, `accepted_by` and
`accepted_reason`. The workflow fails when any finding of `kind` `tag` has a status other than `accepted`, in
addition to the commit findings of FR-077.

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
- The tag comparison SHALL NOT drop an earlier receipt because a later receipt exists for the same ref.
- The tag comparison SHALL list a receipt that does not verify as `invalid_receipt` with its reason.
- An `invalid_receipt` finding SHALL carry its note commit and reason.
- An `invalid_receipt` finding SHALL NOT carry a ref or tag name because the candidate content is untrusted.
- The commands SHALL read and verify stored tag receipt notes only when the receipt policy contains `tag_receipts`.
- The tag comparison SHALL NOT judge a receipt that does not verify as missing.
- The tag comparison SHALL list each current tag that starts with a policy prefix and has no verified receipt for its current object id as `tag_no_receipt`.
- The tag comparison SHALL read an absent notes ref as an empty receipt set.
- The tag comparison SHALL be a pure function of the verified receipts, the current tag set and the `tag_receipts` policy member, with no I/O.
- The tag comparison SHALL report a `missing_tag` or `tag_no_receipt` finding that an `accepted_tags` entry matches on `ref` and `tag_object` with the status `accepted`, `finding`, the entry's `accepted_by` and `accepted_reason`.
- The tag comparison SHALL NOT accept an `invalid_receipt`.
- The tag comparison SHALL NOT match a null `tag_object` to any finding other than `tag_absent`.
- The `receipt-missing` command SHALL give every finding the member `kind`, `change` or `tag`.
- The missing-receipt workflow SHALL fail when a finding of `kind` `tag` has a status other than `accepted`.
- The commands SHALL NOT write to the repository.
- The commands SHALL NOT keep a log.
- The commands SHALL NOT report a finding as accepted unless an `accepted_tags` entry matches it.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-082-AC-1 | A fixture with a verified receipt for `refs/tags/approve-1` and the tag deleted lists one `missing_tag` with reason `tag_absent` and no other finding. | Test |
| FR-082-AC-2 | The same fixture with the tag re-created as a different tag object lists `missing_tag` with reason `tag_changed` and `current_object` equal to the new object id. | Test |
| FR-082-AC-3 | The same fixture with the tag deleted and pushed again at the identical object id lists no finding; a tag with receipts for two objects, only the second current, lists exactly the first as `tag_changed`. | Test |
| FR-082-AC-4 | A receipt with a changed byte, an unparsable note line and a receipt signed by an unlisted identity are each listed as `invalid_receipt` with `signature_invalid`, `predicate_invalid` and `untrusted_signer` and are not judged as missing. | Test |
| FR-082-AC-5 | The comparison is exercised by the pure function alone for absent, equal, different, repeated-ref, empty-receipt and absent-notes-ref inputs, with identical results from `approval-trail` and `receipt-missing` for the same inputs. | Test |
| FR-082-AC-6 | `receipt-missing` lists a current tag with a policy prefix and no receipt for its current object as `tag_no_receipt`, whatever its age; a tag with a non-matching prefix is not listed; with the notes ref absent every prefix-matching tag is listed `tag_no_receipt` and no error is raised; a tag that gains its receipt is not listed by the next run. | Test |
| FR-082-AC-7 | An empty finding list leaves `missing-receipt-check.yml` green, and each of `tag_no_receipt`, `missing_tag` and `invalid_receipt` of `kind` `tag` fails it. | Test |
| FR-082-AC-8 | A release tag in the policy prefixes deleted after its release attestation was written yields `missing_tag` while the attestation is unchanged and still verifies; no receipt content is read by the release verifier and no attestation content is read by the tag comparison. | Test |
| FR-082-AC-9 | Neither command writes to the repository or keeps a log, and a finding no policy entry matches is never reported as accepted. | Inspection |
| FR-082-AC-10 | An `accepted_tags` entry naming a re-signed tag's earlier `ref` and `tag_object` makes that `tag_changed` finding `accepted` with `finding` `missing_tag`, `accepted_by` and `accepted_reason` and leaves the workflow green; an entry with the same ref and another object id matches nothing; a null entry accepts a `tag_absent` finding; no entry accepts an `invalid_receipt`. | Test |
| FR-082-AC-12 | An unreceipted tag accepted by an entry naming its current object id is `accepted` with `finding` `tag_no_receipt`; after the tag is moved or re-created at another object id the same entry matches nothing and the tag is a `tag_no_receipt` finding again; an entry with a null `tag_object` matches no `tag_no_receipt` and no `tag_changed` finding. | Test |
| FR-082-AC-11 | A `receipt-missing` result gives every finding `kind`, so a tag `invalid_receipt` has `kind` `tag` and a commit `invalid_receipt` has `kind` `change`, and a tag finding holds the members listed in the description. | Test |
| FR-082-AC-13 | An `invalid_receipt` tag finding carries the note commit and reason. | Test |
| FR-082-AC-14 | An `invalid_receipt` tag finding omits `ref` and tag name; verified `missing_tag` and `tag_no_receipt` findings continue to carry their refs. | Test |

## Dependencies

- **Upstream**: [FR-081](./FR-081-tag-receipt-on-approval-and-release-tag-push.md), [FR-077](./FR-077-missing-receipt-check.md), [FR-066](./FR-066-release-attestation-set-and-subjects.md), [FR-067](./FR-067-release-attestations-bind-to-the-signed-release-tag.md), [FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md).
- **Downstream**: [FR-043](./FR-043-approval-trail-of-a-change-assurance-record.md).
