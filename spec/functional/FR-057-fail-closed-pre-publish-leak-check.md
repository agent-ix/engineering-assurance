---
id: FR-057
title: "Refuse a public publish when any referenced repo or Linear issue is private or undeterminable"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-054"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-056"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-009"
    type: "references"
---

# FR-057: Refuse a public publish when any referenced repo or Linear issue is private or undeterminable

## Description

Tracked by Linear EA-105. Before the public-log sink sends anything, a leak check walks every entry of the record's
`references` ([FR-056](./FR-056-public-and-private-record-split.md)) and resolves the visibility of each: a `repo`
entry through the hosting service, and a `linear_issue` entry through the Linear teams that the sink configuration
lists as public. A Linear issue whose team is not listed as public is private. If any reference is private, or its
visibility cannot be determined, the public sink refuses the record with a named error, sends nothing, and the check
reports each offending reference.

The check applies to the public-log sink only. It runs on every publish, before the sink sends, and it decides from
the references of the record being published, not from an earlier check. The sink configuration
([FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md)) gains an optional member
`public_linear_teams` (a list of team keys) for this check; the default is the empty list, so an unlisted team is
private.

The check does not reroute a refused record to another sink. The private store and `none` do not run it.

## Outputs

- On a clean check, publishing proceeds.
- On a refusal, the publish result of FR-054 has `status` `refused` and `error` `public_sink_private_reference`, plus a
  list `offending` of `{kind, id, reason}` where `reason` is `private` or `visibility_unknown`.
- A check that cannot run at all (for example, no visibility source is reachable) refuses with
  `public_sink_visibility_unavailable`.

## Behavior

- The leak check SHALL resolve the visibility of every reference in the record before the public sink sends.
- The leak check SHALL treat a reference whose visibility cannot be determined as private.
- The leak check SHALL treat a Linear issue whose team is not in `public_linear_teams` as private.
- The leak check SHALL report every offending reference and not only the first.
- The public-log sink SHALL send no request when the leak check refuses.
- The public-log sink SHALL run the leak check on every publish.
- The leak check SHALL NOT read or send any private-form member.
- The sink layer SHALL NOT route a refused record to another sink.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-057-AC-1 | A record referencing one private repo and two public repos is refused with `public_sink_private_reference`, the offending list names exactly the private repo with reason `private`, and the test endpoint receives zero requests. | Test |
| FR-057-AC-2 | A record referencing a Linear issue of a team not listed in `public_linear_teams` is refused with reason `private`; the same record with the team listed is published. | Test |
| FR-057-AC-3 | A reference whose visibility lookup errors, times out or returns an unrecognized value is refused with reason `visibility_unknown`, and nothing is sent. | Test |
| FR-057-AC-4 | With no visibility source reachable, the publish is refused with `public_sink_visibility_unavailable` and nothing is sent. | Test |
| FR-057-AC-5 | A record with two offending references yields an offending list of both. | Test |
| FR-057-AC-6 | The check runs on each of two consecutive publishes: a repository made private between them makes the second publish refuse. | Test |
| FR-057-AC-7 | The private-store and `none` sinks publish the same record that the public sink refused, and neither runs the check. | Test |
| FR-057-AC-8 | Over generated records and generated visibility maps, the public sink sends a request only when every reference resolves to public (NFR-009). | Property |
| FR-057-AC-9 | The refusal sends no request to any other sink. | Test |

## Dependencies

- **Upstream**: [FR-054](./FR-054-record-sink-interface-and-implementations.md),
  [FR-055](./FR-055-per-repo-sink-configuration-and-visibility-default.md),
  [FR-056](./FR-056-public-and-private-record-split.md),
  [NFR-009](../non-functional/NFR-009-no-private-reference-reaches-a-public-log.md).
- **Downstream**: the EA-97 and EA-98 specifications.
