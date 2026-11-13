---
id: FR-057
title: "Refuse a public publish and public-log signing when the publishing repo or a reference is private or undeterminable"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-054"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-055"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-056"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-009"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-010"
    type: "references"
---

# FR-057: Refuse a public publish and public-log signing when the publishing repo or a reference is private or undeterminable

## Description

Tracked by Linear EA-105. Before the public-log sink sends anything, and before any signing step writes to a public
transparency log, a leak check resolves the visibility of every repository and issue that the record could disclose:
the publishing repository, the record's `subject` and each entry of its `references`
([FR-056](./FR-056-public-and-private-record-split.md)). A `repo` is resolved through the hosting service. A
`linear_issue` is resolved through the Linear teams that the sink configuration lists as public in
`public_linear_teams` ([FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md)); an issue of any other
team is private. If any checked item is private, or its visibility cannot be determined, the check refuses with one
named error, nothing is sent and nothing is written to a public log, and the check reports each offending item.

Signing is part of publishing to the public sink. Keyless signing against a public transparency log puts the signing
certificate, which names the repository and workflow, on that public log. The signing step therefore has two paths,
chosen from the resolved sink: the public path (upload to the public log) is reachable only after this check passes
for the record being signed, including the publishing repository's own visibility; the non-public path, used for the
private store and `none`, uses a signing configuration that writes to no public transparency log. This holds for every
signature Engineering Assurance makes, including approval statements ([FR-058](./FR-058-approval-statement-and-predicate.md)).

The check applies to the public path only. It runs on every publish and every signing, decides from the record being
published, and does not reroute a refused record. The private store and `none` do not run it.

## Outputs

- On a clean check, signing and publishing proceed.
- On a refusal, the publish result of FR-054 has `status` `refused` and `error` `public_sink_private_reference`, plus
  a list `offending` of `{kind, id, reason}` where `reason` is `private` or `visibility_unknown`. A failed or
  unanswered visibility lookup, for one item or for all of them, is `visibility_unknown` under this one code.

## Behavior

- The leak check SHALL resolve the visibility of the publishing repository before the public path signs or sends.
- The leak check SHALL resolve the visibility of the record's subject and of every reference before the public path
  signs or sends.
- The leak check SHALL treat an item whose visibility cannot be determined as private.
- The leak check SHALL treat a Linear issue whose team is not in `public_linear_teams` as private.
- The leak check SHALL report every offending item and not only the first.
- The signing step SHALL NOT write to a public transparency log unless the leak check passed for the record being signed.
- The signing step SHALL use a path that writes to no public transparency log when the resolved sink is the private
  store or `none`.
- The public-log sink SHALL send no request when the leak check refuses.
- The public-log sink SHALL run the leak check on every publish.
- The leak check SHALL NOT read or send any private-form member.
- The sink layer SHALL NOT route a refused record to another sink.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-057-AC-1 | A record referencing one private repo and two public repos is refused with `public_sink_private_reference`, the offending list names exactly the private repo with reason `private`, and the test endpoint receives zero requests. | Test |
| FR-057-AC-2 | A record referencing a Linear issue of a team not listed in `public_linear_teams` is refused with reason `private`; the same record with the team listed is published. | Test |
| FR-057-AC-3 | An item whose visibility lookup errors, times out or returns an unrecognized value, and a run in which every lookup fails, are refused with the single code `public_sink_private_reference` and reason `visibility_unknown`, and nothing is sent. | Test |
| FR-057-AC-4 | A record whose `references` list is empty, published by a private publishing repository through the public path, is refused with the publishing repository offending with reason `private`; a record whose `subject` is private and whose `references` omit it is refused likewise. | Test |
| FR-057-AC-5 | A record with two offending items yields an offending list of both. | Test |
| FR-057-AC-6 | The check runs on each of two consecutive publishes: a repository made private between them makes the second publish refuse. | Test |
| FR-057-AC-7 | The private-store and `none` sinks publish the same record that the public sink refused, and neither runs the check. | Test |
| FR-057-AC-8 | Over generated records and generated visibility maps, the public sink sends a request, and the signing step writes to a public log, only when the publishing repository, the subject and every reference resolve to public (NFR-009). | Property |
| FR-057-AC-9 | The refusal sends no request to any other sink. | Test |
| FR-057-AC-10 | Signing an approval statement or record for a private publishing repository with the resolved sink `private_store` or `none` makes zero calls to a stand-in public transparency log client; the same signing for a public repository with the public sink makes its call only after the check passed. | Test |
| FR-057-AC-11 | A refused record causes zero stand-in public-log calls from the signing step. | Test |

## Dependencies

- **Upstream**: [FR-054](./FR-054-record-sink-interface-and-implementations.md),
  [FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md),
  [FR-056](./FR-056-public-and-private-record-split.md),
  [NFR-009](../non-functional/NFR-009-no-private-reference-reaches-a-public-log.md),
  [NFR-010](../non-functional/NFR-010-no-cryptography-of-our-own.md).
- **Downstream**: the EA-97 and EA-98 specifications.
