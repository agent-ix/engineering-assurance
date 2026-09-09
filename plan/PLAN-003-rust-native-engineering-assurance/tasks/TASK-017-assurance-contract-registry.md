---
id: TASK-017
title: "Implement the assurance-contract registry"
type: Task
status: not_started
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/engineering-assurance/TASK-016"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: references
  - target: "ix://agent-ix/engineering-assurance/TC-119"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-120"
    type: verifies
  - target: "ix://agent-ix/engineering-assurance/TC-122"
    type: verifies
---
# TASK-017: Implement the assurance-contract registry

## Scope

Implement the committed owner-reviewed consumer registry and revision-bound artifact compatibility snapshot for AssuranceProfile and MeasurementPlan contracts.

## Subtasks

- [x] Reconcile the diagnostic organization-wide census with the specification;
  record the omitted `quire-code-rs` consumer and the repository/artifact
  cardinality correction for owner review.
- [ ] Validate version, digest, human-owner acceptance, canonical repository identity, uniqueness, classification, owner, and exclusion reason.
- [ ] Bind candidate commits, paths, blobs, provider/schema digests, outcomes, and dispositions.
- [ ] Refuse registry mutation and prevent an invalid profile from governing review selection.

## Deliverables

- Registry and snapshot types, CLI operations, schemas, and accepted consumer fixtures.
- One unique repository entry containing one or more independently classified
  artifact entries; no duplicate repository row is needed for a multi-artifact
  consumer.
- A stable owner-reviewed population that does not pin ordinary code revisions;
  candidate commits and artifact-content digests belong to each generated
  compatibility snapshot.
- A separately bound provider candidate; the registry refuses the Engineering
  Assurance repository that contains it and therefore cannot self-reference its
  own commit.
- Fail-closed owner-review states that distinguish a prepared registry from the
  canonical population digest explicitly accepted by the named human, plus a
  separate snapshot digest over the exact registry input bytes.
- Duplicate, tamper, mutation, legacy/current/malformed, and invalid-governance tests.

## Notes

- This is the authoritative population boundary reused by later consumer migration; workstation scans are diagnostic only.
