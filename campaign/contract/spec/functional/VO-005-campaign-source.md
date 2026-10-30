---
id: VO-005
title: Campaign Source
object: campaign_value
type: FR
name: CampaignSource
---

# Campaign Source

A campaign source names one repository. The caller supplies the Git tree
inventory for the source it selects, and the bounded source projection checks
every tracked regular-file and symlink blob OID against that inventory. Regular
files are selected as inputs. Symlinks are reported as omitted link metadata.
The producer runs in the capability root.

## Properties

| Field | Type | Multiplicity | Constraints |
|-------|------|--------------|-------------|
| repository | String | 1 | nonEmpty |
