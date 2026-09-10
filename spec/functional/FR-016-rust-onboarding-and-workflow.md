---
id: FR-016
title: "Run onboarding and workflow invariants through Rust"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/US-003"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-001"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-005"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: "requires"
---

# FR-016: Run onboarding and workflow invariants through Rust

## Description

Engineering Assurance SHALL perform repository discovery, bounded onboarding,
ix-flow coordination, and canonical workflow-invariant evaluation through the
Rust library and CLI.

## Inputs

- A selected repository root and onboarding boundary.
- Canonical skill and workflow definitions.
- Versioned ix-flow run state.
- An `engineering-assurance.workflow-invariants/v1` request containing an
  ordered non-empty invariant-name list, one ix-flow instance snapshot, and an
  explicit RFC 3339 evaluation instant.
- Explicit human decision actions.

## Outputs

- The existing inventory and bounded onboarding proposal.
- An `engineering-assurance.workflow-invariants-result/v1` result containing
  one ordered typed outcome per requested invariant. Each failed outcome carries
  the invariant name, its stable failure code, and structured details.
- Resumable ix-flow state with attributed human terminal events.

## Behavior

- Preserve FR-001 inventory-before-proposal behavior.
- Delegate run state, transitions, and terminal decision history to ix-flow.
- Return all applicable invariant failures in deterministic order.
- Evaluate exception expiry against the request's explicit evaluation instant;
  the reusable Rust library SHALL NOT read the system clock.
- Reject an unknown invariant name rather than treating it as passed or
  omitting it from the result.
- Reject unknown workflow, phase, or protocol versions.
- Leave a decision-ready run non-terminal when human input is absent.
- Retain the pilot path only as a temporary compatibility alias until the
  canonical Rust path passes locally at the same candidate revision.

## Error Conditions

Unavailable Quire or ix-flow tools, malformed host responses, run-binding
mismatches, invalid transitions, automatic terminal decisions, escaping
artifact targets, unknown invariant names, malformed instance snapshots, and
invalid evaluation instants fail closed without overwriting artifacts or run
history. Machine refusals use the stable codes `workflow_invariant_request_invalid`,
`workflow_invariant_unknown`, `workflow_binding_invalid`, `ix_flow_unavailable`,
or `ix_flow_response_invalid` as applicable.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-016-CON-1 | Engineering Assurance SHALL NOT reimplement ix-flow run state or human-gate mechanics. | Responsibility | Test |
| FR-016-CON-2 | A foreign-language host bridge requires an explicit owner disposition before inclusion. | Architecture | Review |
| FR-016-CON-3 | The reusable Rust invariant evaluator SHALL NOT access the filesystem, environment, process table, network, or system clock. | Determinism | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-016-AC-1 | Rust onboarding matches retained behavior for existing, absent, conflicting, malformed, unavailable, and escaping-boundary inputs. | Property (TC-105) |
| FR-016-AC-2 | For every canonical invariant and focused valid boundary fixture, Rust and the retained reference produce the same complete ordered outcome set at the same explicit evaluation instant; unknown names and malformed Rust requests are refused before an outcome is emitted. | Test (TC-106) |
| FR-016-AC-3 | Interruption, resume, acceptance, rejection, missing choice, invalid transition, and run-binding mismatch preserve ix-flow state and human-gate behavior. | Integration (TC-107) |
| FR-016-AC-4 | Canonical and pilot invocations pass through Rust before either legacy JavaScript path is removed. | Integration (TC-108) |

## Dependencies

- **Upstream**: FR-001 through FR-005, FR-014, FR-015, and an ix-flow interface
  supported by the ix-flow owner.
- **Downstream**: FR-018 governs removal of the old paths.
- **Sequence**: FR-016-AC-2 is an independently reviewable additive slice. It
  does not authorize the host bridge or JavaScript removal required by
  FR-016-AC-4.
