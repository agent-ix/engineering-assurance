---
id: VO-007
title: Campaign Run
object: campaign_value
type: FR
name: CampaignRun
---

# Campaign Run

## Properties

| Field | Type | Multiplicity | Constraints |
|-------|------|--------------|-------------|
| schemaVersion | String | 1 | nonEmpty |
| id | String | 1 | nonEmpty |
| definitionDigest | String | 1 | nonEmpty |
| sourceGraphDigest | String | 1 | nonEmpty |
| attempts | CampaignAttempt | * | |
| verdict | CampaignVerdict | 1 | |
