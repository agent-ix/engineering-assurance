---
id: FR-043
title: "List the audit trail of signed approvals for a change-assurance record"
type: FR
relationships:
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
that tag. The trail is read from git itself: the command keeps no separate log,
records no digest of the trail and writes nothing. A consumer such as Quoin or a
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

Out of scope: creating, pushing or deleting tags, computing the record digest,
deciding which approval is current, what a meaning permits or who may approve, any
signature rule (FR-036, FR-037, FR-040 own them), and a persisted log of the trail.

## Inputs

- One request document `engineering-assurance.approval-trail-request/v1` with the
  members `protocol`, `repository` (absolute directory path), `record_id` and
  `record_digest` (as FR-041), `allowed_signers` (absolute file path, required) and
  optionally `gitsign` (as FR-041). Unknown members are rejected. The command takes
  the document as `--request <json>` or `--request -`, as `approval-verify` does.
- `--format json|table`, default `json`.

## Outputs

- With `--format json`, one line of JSON on stdout and exit status 0, the result
  with the member `protocol` equal to `engineering-assurance.approval-trail-result/v1`:
  - `record_id`, `record_digest` and `trust` as requested (the `trust` member of
    FR-041);
  - `ordering`, the string `claimed_at_ascending`;
  - `entries`, the array of entries in order;
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
  - `complete`, true only when `truncated` is false and `unreadable` is empty. A
    trail that is not `complete` is a partial list and says so.
- An entry carries `tag` (the tag name without `refs/tags/`), `object` (the id of
  the tag object that was verified, so one signed object published under several
  ref names can be told from independent approvals), `claimed_at` (a UTC instant
  `YYYY-MM-DDTHH:MM:SSZ`, absent when the tagger date is unreadable) and every other
  member of the FR-041 result for that tag except `protocol`, `record_id`,
  `record_digest` and `trust`: `status`, `reason`, `signature`, `binding` and, on
  `approved`, `approver`, `meaning`, `signature_kind` and the signing time.
- Entries are ordered by `claimed_at` ascending; entries without one follow all
  entries with one; entries with equal or no `claimed_at` are ordered by `tag` in
  byte order. The order is therefore total and does not depend on listing order.
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
  can list, or the listing cannot be read) and `approval_trail_result_encoding_failed`.

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
| FR-043-AC-10 | A request with an unknown member, a wrong protocol, a bad record id or digest, a relative path or an invalid `gitsign` member writes `approval_trail_request_invalid`; unreadable JSON and an invalid allowed-signers file write their codes; a path that is not a git repository writes `approval_trail_repository_unavailable`; each exits 2. | Test |
| FR-043-AC-11 | A gitsign-signed tag naming the record, through the stand-in, is listed `approved` with `signature_kind` `gitsign`, and one the stand-in rejects is listed `untrusted_signer`. | Test (stand-in) |
| FR-043-AC-12 | The ordering and the record-naming test are exercised by pure functions alone for equal, missing and out-of-order times and for repeated, different and case-variant ids. | Test |
| FR-043-AC-13 | The total time bound is 300 seconds and each run uses the shared process host's time and output bounds. | Inspection |
| FR-043-AC-14 | An annotated tag whose ref name holds a byte that is not valid UTF-8 is listed in `unreadable` with `tag` escaped as `\xNN`, `tag_escaped` true, `tag_hex` equal to the raw name bytes and reason `tag_name_not_utf8`, and the result is not `complete`. | Test |
| FR-043-AC-15 | A tag whose message exceeds the output bound and which names the record with a `revoked` meaning is listed in `unreadable` with reason `output_exceeded`, matching the `unverifiable` reason `approval-verify` gives for it, and the result is not `complete`. | Test |
| FR-043-AC-16 | With the total time already expired, the result has no entries, `truncated` true, `truncation` `time_limit`, `considered` equal to the number of refs reached before the stop, and `complete` false. | Test |
| FR-043-AC-17 | Every entry carries `object` equal to the id of the verified tag object, and two ref names of one tag object carry the same `object`. | Test |
| FR-043-AC-18 | In `--format table`, escape, C1 control, bidirectional override and zero-width characters in a tag name or meaning are replaced by `?`. | Test |
| FR-043-AC-19 | The command writes nothing to the repository and keeps no log or digest of the trail. | Inspection |

## Dependencies

- **Upstream**: [FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md),
  [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md),
  [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md),
  [FR-040](./FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md).
- **Downstream**: Quoin's change-assurance verification and the dossier release-record
  document consume the machine result.
