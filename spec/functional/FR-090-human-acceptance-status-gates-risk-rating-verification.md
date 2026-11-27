---
id: FR-090
title: "Accept only human-accepted feature risk ratings"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-037"
    type: "references"
---

# FR-090: Accept only human-accepted feature risk ratings

## Description

The rating-acceptance command verifies signed FeatureRiskRating records. A valid
signature proves who committed the record; it does not replace the human
acceptance decision recorded by the rating's `status`.

## Behavior

The command SHALL accept a rating only when its status is `accepted` and the
existing signer, principal, commit, and exact signed-meaning checks pass. A
`proposed` rating SHALL return `not_accepted` with reason `status_proposed`,
even when all signature checks pass. Any other missing or non-accepted status
SHALL return `not_accepted` with reason `status_not_accepted`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-090-AC-1 | A signed, clean rating with status `proposed` and the acceptance meaning trailer yields `not_accepted` with reason `status_proposed`; the same signed record with status `accepted` is accepted. | Test |

