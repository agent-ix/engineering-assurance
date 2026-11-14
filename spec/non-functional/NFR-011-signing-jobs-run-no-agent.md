---
id: NFR-011
title: "Signing jobs run no agent, no repository code and hold no agent-reachable credential"
type: NFR
quality_attribute: security
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-068"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-070"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-074"
    type: "constrains"
  - target: "ix://agent-ix/engineering-assurance/FR-075"
    type: "constrains"
---

# NFR-011: Signing jobs run no agent, no repository code and hold no agent-reachable credential

## Statement

Engineering Assurance shall run every workflow job that holds signing permission without any AI agent step, without any code from the repository under change, and without a credential that an agent can read.

## Scope

- Applies to: the workflow files of FR-070 and every job in them that declares `id-token: write` or `attestations: write`.

## Rationale

A receipt or release attestation is evidence about what an agent produced. If the agent, or code the agent wrote, could sign, the evidence would be its own claim: code that runs in a job holding the signing permission can mint the signing token. The repository's gates and builds therefore run in a separate job with read access only, and hand results to the signing job as an artifact.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Count of steps in a signing job that invoke an agent harness or model client | 0 | 0 | Source inspection of the workflow files |
| Count of signing jobs that execute code from the repository under change | 0 | 0 | YAML-parsing test and source inspection |
| Count of signing jobs whose identity is not the workflow identity | 0 | 0 | Test with a stand-in signer |
| Count of signing jobs whose signing credential is readable by an AI agent step or process | 0 | 0 | Inspection |

## Verification

A source inspection checks the first count over the workflow files; the YAML-parsing tests of FR-070 and the gate-isolation case of FR-075 check the second; the verification tests of FR-068 and FR-078 check the third; a source inspection checks whether any signing credential is readable by an agent.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-011-AC-1 | Every signing-permission workflow job contains zero steps that invoke an AI agent harness or model client. | Inspection |
| NFR-011-AC-2 | Every signing-permission workflow job executes no code from the repository under change. | Test |
| NFR-011-AC-3 | Every signing-permission workflow job uses the workflow identity for signing. | Test |
| NFR-011-AC-4 | No signing job makes a signing credential readable to an AI agent step or process. | Inspection |

## Dependencies

- **Upstream**: [FR-070](../functional/FR-070-release-and-receipt-workflow-files.md).
- **Downstream**: [FR-068](../functional/FR-068-verification-summary-signed-by-the-pinned-workflow.md),
  [FR-074](../functional/FR-074-per-change-receipt-bundle-records-failures-too.md),
  [FR-075](../functional/FR-075-push-to-main-receipt-workflow.md).
