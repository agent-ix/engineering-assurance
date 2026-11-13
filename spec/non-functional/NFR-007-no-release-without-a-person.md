---
id: NFR-007
title: "No release without a verified person's signed decision"
type: NFR
quality_attribute: security
relationships:
  - target: "ix://agent-ix/engineering-assurance/NFR-004"
    type: "extends"
  - target: "ix://agent-ix/engineering-assurance/FR-047"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-048"
    type: "constrains"
---

# NFR-007: No release without a verified person's signed decision

## Statement

Engineering Assurance shall report a release as `released` only for a signed
annotated tag whose signer a supplied trust input maps to the release record's
named decision owner.

## Scope

- Applies to: `release-verify`, the `release-decision` workflow, and the
  Baseline and Release archetypes.

## Rationale

A release is a person's accountable act. A tool that could produce or default
that decision would turn evidence into approval, which this repository exists
to avoid. FR-047 and FR-048 carry the obligations that the release module signs
nothing and the workflow chooses nothing.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Count of `released` results over generated inputs where the FR-041 status is not `approved` or the approver is not the decision owner | 0 | 0 | Property test over the pure decision |
| Count of code paths in the release module and workflow that create, sign or push a tag or choose `release` | 0 | 0 | Source inspection |

## Verification

The pure decision of FR-047-AC-12 is exercised over every combination, the
workflow tests of FR-048 run with no owner choice and with evidence that
has every criterion `bound`, and a source inspection checks the second count.

## Dependencies

- **Upstream**: [FR-047](../functional/FR-047-release-record-and-signed-release-tag.md),
  [FR-048](../functional/FR-048-human-release-decision-workflow.md).
