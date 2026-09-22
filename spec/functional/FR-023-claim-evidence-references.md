---
id: FR-023
title: "Require evidence references for a supported assurance claim"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-005"
    type: "implements"
---

# FR-023: Require evidence references for a supported assurance claim

## Description

The AssuranceArgument frontmatter schema SHALL require every claim whose status
is `supported` to reference at least one authoritative evidence record through
its own `evidence_refs` list.

## Inputs

- An authored AssuranceArgument frontmatter document whose `top_claim` carries
  a status of `open`, `supported`, `challenged`, or `rejected`.
- An optional per-claim `evidence_refs` list of `ix://` references to the
  evidence records the claim cites, in the same `ix://` form the argument uses
  for its `profile`, `resolution_ref`, and relationship targets.

## Outputs

- A schema validation result that accepts or rejects the claim's status and
  evidence references together.

## Behavior

- When a claim's status is `supported`, the schema SHALL require an
  `evidence_refs` list containing at least one reference.
- When a claim carries `evidence_refs`, the schema SHALL require a non-empty
  list of unique `ix://` references.
- When a claim's status is `open`, `challenged`, or `rejected`, the schema SHALL
  accept the claim with or without `evidence_refs`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-023-AC-1 | A `supported` claim without `evidence_refs` fails schema validation, and the same claim with one `ix://` evidence reference passes. | Test (TC-139) |
| FR-023-AC-2 | An `open`, `challenged`, or `rejected` claim passes schema validation both with and without `evidence_refs`. | Test (TC-139) |
| FR-023-AC-3 | An empty `evidence_refs` list, a duplicated reference, and a reference that is not an `ix://` URI each fail schema validation. | Test (TC-139) |

## Dependencies

- **Upstream**: [US-005](../usecase/US-005-correlate-definition-result-evidence.md).
- **Downstream**: Quoin owns the evidence records the references identify;
  Quire validates authored AssuranceArgument frontmatter against this schema.
