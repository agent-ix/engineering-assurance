---
id: FR-043
title: "List the audit trail of signed approvals for a change-assurance record"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-081"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-082"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-041"
    type: "references"
---

# FR-043: List the audit trail of signed approvals for a change-assurance record

## Description

Engineering Assurance SHALL provide an `approval-trail` command that lists, oldest
first, every annotated tag in a repository whose message names one
change-assurance record, each with the full `approval-verify` result of FR-041 for
that tag. It also lists a current tag covered by the receipt policy when its
readable trailers do not name the requested record and no verified receipt names
its current object: that entry has status `tag_no_receipt` and `tag_receipt` `none`.
This keeps a missing receipt visible even when the tag belongs to another record.
The trail is read from git itself and from the tag receipts CI wrote
([FR-081](./FR-081-tag-receipt-on-approval-and-release-tag-push.md)): the command keeps no log of its own,
records no digest of the trail and writes nothing; its evidence is git's tag refs and the receipt set. A consumer such as Quoin or a
release-record dossier retains the machine result as data and never re-verifies it.

Nothing is hidden. A tag that names the record is listed whatever its verdict:
`approved`, `wrong_record` (a different digest), `untrusted_signer` (an unmapped
signer), `meaning_missing`, `signed_bad`, `unsigned` and `unverifiable` entries all
appear, including an approval a later tag supersedes or revokes. A revocation is
only a `Signed-off-meaning` string, listed like any other meaning; the command
attaches no semantics to a meaning and decides neither which approval stands nor
whether the record is currently approved.

A tag names the record when its `Approved-Record-Id` trailer equals the requested
`record_id`. The command reads the trailers of every annotated tag with the isolation
FR-041 uses and verifies only the tags that name the record. One rule covers every
annotated tag whose trailers cannot be read, for any reason (the message holds a NUL
byte or is not valid UTF-8, git cannot parse it, a git run times out or exceeds the
output bound, the ref cannot be resolved or its object read, or the ref name is not
valid UTF-8): such a tag cannot be known to name the record, so it is never counted
anonymously and never dropped. It is listed by name in the `unreadable` array with
the reason, and the result is not `complete`.

The order is by the time each tag claims. The key is the tagger date of FR-041,
which the signer wrote into the signed payload: it is the signer's own claim, not
an independent attestation, and the trail reports it on every entry as `claimed_at`
whatever the verdict, so that an entry without an accepted signature still has its
place. `claimed_at` on a status other than `approved` attests nothing; the
`signed_at` members of FR-041 stay on `approved` entries only.

Deleted and changed tags. A deleted tag cannot be seen in the tag set, so the command also
reads the tag receipts under `refs/notes/assurance-tag-receipts` and verifies each under
[FR-081](./FR-081-tag-receipt-on-approval-and-release-tag-push.md) (which applies [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md)).
It applies the comparison of [FR-082](./FR-082-detect-deleted-and-changed-tags-from-tag-receipts.md) to the receipts that belong
to the requested record. A receipt belongs to the record when its `record_ids` holds the requested `record_id`
byte for byte; a receipt with `trailers_readable` false belongs to no record by content and is therefore reported in the
trail of every record, with `attributed` false. The `record_ids` are the tag's trailer values captured at push
time, so a tag that no longer exists can still be attributed. A receipt whose tag is still present and still the
receipted object is not a finding: the tag's own entry carries `tag_receipt` `verified`. A tag deleted and pushed
again at the identical object id passes, because the object id matches; a tag re-signed or re-created has a different
object id, and the earlier receipt is listed as `missing_tag` with reason `tag_changed`. The command reports and never
decides whether a changed tag was legitimate; a person records acceptance of a specific finding in the `accepted_tags`
list of the receipt policy ([FR-082](./FR-082-detect-deleted-and-changed-tags-from-tag-receipts.md)), and a matched `missing_tag` finding is listed with the status `accepted`, `finding` `missing_tag` and its other members, `accepted_by` and `accepted_reason`, exactly as FR-082 reports it. An absent or changed tag is never `approved`: it is listed with the status
`missing_tag`. A receipt that does not verify is listed with the status `invalid_receipt` and never dropped. An invalid receipt entry carries its note `commit` and `reason`, has no `ref` or tag name, and has `attributed` false because its content is untrusted. A current tag whose name starts with a policy prefix and has no verified receipt for its current object id carries `tag_receipt` `none` and counts against `complete` unless accepted, including when its readable trailers do not name the requested record. A policy file without a `tag_receipts` member is treated as an empty tag-receipt policy: it has no prefixes and accepts nothing, but receipts already written are still verified and compared. A notes ref that does not exist is an empty receipt set: `checked` is true and every such tag is `none`.

