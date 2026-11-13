---
id: NFR-009
title: "No private reference reaches a public log"
type: NFR
quality_attribute: security
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-057"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-054"
    type: "constrains"
---

# NFR-009: No private reference reaches a public log

## Statement

Engineering Assurance shall send a record to, or sign a record through, a public log only when the publishing repository, the record's subject and every reference in the record resolve to public.

## Scope

- Applies to: the public-log sink and its leak check, for every record kind.

## Rationale

A public transparency log cannot delete what it accepts, and a keyless signature on it names the repository and workflow. The leak check is the only barrier, so it fails closed: an
unknown visibility is private, and a missing visibility source blocks the send.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Count of public-log sends or public-log signatures where the publishing repository, the subject or any reference is private or undeterminable, over generated records and visibility maps | 0 | 0 | Property test over the sink layer |
| Count of code paths that reach the public-log send without passing the leak check | 0 | 0 | Source inspection |

## Verification

The property test of FR-057-AC-8 covers the first count; a source inspection checks the second.

## Dependencies

- **Upstream**: [FR-057](../functional/FR-057-fail-closed-pre-publish-leak-check.md),
  [FR-054](../functional/FR-054-record-sink-interface-and-implementations.md).
