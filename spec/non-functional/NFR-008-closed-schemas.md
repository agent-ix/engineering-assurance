---
id: NFR-008
title: "Records, configuration, statements and policies use closed schemas"
type: NFR
quality_attribute: security
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-055"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-056"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-058"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-061"
    type: "constrains"
---

# NFR-008: Records, configuration, statements and policies use closed schemas

## Statement

Engineering Assurance shall reject any sink configuration, public or private record form, approval statement, approval
predicate, role policy and workflow-identity policy that carries a member its schema does not define.

## Scope

- Applies to: the documents of FR-055, FR-056, FR-058, FR-061 and FR-064, and the publish result of FR-054.

## Rationale

An open schema lets a record carry private text under a new member name past a leak check, and lets a policy carry an
ignored member that reads as enforced. A closed schema makes the member set the whole contract.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Count of accepted documents with an unknown member, over one generated unknown member per document kind | 0 | 0 | Schema test per document kind |

## Verification

A test per document kind adds one unknown member to a valid document and expects rejection.

## Dependencies

- **Upstream**: none.
- **Downstream**: [FR-055](../functional/FR-055-per-repo-sink-configuration-and-visibility-default.md),
  [FR-056](../functional/FR-056-public-and-private-record-split.md),
  [FR-058](../functional/FR-058-approval-statement-and-predicate.md),
  [FR-061](../functional/FR-061-role-policy-and-distinct-individuals.md).
