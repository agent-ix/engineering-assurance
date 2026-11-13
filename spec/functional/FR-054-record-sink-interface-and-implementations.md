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
in the same configuration. Restricting who can read that destination is a requirement on its configuration by the
repository's operator; Engineering Assurance neither enforces nor checks it. `none` accepts a record and keeps it
nowhere.

Out of scope: the record kinds themselves, the signing of a record ([NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md)),
the trimmed and full record forms ([FR-056](./FR-056-public-and-private-record-split.md)), the leak check
([FR-057](./FR-057-fail-closed-pre-publish-leak-check.md)), and the workflow that calls the interface (the
publishing role of [FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md)).

## Inputs

- A record in the forms of FR-056 and the repository's resolved sink selection (FR-055).

## Outputs

- One publish result `engineering-assurance.sink-result/v1` with the members `protocol`, `sink` (`public_log`,
  `private_store` or `none`), `status` (`published`, `refused` or `discarded`), `record_id`, and on `refused` the
  `error` code of the refusal. No other member exists.

## Behavior

- The sink interface SHALL expose one operation, publish, that takes a record and returns one publish result.
- The public-log sink SHALL send the trimmed public form of FR-056 and nothing else.
- The sink layer SHALL write the full private form of FR-056 to the configured private store for every publish whose resolved sink is `public_log` or `private_store`.
- The sink layer SHALL write the private form before the public-log sink sends, and only after the leak check passed.
- The private-store sink SHALL store the full private form of FR-056 and the trimmed public form it is linked to.
- The none sink SHALL send and store nothing.
- The none sink SHALL return `discarded`.
- The sink layer SHALL select exactly one implementation per publish from the repository's sink selection.
- The sink layer SHALL NOT fall back to a different implementation when the selected one refuses or fails.
- The sink layer SHALL return `refused` with a named error when a sink cannot accept the record.
- The sink layer SHALL NOT report `published` for a record that was not accepted.
- The sink layer SHALL accept a new implementation without a change to the record forms of FR-056.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-054-AC-1 | Publishing a valid record with each of the three implementations returns `published` for `public_log` and `private_store` and `discarded` for `none`, with `record_id` equal to the record's id and no member beyond the closed set; the `public_log` publish leaves the full private form in the test private store and sends only the trimmed public form to the test endpoint. | Test |
| FR-054-AC-2 | After a publish to `none`, the test public-log endpoint and the test private store hold no request and no object. | Test |
| FR-054-AC-3 | A public-log endpoint that rejects the record yields `refused` with a named error, the full private form stays in the private store, and no record is published through the private store or `none` as a fallback. | Test |
| FR-054-AC-4 | A test implementation added in the test crate and registered through the interface publishes a record with the record forms of FR-056 unchanged. | Test |
| FR-054-AC-5 | A publish result with an unknown member, or a `status` outside the three values, is rejected by the result schema. | Test |

## Dependencies

- **Upstream**: [FR-056](./FR-056-public-and-private-record-split.md),
  [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md).
- **Downstream**: the release attestations of EA-97 and the change-record receipt of EA-98, which publish through this
  interface.
