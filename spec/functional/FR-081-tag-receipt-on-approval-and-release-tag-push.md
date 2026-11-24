---
id: FR-081
title: "Write a signed tag receipt when an approval or release tag is pushed"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-064"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-070"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-072"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-074"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-075"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-076"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-008"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-011"
    type: "references"
---

# FR-081: Write a signed tag receipt when an approval or release tag is pushed

## Description

Tracked by Linear EA-137. A git tag ref can be deleted or moved, and a deleted tag leaves nothing in the tag set to
show it was ever there. Only evidence recorded outside the deletable ref can show it. This requirement defines that
evidence: a tag receipt, written by CI at the moment a matching tag is pushed. A tag receipt records what was pushed
and never what the tag means; it neither reads a `Signed-off-meaning`, nor decides whether the tag is an approval,
nor verifies the tag's own signature (FR-036, FR-041 and FR-047 own those).

The receipt policy of [FR-075](./FR-075-push-to-main-receipt-workflow.md) gains one optional member, `tag_receipts`,
a closed object with `prefixes` (a non-empty list of at most 16 literal strings of 1 to 128 bytes, each compared as
literal text against the tag name after `refs/tags/`, with no wildcard) and `adopted_at` (a UTC instant
`YYYY-MM-DDTHH:MM:SSZ`; the earliest push time that needs a tag receipt). A repository without `tag_receipts` writes no
tag receipt and this requirement does not apply to it. A tag pushed before `adopted_at` has no receipt and is not
reported missing ([FR-082](./FR-082-detect-deleted-and-changed-tags-from-tag-receipts.md)).

The reusable workflow `tag-receipt.yml` ([FR-070](./FR-070-release-and-receipt-workflow-files.md)) runs on a push of a
tag. It reads the receipt policy from the protected base branch ([FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md)),
and when the pushed tag name starts with one of the `prefixes` and the platform's push time is not before `adopted_at`,
it signs one in-toto statement with the workflow identity of the `receipt_writer` role ([FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md)),
and stores it as one JSON Lines entry in a git note on the peeled commit under the notes ref
`refs/notes/assurance-tag-receipts`. This is a different ref from `refs/notes/assurance-receipts` of
[FR-076](./FR-076-receipt-storage-in-git-notes.md), so a change receipt and a tag receipt never share a note. A tag whose
name matches no prefix, or whose push is before `adopted_at`, ends the run successfully with no receipt. The job that
signs executes no code from the repository and no agent ([NFR-011](../non-functional/NFR-011-signing-jobs-run-no-agent.md));
the signing configuration, sink and failure codes follow [FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md)
for the resolved sink of the repository.

The statement is a closed schema ([NFR-008](../non-functional/NFR-008-closed-schemas.md)). Its type is the URI with
scheme `https`, host `schemas.agent-ix.ai` and path `/attestation/tag-receipt/v1`; the URI is an identity string and
no document needs to be hosted there. It has one subject: `name` equal to the full ref name `refs/tags/<tag>` and
`digest` with the single member `gitTag`, the id of the tag object the ref named when pushed (for a lightweight tag
that is the commit id). The predicate has exactly these members: `repository` (exact `owner/name`), `ref` (equal to the
subject name), `tag_object` (equal to the subject digest), `commit` (the id the tag peels to, which is the commit
that carries the note), `pushed_at` (the platform's push time as a UTC instant, taken from the trigger context and
never from a tagger, author or committer date), `trailers_readable` (boolean) and `record_ids`. `record_ids` is the
list of every `Approved-Record-Id` trailer value of the tag message, copied byte for byte in message order, and it is
empty when the tag has none. This is the decision on attribution: the receipt carries the trailer values because a
receipt whose tag has since been deleted could otherwise not be assigned to any record, and an `approval-trail` for one
record would have to list every deleted tag of the repository, or none. The values are data captured by the same
trailer reader FR-043 uses; the receipt compares them with nothing and attaches no meaning to them. When the tag
message cannot be read for any reason of FR-043's `unreadable` rule, repeats more than 64 values, or holds a value
over 256 bytes, `trailers_readable` is false and `record_ids` is empty; such a receipt is attributed to no record
and is reported in the trail of every record (FR-043).

