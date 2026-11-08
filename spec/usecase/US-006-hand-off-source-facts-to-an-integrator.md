---
id: US-006
title: "Hand source-release facts to an integrator without closing the integrator's validation"
type: US
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-002"
    type: "derives_from"
---

# US-006: Hand source-release facts to an integrator without closing the integrator's validation

## Story

**As a** maintainer of a source library that others integrate
**I want** one shared, versioned package format that carries my release
candidate's assumptions, limitations, negative results, reviews, decisions,
licenses and provenance, and that records what the integrating system still
has to establish for itself
**So that** an integrator receives exactly what I established, can see what I
did not, and is never told that its own validation has already been done.

## Acceptance Examples (Illustrative)

| ID | Criteria | Verification |
| --- | --- | --- |
| US-006-AC-1 | A package written for one candidate reads back with every fact, state and adopter obligation exactly as written, including those that are open, negative, conflicting or refused. | Test |
| US-006-AC-2 | Publishing the same package twice yields one object; publishing different content for the same candidate attempt yields a conflict and presents neither version. | Test |
| US-006-AC-3 | A package altered after writing, or written by a different protocol version, is refused rather than partly read. | Test |

## Dependencies (Contextual)

Candidate selection, release decisions and the integrator's own validation
remain with their owners; the package only carries them.

## Exception Scenarios

- A fact nobody has examined is stated as incomplete, not left out.
- An integrator-supplied response is carried as the integrator's own input and
  is never promoted to an accepted obligation by the package.
- Two writers disagree about one attempt: the attempt is in conflict.

## Assurance Risk

The primary risk is a package that reads as complete or accepted when a fact was
lost, improved or replaced in transit.

## Priority and Risk (Informative)

- **Priority:** P0.
- **Primary risk:** silent loss or improvement of an open or negative fact.