Out of scope: creating, pushing or deleting tags, writing a receipt (FR-081 owns it), computing the record digest,
deciding which approval is current, what a meaning permits or who may approve, any
signature rule (FR-036, FR-037, FR-040 own them), and any log or store that the command itself writes.

## Inputs

- One request document `engineering-assurance.approval-trail-request/v1` with the
  members `protocol`, `repository` (absolute directory path), `record_id` and
  `record_digest` (as FR-041), `allowed_signers` (required: an absolute file path or a `<rev>:<path>` trust source, as FR-041) and
  optionally `tag_receipts`, an object with the member `workflow_identities` (absolute path of the workflow-identity policy file of FR-064, read by the caller from the protected base branch) and `receipt_policy` (absolute path of the receipt policy file of FR-075, read by the caller from the protected base branch, whose `tag_receipts` member gives the prefixes and `accepted_tags`), and optionally `gitsign` with its `identity_registry` (as FR-041, whose `gitsign.principal` and registry refusals use the codes `approval_trail_gitsign_principal_refused` and `approval_trail_identity_registry_invalid`). Unknown members are rejected. The command takes
  the document as `--request <json>` or `--request -`, as `approval-verify` does.
- `--format json|table`, default `json`.

## Outputs

- With `--format json`, one line of JSON on stdout and exit status 0, the result
  with the member `protocol` equal to `engineering-assurance.approval-trail-result/v1`:
  - `record_id`, `record_digest` and `trust` as requested (the `trust` member of
    FR-041, with `allowed_signers_sha256`, `allowed_signers_commit` for a
    `<rev>:<path>` source, `ea_version` and `git_version` as FR-041 defines (a git failure while the allowed-signers `<rev>:<path>` source is read is `approval_trail_repository_unavailable`, so there is no result and no `trust`), read once for the
    whole trail, so every entry is verified against the same bytes, and with
    `gitsign` also `identity_registry_commit` whenever the revision resolved);
  - `ordering`, the string `claimed_at_ascending`;
  - `entries`, the array of entries in order;
  - `tag_receipts`, an object with `checked` (true when the request carried a `tag_receipts` member and the notes ref was read or found absent, false otherwise; with false no receipt was consulted and `complete` is false) and `considered`, the number of receipt entries read;
  - `unreadable`, the array of annotated tags that cannot be known to name the
    record, in listing order (the byte order of the ref name). Each item has `tag`
    (the name; every byte that is not valid UTF-8 and every control character is
    written `\xNN`), `tag_escaped` (true, present only when `tag` was escaped),
    `tag_hex` (the raw name bytes in lower-case hexadecimal, present only when
    `tag_escaped` is true) and `reason`, one of the FR-036 and FR-041 reasons for
    an unreadable tag (`trailer_unreadable`, `output_exceeded`, `timeout`,
    `revision_unresolved`, `git_not_started`, `unrecognized_output`) or
    `tag_name_not_utf8`;
  - `considered`, the number of refs under `refs/tags` reached, in listing order,
    before the command stopped: all examined refs when the trail ran to the end or
    to the tag limit, and only the refs before the stop after a `time_limit`;
    `truncated`, a boolean; and, only when `truncated` is true, `truncation`, one of
    `tag_limit` or `time_limit`;
  - `complete`, true only when `truncated` is false, `unreadable` is empty, `tag_receipts.checked` is true, no entry has the status `missing_tag` or `invalid_receipt`, and no entry for a tag whose name starts with a policy prefix has `tag_receipt` `none` unless an `accepted_tags` entry accepts it. Accepted entries do not count against `complete`. A
    trail that is not `complete` is a partial list and says so.
