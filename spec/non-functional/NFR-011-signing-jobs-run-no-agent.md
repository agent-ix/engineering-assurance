---
id: NFR-011
title: "Signing jobs run no agent and hold no agent-reachable credential"
type: NFR
quality_attribute: security
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-068"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-074"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-075"
    type: "constrains"
---

# NFR-011: Signing jobs run no agent and hold no agent-reachable credential

## Statement

Engineering Assurance shall run every workflow job that holds signing permission without any AI agent step and without a credential that an agent can read.

## Scope

- Applies to: the five workflow files of FR-070 and every job in them that declares `id-token: write` or `attestations: write`.

## Rationale

A receipt or release attestation is evidence about what an agent produced. If the agent could sign, the evidence would be the agent's own claim. The signing identity is the CI workflow identity only.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Count of steps in a signing job that invoke an agent harness or model client | 0 | 0 | Source inspection of the workflow files |
| Count of signing jobs whose identity is not the workflow identity | 0 | 0 | Test with a stand-in signer |

## Verification

A source inspection checks the first count over the five files; the verification tests of FR-068 and FR-078 check the second.

## Dependencies

- **Upstream**: [FR-070](../functional/FR-070-release-and-receipt-workflow-files.md).
- **Downstream**: [FR-068](../functional/FR-068-verification-summary-signed-by-the-pinned-workflow.md),
  [FR-074](../functional/FR-074-per-change-receipt-bundle-records-failures-too.md),
  [FR-075](../functional/FR-075-push-to-main-receipt-workflow.md).
