---
id: FR-010
title: "Provide read-only legacy compatibility and bounded reports"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-005"
    type: "implements"
---

# FR-010: Provide read-only legacy compatibility and bounded reports

## Description

Engineering Assurance SHALL define a read-only PGM-01 v1/v2 mapping and a
bounded report projection that references authoritative records without
rewriting history or creating a parallel evidence schema.

## Inputs

- Immutable PGM-01 v1/v2 envelope, manifest, outcome, identity, and
  availability fields.
- Current Quire, Quoin, and ix-flow record references.

## Outputs

- A compatibility result containing source references, mapped targets,
  limitations, and an explicit compatible, lossy, incompatible, or unreadable
  outcome.
- JSON and Markdown report projections organized around claims, evidence,
  counterevidence, gaps, owner, and action.

## Behavior

- Compatibility SHALL perform no writes to the source record or evidence tree.
- The compatibility mapper SHALL classify ambiguous fields as unmapped or lossy
  without synthesizing values.
- Unreadable, malformed, stale, tampered, or contradictory records SHALL NOT be
  accepted as passed.
- Reports SHALL retain claims, evidence, counterevidence, gaps, owner, and
  action without an overall trust score.
- A report SHALL reference, not replace, Quoin evidence/audit records and
  ix-flow human decisions.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-010-AC-1 | Mapping a legacy fixture leaves every source byte unchanged. | Test (TC-064) |
| FR-010-AC-2 | Known fields preserve source revision, producer/config identity, result state, retained-output identity, and limitations. | Test (TC-065) |
| FR-010-AC-3 | Ambiguous, unreadable, malformed, stale, and tampered fixtures remain non-successful with explicit reasons. | Test (TC-066) |
| FR-010-AC-4 | JSON and Markdown reports expose claims, evidence, counterevidence, gaps, owner, and action with no overall trust score. | Test (TC-067) |

## Dependencies

FR-008 and FR-009. The representative compatibility fixture gate remains
non-enforcing until Engineering Assurance #9 is accepted.
