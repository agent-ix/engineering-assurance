---
id: StR-001
title: "Operators need bounded existing-repository onboarding"
type: StR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-001"
    type: "satisfied_by"
  - target: "ix://agent-ix/engineering-assurance/FR-004"
    type: "satisfied_by"
  - target: "ix://agent-ix/engineering-assurance/FR-005"
    type: "satisfied_by"
---

# StR-001: Operators need bounded existing-repository onboarding

## Stakeholder Need

Engineering operators require an agent to assess an existing repository before
it shall propose assurance artifacts or conclusions, so that onboarding adds only
work justified by declared decisions, measurements, evidence, and human intent.

## Rationale

An onboarding tool enters repositories with different maturity, policies, and
evidence producers. Generic scaffolding can create the appearance of assurance
without a real decision boundary or measurement use. An inventory-first process
keeps missing information visible, makes provenance inspectable, and preserves the
operator as the owner of consequential choices.

## Validation Criteria

| ID | Criteria | Validation |
|----|----------|------------|
| StR-001-VC-1 | Across the required evaluation scenarios, onboarding inventories existing decisions and measurements before proposing any artifact. | Test (TC-001) |
| StR-001-VC-2 | When no applicable profile is justified, onboarding records that outcome and creates no generic profile. | Test (TC-002) |
| StR-001-VC-3 | Every terminal workflow outcome is selected by the named human decision owner. | Test (TC-003) |

## Stakeholders

The primary stakeholders are engineering operators and decision owners adopting
assurance workflows in an existing repository. Coding agents are direct users of
the onboarding contract. Maintainers of Quire, Quoin, and ix-flow are affected
parties whose ownership boundaries must remain intact.

## Context and Assumptions

The target repository may contain complete, partial, malformed, or no assurance
artifacts. Evidence producers may be available, unavailable, not yet invoked, or
irrelevant to the chosen boundary. The operator can identify a human owner for
any terminal workflow decision.

## Dependencies

- **Downstream**: [FR-001](../functional/FR-001-inventory-before-proposal.md)
  defines the inventory-first behavior.
- **Downstream**: [FR-004](../functional/FR-004-evidence-state-and-provenance.md)
  defines unavailable and uncomputed evidence handling.
- **Downstream**: [FR-005](../functional/FR-005-resumable-human-decisions.md)
  preserves explicit human terminal choices.

## Priority and Risk (Informative)

Priority is P0. If unmet, onboarding may manufacture assurance posture, erase
unknowns, or allow automation to make a decision assigned to a human.
