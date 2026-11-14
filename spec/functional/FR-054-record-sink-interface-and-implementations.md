---
id: FR-054
title: "Publish an assurance record through a sink interface with a public log, a private store and a none implementation"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-055"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-056"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-057"
    type: "references"
---

# FR-054: Publish an assurance record through a sink interface with a public log, a private store and a none implementation

## Description

Tracked by Linear EA-105. A record sink is the place an assurance record goes after it is made. Engineering Assurance
owns one sink interface with three implementations: the public transparency log, the private store, and none.
Later record kinds (the release attestations of EA-97 and the change-record receipt of EA-98) publish through this
interface and add no sink of their own. Approval statements ([FR-058](./FR-058-approval-statement-and-predicate.md))
are stored and published through it as well, with the same split and the same leak check.

The public log is an append-only transparency log reached at an endpoint named in the sink configuration
([FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md)). The private store is a destination named
in the same configuration. For the concrete Git private store, the destination is a separate private Git repository,
not the publishing repository, and its records are kept on the fixed ref
`refs/heads/engineering-assurance-private-records`. Restricting who can read that destination is a requirement on its
configuration by the repository's operator; Engineering Assurance neither enforces nor checks it. `none` accepts a
record and keeps it nowhere.

The Git private store writes each FR-056 private form as UTF-8 RFC 8785 canonical JSON at
`records/<owner-hex>/<repository-hex>/<record-id>/private.json`, where each of `owner-hex` and `repository-hex` is the
lowercase hexadecimal encoding of the UTF-8 bytes of that exact component of `owner/name` from the public form's
`subject.id`, and `record-id` is its canonical lowercase UUID. Encoding each component makes every path segment safe,
including the valid values `.` and `..`, and reversible without ambiguity. For a private-store sink, it writes the
linked public form separately as UTF-8 RFC 8785 canonical JSON at the sibling path `public.json`. After the selected
sink returns `published` or `refused`, the sink layer writes an outcome marker as UTF-8 RFC 8785 canonical JSON at
`outcome.json`, with exactly the members `protocol` (`engineering-assurance.private-store-outcome/v1`), `record_id`
(the record's lowercase UUID), `sink` (`public_log` or `private_store`) and `outcome` (`published` or `refused`). These files contain the existing FR-056
forms and this minimal outcome marker, not a second copy of the approval statement or a new record format. The private
form's approval statement object and exact signature-bundle string are preserved; canonicalizing its nested
`statement` object recreates the exact RFC 8785 bytes that FR-058 passed to the signer. A public-log sink writes no
public form to the Git private store. The source repository's refs, including its receipt notes ref, are never used for
this private data.

Out of scope: the record kinds themselves, the signing of a record ([NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md)),
the trimmed and full record forms ([FR-056](./FR-056-public-and-private-record-split.md)), the leak check
([FR-057](./FR-057-fail-closed-pre-publish-leak-check.md)), and the workflow that calls the interface (the
publishing role of [FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md)).

## Inputs

- A record in the forms of FR-056 and the repository's resolved sink selection (FR-055).

## Outputs

- One publish result `engineering-assurance.sink-result/v1` with the members `protocol`, `sink` (`public_log`,
  `private_store` or `none`), `status` (`published`, `refused` or `discarded`), `record_id`, and on `refused` the
  `error` code of the refusal, including `private_store_outcome_write_failed`,
  `private_store_outcome_incomplete` or `private_store_record_previously_refused` when applicable. No other
  member exists.

## Behavior

- The sink interface SHALL expose one operation, publish, that takes a record and returns one publish result.
- The public-log sink SHALL send the trimmed public form of FR-056 and nothing else.
- The sink layer SHALL write the full private form of FR-056 to the configured private store for every publish whose resolved sink is `public_log` or `private_store`.
- The sink layer SHALL write the private form first, before the leak check and before any send.
- The sink layer SHALL fail the publish when the private store write fails.
- The sink layer SHALL persist the selected sink's final `published` or `refused` outcome in the private store after
  the selected sink returns.
- The sink layer SHALL return `refused` with `private_store_outcome_write_failed` when it cannot persist that outcome.
- The sink layer SHALL NOT retry a selected sink after outcome persistence fails.
- The private store's `write_private` operation SHALL atomically prepare the destination record using the complete
  record forms and resolved sink, and SHALL return `NewRecord`, `AlreadyPublished` or `AlreadyRefused` before the sink
  runs.
- The private store SHALL expose `write_publish_outcome(destination, record_id, sink, outcome)` to persist the final
  selected-sink result after publication.
- When `write_private` returns `AlreadyPublished`, the sink layer SHALL return the stored `published` result without
  invoking the selected sink.
- When `write_private` returns `AlreadyRefused`, the sink layer SHALL return `refused` with
  `private_store_record_previously_refused` without invoking the selected sink.
- When `write_private` finds an existing private form without a valid terminal outcome marker, the sink layer SHALL
  return `refused` with `private_store_outcome_incomplete` without invoking the selected sink.
- When a prior selected-sink call has no valid terminal marker, the sink layer SHALL permit only a retry of outcome
  persistence when the caller retains the exact prior result; it SHALL NOT invoke the selected sink again for that
  record ID.
- When preparation finds private bytes that differ from the proposed private form, or an inconsistent public-form
  file for the stored sink and outcome, the private store SHALL fail closed before the selected sink is invoked.
- The none sink SHALL NOT write an outcome marker because it retains no record.
- The private-store sink SHALL store the full private form of FR-056 and the trimmed public form it is linked to.
- The none sink SHALL send and store nothing.
- The none sink SHALL return `discarded`.
- The sink layer SHALL select exactly one implementation per publish from the repository's sink selection.
- The sink layer SHALL NOT fall back to a different implementation when the selected one refuses or fails.
- The sink layer SHALL return `refused` with a named error when a sink cannot accept the record.
- The sink layer SHALL NOT report `published` for a record that was not accepted.
- The sink layer SHALL accept a new implementation without a change to the record forms of FR-056.
- The Git private store SHALL write private forms and any private-store public forms only to the configured separate
  private Git repository and the fixed ref `refs/heads/engineering-assurance-private-records`.
- The Git private store SHALL create one `private.json` file for each record at the path defined in this requirement.
- The Git private store SHALL create the sibling `public.json` only when the selected sink is `private_store`.
- The Git private store SHALL create a sibling `outcome.json` containing the final selected sink and `published` or
  `refused` result for each retained record.
- The Git private store SHALL encode that marker as `engineering-assurance.private-store-outcome/v1` with exactly
  `protocol`, `record_id`, `sink` and `outcome` members.
- The Git private store SHALL write `outcome.json` after the selected sink returns its final result.
- When the same final sink and outcome are written again for a record ID, the Git private store SHALL treat the marker
  write as successful without changing the file.
- When a different final sink or outcome is written for a record ID with a valid marker, the Git private store SHALL
  return `private_store_outcome_write_failed` without changing the file.
- When `write_private` finds a private form and a valid marker for the same record ID, the Git private store SHALL
  return `AlreadyPublished` or `AlreadyRefused` according to the marker without invoking any sink.
- When `write_private` finds a private form without a valid terminal marker or finds public-form files inconsistent with
  the marker's sink and outcome, the Git private store SHALL fail with `private_store_outcome_incomplete`.
- When `write_private` finds different private-form bytes for an existing record ID, the Git private store SHALL fail
  with `private_store_write_failed` and preserve the existing files.
- When `outcome.json` is missing or malformed for a retained record, the approval reader SHALL fail the lookup as
  unreadable instead of treating the record as published or refused.
- When a file with the same record ID already contains identical bytes, the Git private store SHALL treat the write as
  successful without changing the stored file.
- When a file with the same record ID contains different bytes, the Git private store SHALL return
  `private_store_write_failed` without changing the stored file.
- Concurrent writes to different record IDs SHALL preserve both records after bounded push retries.
- When a push fails because the remote ref has advanced, the Git private store SHALL fetch the latest ref, treat an
  identical record file there as a successful idempotent write, and otherwise reapply its write.
- The Git private store SHALL make no more than five push attempts for one write.
- When five push attempts fail and the refreshed target ref lacks the record file, the Git private store SHALL return
  `private_store_write_failed` without force-pushing.
- The Git private store SHALL NOT force-push or delete a record file.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-054-AC-1 | Publishing a valid record with each of the three implementations returns `published` for `public_log` and `private_store` and `discarded` for `none`, with `record_id` equal to the record's id and no member beyond the closed set; the `public_log` publish leaves the full private form in the test private store and sends only the trimmed public form to the test endpoint. | Test |
| FR-054-AC-2 | After a publish to `none`, the test public-log endpoint and the test private store hold no request and no object. | Test |
| FR-054-AC-3 | A public-log endpoint that rejects the record yields `refused` with a named error, the full private form and `refused` outcome stay in the private store, and no record is published through the private store or `none` as a fallback. | Test |
| FR-054-AC-4 | A test implementation added in the test crate and registered through the interface publishes a record with the record forms of FR-056 unchanged. | Test |
| FR-054-AC-5 | A publish result with an unknown member, or a `status` outside the three values, is rejected by the result schema. | Test |
| FR-054-AC-6 | A record that the leak check refuses still has its full private form in the test private store, and nothing is sent to the test public log; a failing private store write fails the publish with a named error before the leak check runs and nothing is sent. | Test |
| FR-054-AC-7 | For a fixture public source repository with the Git private-store destination configured, publishing its record creates the private-form file at the specified path on `refs/heads/engineering-assurance-private-records`; no private-form bytes or source-repository notes are written to the public source repository. | Test |
| FR-054-AC-8 | A private-store publish creates the matching `public.json` beside `private.json`; a public-log publish does not create `public.json` in the private Git store; both non-`none` paths persist the selected sink's exact final outcome in `outcome.json`. | Test |
| FR-054-AC-9 | Reapplying a Git file write after a non-fast-forward update with the same bytes succeeds without changing stored bytes; different bytes for the same record path fail with `private_store_write_failed` and preserve the original. | Test |
| FR-054-AC-10 | Two concurrent writes with different record IDs both remain readable after retrying a non-fast-forward push; after five failed push attempts and a refreshed target ref that lacks the record file, the write fails with `private_store_write_failed` without force-pushing or deleting a record. | Test |
| FR-054-AC-11 | A record for a source repository whose owner or repository component is `.` or `..` is stored under the lowercase-hex path component for that exact UTF-8 value, and the resulting path contains no `.` or `..` directory component. | Test |
| FR-054-AC-12 | A stored approval private form is UTF-8 RFC 8785 canonical JSON; canonicalizing its nested statement object reproduces byte-for-byte the statement bytes passed to the signer, and decoding it preserves the exact signature-bundle string. | Test |
| FR-054-AC-13 | After a public-log success, private-store success, or sink refusal, `outcome.json` has exactly the specified protocol, record ID, sink and outcome; a failed outcome-marker write yields `refused` with `private_store_outcome_write_failed`. The `none` sink creates no private-store files. | Test |
| FR-054-AC-14 | Writing the same final sink and outcome again for a record ID succeeds without changing the marker; attempting to replace it with a different sink or outcome fails with `private_store_outcome_write_failed` and preserves the original marker. | Test |
| FR-054-AC-15 | Replaying a record with an identical private form and a valid `published` marker returns `published` without invoking its sink; replaying one with a valid `refused` marker returns `refused` with `private_store_record_previously_refused` without invoking its sink. | Test |
| FR-054-AC-16 | A prior private form without a valid terminal marker, private bytes conflicting with the proposed form, or a public-form file inconsistent with the stored sink/outcome fails closed before the selected sink is invoked. A fresh record ID is required for a new publish attempt. | Test |

## Dependencies

- **Upstream**: [FR-056](./FR-056-public-and-private-record-split.md),
  [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md).
- **Downstream**: the release attestations of EA-97 and the change-record receipt of EA-98, which publish through this
  interface.
