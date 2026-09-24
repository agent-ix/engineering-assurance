---
id: VO-004
title: Campaign Definition
object: campaign_value
type: FR
name: CampaignDefinition
---

# Campaign Definition

## Properties

| Field | Type | Multiplicity | Constraints |
|-------|------|--------------|-------------|
| schemaVersion | String | 1 | nonEmpty |
| id | String | 1 | nonEmpty |
| subjectName | String | 1 | nonEmpty |
| subjectVersion | String | 1 | nonEmpty |
| sourceGraph | CampaignSource | 1..* | |
| members | CampaignMember | 1..* | |
| completionRule | CampaignCompletionRule | 1 | |
