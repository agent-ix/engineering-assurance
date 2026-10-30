---
id: VO-008
title: Campaign Attempt
object: campaign_value
type: FR
name: CampaignAttempt
---

# Campaign Attempt

## Properties

| Field | Type | Multiplicity | Constraints |
|-------|------|--------------|-------------|
| member | String | 1 | nonEmpty |
| index | Integer | 1 | min: 1 |
| collectionId | String | 0..1 | |
| rawArtifacts | CampaignRawArtifact | * | |
| status | CampaignAttemptStatus | 1 | |
| reason | String | 0..1 | |
