---
id: FR-008
title: "Distinguish verification definitions, executions, results, and views"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-005"
    type: "implements"
---

# FR-008: Distinguish verification definitions, executions, results, and views

## Description

Engineering Assurance SHALL define VerificationDefinition,
VerificationExecution, CheckResult, Evidence, Measurement, Diagnostic, Report,
and HumanDecision as distinct semantic concepts and SHALL map each concept to
its authoritative Quire, Quoin, native-producer, or ix-flow representation.

## Inputs

- Quire `assurance-v1` static exports.
- Quoin evidence-store, measurement, operational, change-assurance,
  attestation, audit, and receipt contracts.
- Native structured producer result formats.
- ix-flow retained human decision events.

## Outputs

- An accepted ownership/type-fit ADR.
- A machine-readable ownership registry used by compatibility tests and
  generated-language fixtures, not as a persisted evidence record.

## Behavior

- Definitions SHALL identify the governing definition version and source.
- Executions SHALL retain candidate revision, command, tool/config/environment,
  and time without being treated as a result or decision.
- Results SHALL preserve the native producer state and link to their execution
  and definition.
- Evidence SHALL identify exact retained records or bytes and their integrity
  state without copying them into a second store.
- Measurements SHALL link an active MeasurementPlan definition to a Quoin
  observation and collection producer tuple.
- Diagnostics SHALL remain explanatory facts that never count as success.
- The reporting projection SHALL derive views over referenced records without
  becoming an additional source of evidence truth.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-008-AC-1 | Every semantic concept has exactly one authoritative owner/type and a declared link direction. | Test (TC-056) |
| FR-008-AC-2 | Definition, execution, result, retained evidence, and report identities remain distinct in valid fixtures. | Test (TC-057) |
| FR-008-AC-3 | Missing definition, execution, result, or evidence references fail validation instead of being inferred. | Test (TC-058) |
| FR-008-AC-4 | The contract contains no runner, shell command execution, evidence persistence, or human-decision inference capability. | Static (TC-059) |

## Dependencies

The accepted Quoin semantic-module architecture and type-fit review, Quire
assurance export, Quoin #267/#281/#282, and ix-flow decision-event contract.
