---
id: VO-001
title: Measurement Procedure
object: campaign_value
type: FR
name: MeasurementProcedure
---

# Measurement Procedure

## Properties

| Field | Type | Multiplicity | Constraints |
|-------|------|--------------|-------------|
| schemaVersion | String | 1 | nonEmpty |
| producerName | String | 1 | nonEmpty |
| producerVersion | String | 1 | nonEmpty |
| sourceRepository | String | 1 | nonEmpty |
| arguments | ProcedureArgument | * | |
| environment | ProcedureEnvironment | * | |
| inputs | ProcedureArtifact | * | |
| inputRolePrefixes | ProcedureInputPrefix | * | |
| outputs | ProcedureArtifact | * | |
| outputTrees | ProcedureArtifact | * | |
| responseProtocol | String | 1 | nonEmpty |
| responseAdapter | String | 1 | nonEmpty |
| responseAdapterVersion | String | 1 | nonEmpty |
| repetitions | Integer | 1 | min: 1 |
| timeoutMillis | Integer | 1 | min: 1 |
