---
id: VO-012
title: Procedure Input Origin
object: campaign_value
type: FR
name: ProcedureInputOrigin
---

# Procedure Input Origin

## Properties

| Field | Type | Multiplicity | Constraints |
|-------|------|--------------|-------------|
| role | String | 1 | nonEmpty |
| kind | ProcedureInputOriginKind | 1 | |
| sourceRepository | String | 0..1 | nonEmpty |
| sourcePath | String | 0..1 | nonEmpty |
| dependencyMember | String | 0..1 | nonEmpty |
| dependencyArtifactRole | String | 0..1 | nonEmpty |
