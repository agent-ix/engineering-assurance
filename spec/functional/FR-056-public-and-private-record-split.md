---
id: FR-056
title: "Split a record into a trimmed public form and a full private form linked by id"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-054"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-008"
    type: "references"
---

# FR-056: Split a record into a trimmed public form and a full private form linked by id

## Description

Tracked by Linear EA-105. A record exists in two forms that share one record id. The public form carries only
identifiers, hashes, verdicts, timestamps and the signature. The private form carries what a public log must never
hold: ticket text, criteria text and model names. The public form is derived from the full record by selecting its
allowed members; it is never produced by removing members from the private form.

Both forms are closed schemas ([NFR-008](../non-functional/NFR-008-closed-schemas.md)). The public form,
`engineering-assurance.record-public/v1`, has the members `protocol`, `record_id`, `kind`, `references` (a list of
`{kind, id}` entries where `kind` is `repo` or `linear_issue`), `hashes` (named digests), `verdicts` (named verdict
values), `timestamps`, and `signature`. The private form, `engineering-assurance.record-private/v1`, has the members
`protocol`, `record_id`, `ticket_text`, `criteria_text` and `model_names`. The two are linked only by equal
`record_id`.

Out of scope: the record kinds that fill these members (EA-97, EA-98) and how the signature is produced
([NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md)). A record kind adds members only by amending this
requirement.

## Behavior

- Engineering Assurance SHALL build the public form by copying only the allowed public members from the record.
- Engineering Assurance SHALL build the private form from the record's ticket text, criteria text and model names.
- Engineering Assurance SHALL give both forms the same `record_id`.
- Engineering Assurance SHALL reject a public form that carries a member outside the closed set.
- Engineering Assurance SHALL reject a record whose public members hold a model name, ticket text or criteria text.
- Engineering Assurance SHALL NOT embed a private-form member in the public form under another name.
- Engineering Assurance SHALL NOT include any private-form member in the signed bytes of the public form.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-056-AC-1 | For a record with ticket text, criteria text and model names, the public form has no ticket text, criteria text or model name anywhere in its serialized bytes, and the private form holds all three. | Test |
| FR-056-AC-2 | Both forms carry the same `record_id`, and the private form carries no member of the public form other than `protocol` and `record_id`. | Test |
| FR-056-AC-3 | A public form with an extra member, or a private form with an extra member, fails schema validation. | Test |
| FR-056-AC-4 | A record whose `verdicts` or `references` value contains a model name or a ticket-text string is rejected before any form is built. | Test |
| FR-056-AC-5 | Changing only the private form's contents leaves the public form's signed bytes and signature unchanged. | Test |

## Dependencies

- **Upstream**: [FR-054](./FR-054-record-sink-interface-and-implementations.md),
  [NFR-008](../non-functional/NFR-008-closed-schemas.md).
- **Downstream**: [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md); the EA-97 and EA-98 specifications.
