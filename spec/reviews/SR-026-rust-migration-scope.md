---
id: SR-026
title: "Scope and boundary review of the Rust-native Engineering Assurance migration"
type: SpecReview
analysis: scope-boundary
scope: "StR-003, ADR-002, FR-014..FR-018, NFR-005"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---

## Summary

The selected repository and the Quoin/ix-flow/portable-contract boundaries are
well chosen. One sentence allocates behavior directly to Quire, an external
component, instead of specifying Engineering Assurance's response at its own
integration boundary.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-061 | high | “Quire SHALL NOT apply review selection” is a normative obligation on another repository. Engineering Assurance may refuse or flag a host result that applies policy from an invalid profile; any Quire-side behavior change must be specified and reviewed by the Quire owner. | FR-015:72-76,104; StR-003:23-25; spec/spec.md:67-68 | wrong-requirement |

## Boundary allocation

- Engineering Assurance owns its Rust domain library, CLI, scenarios,
  assertions, compatibility census, and repository qualification.
- Quire owns authored-fact validation; Quoin owns evidence; ix-flow owns run and
  human-gate state; portable verification contracts remain externally owned.
- Foreign-language fixtures may remain inert data, never executable assurance
  logic.

## Repeat-review disposition

| ID | Disposition |
| --- | --- |
| FND-061 | Fixed: FR-015 now allocates refusal/reporting to Engineering Assurance at its Quire host boundary and explicitly routes any Quire behavior change through a separate accepted Quire specification. |