The receipt is evidence for one push of one tag object. A tag deleted and pushed again at the same object id is a second
push of an identical statement: the workflow skips writing when the note already holds an entry with the same `ref` and
`tag_object`, so the note is not doubled, and the earlier receipt still matches the tag. A tag re-signed or re-created
with different content has a different object id: it gets its own new entry, and the earlier entry is never removed
or replaced. The note holds at most 1 MiB; a larger note is refused with `tag_receipt_note_too_large`. The `notes` job
fetches the notes ref, merges with `git notes merge` using the `cat_sort_uniq` strategy and pushes, retries a rejected
push up to five times with a fresh fetch and merge, and then fails with `tag_receipt_note_conflict` and writes nothing.
A note on a public repository is a public write: the workflow runs the leak check of
[FR-057](./FR-057-fail-closed-pre-publish-leak-check.md) on the text members of the statement first, and a refusal
fails the run with `tag_receipt_leak_check_refused` and writes no receipt.

A tag receipt verifies when the signature is valid through the delegated stack ([NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md)),
the signer matches a `receipt_writer` entry of the workflow-identity policy that carries `source_ref_prefix` on every
member of [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md), the certificate's source ref
equals the statement's `ref`, the predicate type and closed schema hold, the subject name equals the predicate `ref`,
the subject digest equals `tag_object`, `repository` equals the repository, and `commit` is the commit that carries
the note. Anything else is an `invalid_receipt` with one reason of `signature_invalid`, `untrusted_signer`,
`predicate_invalid` or `wrong_subject`. A change receipt is verified only against the `receipt_writer` entry with
`source_ref`, so a tag receipt is never accepted as a change receipt and a change receipt is never accepted as a tag
receipt.

Out of scope: verifying or interpreting the tag's own signature or meaning, protecting any ref (the documentation
requirement of [FR-083](./FR-083-document-the-adoption-ruleset-for-tags-notes-and-records.md) covers it), changing the
change receipt of FR-074, and the release attestation (FR-066, FR-067), which this receipt neither replaces nor repeats.

## Behavior