- An approval entry carries `tag` (the tag name without `refs/tags/`), `object` (the id of
  the tag object that was verified, so one signed object published under several
  ref names can be told from independent approvals), `claimed_at` (a UTC instant
  `YYYY-MM-DDTHH:MM:SSZ`, absent when the tagger date is unreadable) and every other
  member of the FR-041 result for that tag except `protocol`, `record_id`,
  `record_digest` and `trust`: `status`, `reason`, `signature`, `binding` and, on
  `approved`, `approver`, `meaning`, `signature_kind` and the signing times of FR-041
  (`signed_at` or `signed_at_missing`, `trusted_at` with `trusted_at_source`
  or `trusted_at_missing`, and `time_skew`).
- An approval entry for a present tag also carries `tag_receipt`: `verified` when a verified receipt names its ref and object id, otherwise `none` (with `accepted_by` and `accepted_reason` when an `accepted_tags` entry accepts it as FR-082 matches a `tag_no_receipt` finding). A `tag_no_receipt` entry carries `tag`, `object` (the current tag object id), status `tag_no_receipt` or `accepted`, `tag_receipt` `none`, and, when accepted, `finding` `tag_no_receipt`, `accepted_by` and `accepted_reason`; it has no approval-verification fields. An entry with the status `missing_tag` carries `tag`, `object` (the receipted object id), `reason` (`tag_absent` or `tag_changed`), `current_object` (only for `tag_changed`), `commit`, `pushed_at` and `attributed` (false only for a receipt with `trailers_readable` false); it has no `claimed_at`, `signature` or `approver`. An entry with the status `invalid_receipt` carries `commit`, `reason` (`signature_invalid`, `untrusted_signer`, `predicate_invalid` or `wrong_subject`) and `attributed` false, and no `ref` or tag name, because an unverified receipt's content is not trusted.
- Entries are ordered by `claimed_at` ascending; entries without one follow all
  entries with one; entries with equal or no `claimed_at` are ordered by `tag` in
  byte order, then `object`, then `commit` (an entry without `tag` follows those with one). The order is therefore total and does not depend on listing order.
- With `--format table`, the human-readable rendering of the same result on stdout
  and exit status 0: a header row, one row per entry in the same order and then one
  row per `unreadable` item (status `unreadable`), with the columns `CLAIMED_AT`,
  `STATUS`, `REASON`, `APPROVER`, `MEANING`, `TAG` (`-` for an absent value; multiple
  meanings joined by `; `; every control character and every Unicode format or
  line-separator character, such as the bidirectional controls U+202A to U+202E and
  U+2066 to U+2069, replaced by `?`), and a final line
  `considered N, unreadable N, truncated yes|no, complete yes|no`. The table is for
  people and is not a machine contract.
- A request that yields no result writes the `engineering-assurance.error/v1` machine
  error for capability `approval-trail` to stdout and its message to stderr, and
  exits with status 2. The codes are `approval_trail_request_unreadable`,
  `approval_trail_request_invalid`, `approval_trail_allowed_signers_invalid`,
  `approval_trail_repository_unavailable` (the repository is not a directory git
  can list, the listing cannot be read, or git fails while the identity registry
  is read, FR-037), `approval_trail_gitsign_principal_refused`,
  `approval_trail_identity_registry_invalid` and
  `approval_trail_result_encoding_failed`.

## Behavior

- The command SHALL list tags with `git for-each-ref` over `refs/tags` using the
  format `%(refname)%00%(objecttype)` and no message text, and SHALL examine them in
  the byte order of the ref name, which is git's listing order.
- The command SHALL consider a listed ref only when its object type is `tag`;
  lightweight tags (object type `commit`) and refs to other objects are ignored.
- The command SHALL first load each candidate (resolve the full ref name
  `refs/tags/<name>` to an object id, read the tag object and read its trailers with
  the isolation FR-041 uses) and SHALL verify only the tags that name the record,
  against the object id already resolved.
- The command SHALL NOT restate, relax or re-implement any rule of FR-036, FR-037,
  FR-040 or FR-041.
- The command SHALL list a tag when any `Approved-Record-Id` trailer value equals the
  requested `record_id` byte for byte; a tag repeating the trailer is listed with
  binding `repeated`.
- The command SHALL exclude every readable tag that does not name the record,
  including tags naming a different record.
