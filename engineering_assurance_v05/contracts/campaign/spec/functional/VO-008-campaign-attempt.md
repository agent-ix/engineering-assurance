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
| requestDigest | String | 0..1 | |
| resultDigest | String | 0..1 | |
| collectionId | String | 0..1 | |
| collectionDigest | String | 0..1 | |
| verdictDigest | String | 0..1 | |
| domainVerdictDigest | String | 0..1 | |
| checkerRequestDigest | String | 0..1 | |
| checkerResultDigest | String | 0..1 | |
| rawArtifacts | CampaignRawArtifact | * | |
| status | CampaignAttemptStatus | 1 | |
| reason | String | 0..1 | |
