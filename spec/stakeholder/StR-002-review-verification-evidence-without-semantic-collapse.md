---
id: StR-002
title: "Review verification evidence without semantic collapse"
type: StR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-008"
    type: "satisfied_by"
  - target: "ix://agent-ix/engineering-assurance/FR-009"
    type: "satisfied_by"
  - target: "ix://agent-ix/engineering-assurance/FR-010"
    type: "satisfied_by"
---

# StR-002: Review verification evidence without semantic collapse

## Stakeholder Need

Assurance owners require definitions, executions, results, evidence,
measurements, diagnostics, reports, and human decisions to remain distinct and
linked so that a retained artifact cannot appear to prove more than its
producer and governing definition establish.

## Rationale

Collapsing these concepts loses whether a check ran, what it observed, which
definition governed it, which bytes were retained, and who made the terminal
decision. Reusing the existing Quire, Quoin, and ix-flow records keeps those
facts independently inspectable without a parallel evidence framework.

## Validation Criteria

| ID | Criteria | Validation |
| --- | --- | --- |
| StR-002-VC-1 | The ownership contract and compatibility fixtures distinguish every semantic concept, preserve all non-success states and provenance, and introduce no second executor, evidence store, or overall trust score. | Integration (TC-052) |

## Stakeholders

Assurance owners, native producer maintainers, reviewers of historical PGM-01
records, and Agents A/B/C applying the later migration contract.

## Dependencies

Quire static exports, Quoin evidence and change-assurance records, and ix-flow
human decision events remain authoritative inputs.