- The command SHALL list in `unreadable` every annotated tag whose trailers cannot
  be read, and SHALL NOT report such a tag only as a count or omit it.
- The command SHALL list entries of every status.
- The command SHALL NOT omit, merge or collapse an entry because a later tag
  supersedes it or carries a revoking meaning.
- The command SHALL take `claimed_at` only from the tagger line as FR-041 reads it.
- The command SHALL NOT default `claimed_at`.
- The command SHALL NOT use verification time, commit date or author date for
  `claimed_at`.
- The command SHALL examine at most 10000 refs under `refs/tags`, in git's ref-name
  order, and report `truncated` true with `truncation` `tag_limit` when more exist.
- The command SHALL start no new tag load after 300 seconds of total time, and
  report `truncated` true with `truncation` `time_limit`; each tag's git,
  `ssh-keygen` and `gitsign` runs keep the bounds of the shared process host. The
  refs a `time_limit` result holds are those before the stop in listing order, so
  they are determined by the order and the stop point alone.
- The command SHALL pass a tag name to git only as part of the full
  `refs/tags/<name>` ref, or after `--` or `--end-of-options`, so that an
  option-like name such as `--help` or a name with characters a shell would treat
  specially is handled as data.
- The command SHALL NOT pass a tag name to a shell.
- The command SHALL read the tag receipts under `refs/notes/assurance-tag-receipts`, verify each under FR-081, and apply the comparison of FR-082 to the receipts that belong to the requested record.
- The command SHALL list a receipt whose ref is absent, or names a different object id, with the status `missing_tag`.
- The command SHALL NOT list a receipt whose tag is absent or changed as `approved`.
- The command SHALL list a receipt that does not verify with the status `invalid_receipt`.
- The command SHALL list a current policy-prefix tag with no verified receipt for its object as `tag_no_receipt` with `tag_receipt` `none`, even when readable trailers do not name the requested record.
- The command SHALL treat a receipt policy without `tag_receipts` as an empty policy with no prefixes and no accepted tags, while still comparing receipts that are present.
- An `invalid_receipt` entry SHALL carry its note commit and reason, and SHALL NOT carry a ref or tag name.
- The command SHALL NOT drop a receipt that does not verify.
- The command SHALL report a receipt with `trailers_readable` false in the trail of every record with `attributed` false.
- The command SHALL report `complete` false while any `missing_tag` or `invalid_receipt` entry exists, any prefix-matching present tag has `tag_receipt` `none`, or `tag_receipts.checked` is false, except for findings an `accepted_tags` entry accepts.
- The command SHALL read an absent notes ref as an empty receipt set with `checked` true.
- The command SHALL report a finding that an `accepted_tags` entry matches with the status `accepted`, `finding` naming the replaced status, `accepted_by` and `accepted_reason`.
- The command SHALL NOT count a finding that an `accepted_tags` entry matches against `complete`.
- The command SHALL NOT decide whether an absent or changed tag was legitimate.
- The command SHALL NOT write to the repository, create a log, or record a digest of
  the trail.
- The command SHALL keep every hardening rule of FR-036, FR-037, FR-040 and FR-041
  (absolute-`PATH` program resolution, pinned `gpg` programs, refusal of a repository
  that sets a `gitsign.*` key, repository-configuration isolation) by verifying
  through the same entry points.
- The trail order and the test of whether a tag names the record SHALL be a
  pure function with no I/O.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-043-CON-1 | An entry SHALL carry exactly the FR-041 status and signature verdict that `approval-verify` yields for the same tag, request and trust input. | Safety | Test |
| FR-043-CON-2 | The command SHALL contain no signature, certificate, trailer-parsing or approval-decision code of its own, and no rule of FR-036, FR-037, FR-040 or FR-041 restated. | Architecture | Inspection |
| FR-043-CON-3 | A result SHALL NOT omit an annotated tag that names the record or could name it: such a tag is an entry or an `unreadable` item unless `truncated` is true. | Safety | Test |

## Acceptance Criteria