- The tag receipt workflow SHALL run on a push of a tag.
- The tag receipt workflow SHALL read the receipt policy from the protected base branch.
- The tag receipt workflow SHALL write nothing and end successfully when the tag name starts with no policy prefix or the platform's push time is before `adopted_at`.
- The tag receipt workflow SHALL sign one statement with the workflow identity of the `receipt_writer` role whose subject is the full tag ref name and the tag object id.
- The tag receipt workflow SHALL take `pushed_at` from the platform's trigger context and never from a date inside the tag or commit.
- The tag receipt workflow SHALL copy every `Approved-Record-Id` trailer value of the tag message into `record_ids` byte for byte.
- The tag receipt workflow SHALL set `trailers_readable` false with an empty `record_ids` when the trailers cannot be read.
- The tag receipt workflow SHALL store the statement as one JSON Lines entry in a note on the peeled commit under `refs/notes/assurance-tag-receipts`.
- The tag receipt workflow SHALL write no second entry for a ref and tag object id the note already holds.
- The tag receipt workflow SHALL add an entry for a different tag object id.
- The tag receipt workflow SHALL NOT remove or replace an earlier entry.
- The tag receipt workflow SHALL refuse a note over 1 MiB with `tag_receipt_note_too_large`.
- The tag receipt workflow SHALL retry a rejected notes push up to five times and then fail with `tag_receipt_note_conflict`.
- The tag receipt workflow SHALL run the leak check before writing to a public repository.
- The tag receipt workflow SHALL fail with `tag_receipt_leak_check_refused` when it refuses.
- The tag receipt verifier SHALL return `untrusted_signer` unless the signer matches a `receipt_writer` entry with `source_ref_prefix` and the certificate source ref equals the statement `ref`.
- The tag receipt verifier SHALL return `wrong_subject` when the subject, `ref`, `tag_object`, `repository` or `commit` disagree.
- The tag receipt verifier SHALL return `predicate_invalid` when the statement breaks its closed schema.
- The change receipt verifier SHALL NOT accept a tag receipt.
- The tag receipt verifier SHALL NOT accept a change receipt.
- The tag receipt workflow SHALL NOT interpret a tag's meaning, signature or trailers beyond copying the trailer values.
- The tag receipt workflow SHALL NOT execute code from the repository in the signing job.
- The tag receipt workflow SHALL NOT run an agent.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-081-AC-1 | A push of an annotated tag whose name starts with a policy prefix yields one note on the peeled commit under `refs/notes/assurance-tag-receipts` with one statement whose subject is `refs/tags/<name>` and `gitTag` the tag object id, and whose predicate holds exactly the members `repository`, `ref`, `tag_object`, `commit`, `pushed_at`, `trailers_readable` and `record_ids`. | Test |
| FR-081-AC-2 | A tag name that starts with no policy prefix, and a tag pushed before `adopted_at`, end the run successfully and write no note; a receipt policy with no `tag_receipts` member writes no receipt for any tag. | Test |
| FR-081-AC-3 | A fixture tag whose tagger date is far in the future gets `pushed_at` equal to the platform's push time of the fixture trigger, not the tagger date. | Test |
| FR-081-AC-4 | A tag with two `Approved-Record-Id` trailers yields `record_ids` holding both values in message order; a tag with none yields an empty list and `trailers_readable` true; a tag whose message holds a NUL byte, more than 64 values, or a value over 256 bytes yields `trailers_readable` false and an empty list. | Test |
| FR-081-AC-5 | A tag deleted and pushed again at the same object id leaves a note with one entry and the earlier entry still verifies; a tag re-signed so its object id differs adds a second entry and leaves the first unchanged. | Test |
| FR-081-AC-6 | A note whose content would exceed 1 MiB is refused with `tag_receipt_note_too_large` and nothing is written; a fixture remote rejecting six pushes in a row fails the run with `tag_receipt_note_conflict` and leaves no partial note; two concurrent runs for two tags on one commit both end with their entries present. | Test |
| FR-081-AC-7 | A change receipt note under `refs/notes/assurance-receipts` is not read as a tag receipt and a tag receipt entry is not accepted by the change receipt verifier of FR-078; no tag receipt note appears under `refs/notes/assurance-receipts`. | Test |
| FR-081-AC-8 | A tag receipt signed through the workflow from a tag push `refs/tags/<name>` verifies against a `receipt_writer` entry with `source_ref_prefix`; the same signature checked against an entry with only `source_ref`, a certificate source ref differing from the statement `ref`, a pull request ref, another source repository, or a trigger event other than `push` yields `untrusted_signer`. | Test |
| FR-081-AC-9 | A receipt with a changed byte yields `signature_invalid`; one with an extra predicate member yields `predicate_invalid`; one whose `tag_object` differs from the subject digest, or that sits on a commit other than its `commit`, yields `wrong_subject`. | Test |
| FR-081-AC-10 | A public repository fixture whose tag trailer references a private ticket team fails the run with `tag_receipt_leak_check_refused` and writes no note; the signing job runs no repository file and no agent step. | Test |
| FR-081-AC-11 | A repository on sink `none` fails the run with `receipt_private_store_required` of FR-074 and writes no tag receipt. | Test |

## Dependencies

- **Upstream**: [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md), [FR-070](./FR-070-release-and-receipt-workflow-files.md), [FR-072](./FR-072-change-record-predicate.md), [FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md), [FR-075](./FR-075-push-to-main-receipt-workflow.md), [FR-076](./FR-076-receipt-storage-in-git-notes.md), [FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md), [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md), [NFR-008](../non-functional/NFR-008-closed-schemas.md), [NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md), [NFR-011](../non-functional/NFR-011-signing-jobs-run-no-agent.md).
- **Downstream**: [FR-082](./FR-082-detect-deleted-and-changed-tags-from-tag-receipts.md), [FR-043](./FR-043-approval-trail-of-a-change-assurance-record.md), [FR-083](./FR-083-document-the-adoption-ruleset-for-tags-notes-and-records.md).
