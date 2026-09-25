---
id: VO-006
title: Campaign Member
object: campaign_value
type: FR
name: CampaignMember
---

# Campaign Member

## Properties

| Field | Type | Multiplicity | Constraints |
|-------|------|--------------|-------------|
| name | String | 1 | nonEmpty |
| group | String | 0..1 | nonEmpty |
| planId | String | 1 | nonEmpty |
| definitionVersion | String | 1 | nonEmpty |
| dependsOn | String | * | |
| required | Boolean | 1 | |
| checkerProcedure | MeasurementProcedure | 0..1 | |