Stand-in note: where `gitsign` is not installed, criteria that need it are verified
with the FR-040 stand-in executable; the SSH criteria use real signatures from
throwaway keys generated in the test.

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-043-AC-1 | Two validly signed approvals of one record, one with meaning `approved` and a later one with meaning `revoked`, are both listed oldest first by `claimed_at`, each `approved` with its approver, meaning and signing time, whatever the order the tags were created or named in. | Test |
| FR-043-AC-2 | A tag with a bad signature, a tag signed by an unmapped key, a tag with a different digest and a tag without `Signed-off-meaning`, all naming the record, are listed with `signed_bad`, `untrusted_signer`, `wrong_record` and `meaning_missing`, and none is dropped. | Test |
| FR-043-AC-3 | Tags naming a different record are excluded; lightweight tags and annotated tags without an `Approved-Record-Id` trailer are not listed. | Test |
| FR-043-AC-4 | Each entry equals the `approval-verify` result for the same tag, request and trust input, less `protocol`, `record_id`, `record_digest` and `trust`, and plus `claimed_at` and `object`. | Test |
| FR-043-AC-5 | Entries are ordered by `claimed_at` ascending; entries without a readable tagger date follow, ordered by tag name in byte order, as do equal times; `claimed_at` is reported on entries of every status. | Test |
| FR-043-AC-6 | More than 10000 refs under `refs/tags` yields `truncated` true with `truncation` `tag_limit` and `considered` 10000; a trail within the bound yields `truncated` false and no `truncation`. | Test |
| FR-043-AC-7 | A tag named `--help`, one named like a revision expression and one with characters such as `'`, `"`, `$` and `;` are listed correctly, executed nothing and altered no other entry. | Test |
| FR-043-AC-8 | A validly signed tag whose message holds a NUL byte is listed in `unreadable` with its name and reason `trailer_unreadable`, is not in `entries`, and the result is not `complete`. | Test |
| FR-043-AC-9 | `--format table` prints a header, one row per entry in trail order with status, reason, approver, meaning, claimed time and tag, one `unreadable` row per unreadable tag, and the final `considered`, `unreadable`, `truncated`, `complete` line, and exits 0. | Test |
| FR-043-AC-10 | A request with an unknown member, a wrong protocol, a bad record id or digest, a relative path or an invalid `gitsign` member writes `approval_trail_request_invalid`; unreadable JSON and an invalid allowed-signers file write their codes; a `gitsign` member that carries `principal` in a request that otherwise validates writes `approval_trail_gitsign_principal_refused`; an `identity_registry` whose source is `invalid` or whose content is unusable writes `approval_trail_identity_registry_invalid`; a path that is not a git repository, with or without `gitsign`, writes `approval_trail_repository_unavailable`, never `approval_trail_identity_registry_invalid`; each exits 2. | Test |
| FR-043-AC-11 | A gitsign-signed tag naming the record, through the stand-in, is listed `approved` with `signature_kind` `gitsign`, and one the stand-in rejects, or whose identity the identity registry does not name, is listed `untrusted_signer`. | Test (stand-in) |
| FR-043-AC-12 | The ordering and the record-naming test are exercised by pure functions alone for equal, missing and out-of-order times and for repeated, different and case-variant ids. | Test |
| FR-043-AC-13 | The total time bound is 300 seconds and each git and signature run uses the shared host's time and output bounds. | Inspection |
| FR-043-AC-14 | An annotated tag whose ref name holds a byte that is not valid UTF-8 is listed in `unreadable` with `tag` escaped as `\xNN`, `tag_escaped` true, `tag_hex` equal to the raw name bytes and reason `tag_name_not_utf8`, and the result is not `complete`. | Test |
| FR-043-AC-15 | A tag whose message exceeds the output bound and which names the record with a `revoked` meaning is listed in `unreadable` with reason `output_exceeded`, matching the `unverifiable` reason `approval-verify` gives for it, and the result is not `complete`. | Test |
| FR-043-AC-16 | With the total time already expired, the result has no entries, `truncated` true, `truncation` `time_limit`, `considered` equal to the number of refs reached before the stop, and `complete` false. | Test |
| FR-043-AC-17 | Every entry carries `object` equal to the id of the verified tag object, and two ref names of one tag object carry the same `object`. | Test |
| FR-043-AC-18 | In `--format table`, escape, C1 control, bidirectional override and zero-width characters in a tag name or meaning are replaced by `?`. | Test |
| FR-043-AC-19 | The command writes nothing to the repository and keeps no log or digest of the trail. | Inspection |
| FR-043-AC-26 | The trail result's `trust` carries `allowed_signers_sha256`, `ea_version` and `git_version` on every result that carries `trust`, and `allowed_signers_commit` when the request's `allowed_signers` is a `<rev>:<path>` that resolved; the allowed-signers bytes are read once per trail, so a file changed mid-run changes no entry, and each entry equals the `approval-verify` result against the same bytes. | Test |
| FR-043-AC-27 | An `approved` gitsign entry carries the `trusted_at`, `trusted_at_source` and `time_skew`, or the `trusted_at_missing` reason, that `approval-verify` yields for the same tag; an `approved` SSH entry carries `trusted_at_missing` `ssh_signature`; entries other than `approved` carry none of them; and `claimed_at` alone orders the trail whatever `trusted_at` says. | Test |
| FR-043-AC-20 | A fixture record with a verified tag receipt whose tag has been deleted yields an entry with the status `missing_tag`, reason `tag_absent`, the receipted object id, and `complete` false; the entry is never `approved`. | Test |
| FR-043-AC-21 | A tag deleted and pushed again at the identical object id yields its normal entry with `tag_receipt` `verified`, no `missing_tag` entry, and `complete` true when nothing else is open; the same tag re-signed yields its new entry with `tag_receipt` `none` and a `missing_tag` entry with reason `tag_changed` and `current_object` for the earlier receipt. | Test |
| FR-043-AC-22 | A receipt with a changed byte and a receipt signed by an unlisted identity yield `invalid_receipt` entries with `signature_invalid` and `untrusted_signer`, and `complete` false. | Test |
| FR-043-AC-23 | A receipt whose `record_ids` holds only another record's id is not listed for the requested record; a receipt with `trailers_readable` false for an absent tag is listed with `attributed` false in the trail of each of two different records. | Test |
| FR-043-AC-24 | A request without a `tag_receipts` member yields `tag_receipts.checked` false and `complete` false and consults no receipt; a `tag_receipts` member with a relative path or an unknown member is refused with `approval_trail_request_invalid`. | Test |
| FR-043-AC-25 | The fixture trail's `missing_tag` and `invalid_receipt` entries equal those `receipt-missing` lists for the same receipts and tag set (FR-082), and the command writes nothing to the repository. | Test |
| FR-043-AC-28 | With `tag_receipts` given and the notes ref absent, `checked` is true, no error is raised, each tag whose name starts with a policy prefix has `tag_receipt` `none`, and `complete` is false; the same request with an `accepted_tags` entry for each such tag yields `complete` true. | Test |
| FR-043-AC-29 | A `missing_tag` finding matched by an `accepted_tags` entry on `ref` and `tag_object` is listed with the status `accepted`, `finding` `missing_tag`, `accepted_by` and `accepted_reason` and does not make `complete` false; an entry that differs in either member changes nothing; an `invalid_receipt` entry is never accepted. | Test |
| FR-043-AC-30 | A current policy-prefix tag with readable trailers naming another record and no receipt is listed for the requested record with status `tag_no_receipt` and `tag_receipt` `none`, and makes the trail incomplete; an exact `accepted_tags` entry changes it to `accepted` with `finding` `tag_no_receipt` and leaves the trail complete. | Test |
| FR-043-AC-31 | Receipt verification tries the public-good root with log inclusion required, then the GitHub private-instance root with timestamp-only evidence, against every `receipt_writer` identity carrying `source_ref_prefix`; an invalid receipt entry carries `commit` and `reason` without an untrusted `ref`. | Inspection / Test |
| FR-043-AC-32 | A policy without `tag_receipts` is empty, but receipts already present are still verified and compared; the result reports `tag_receipts.checked` true. | Test |

## Dependencies

- **Upstream**: [FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md),
  [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md),
  [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md),
  [FR-040](./FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md),
  [FR-081](./FR-081-tag-receipt-on-approval-and-release-tag-push.md),
  [FR-082](./FR-082-detect-deleted-and-changed-tags-from-tag-receipts.md).
- **Downstream**: Quoin's change-assurance verification and the dossier release-record
  document consume the machine result.
