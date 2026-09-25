---
id: VO-005
title: Campaign Source
object: campaign_value
type: FR
name: CampaignSource
---

# Campaign Source

For a Git source, `revision` is the exact commit ID and `digest` is the bare
lowercase SHA-256 of the exact raw stdout of
`git ls-tree -r -z --full-tree <revision>`. This NUL-delimited inventory binds
the committed paths, modes, blob object IDs, lockfiles, and gitlinks. Consumers
recompute it against the retained repository before accepting a run and execute
from the selected clean revision. EA's pure resolver checks the declared
revision and digest syntax; it cannot inspect the repository bytes. The bounded
source projection verifies every tracked regular-file and symlink blob OID.
Regular files are selected as sealed inputs. Symlinks are reported as omitted
link metadata. Omitted-link target bytes are verified when the procedure is
resolved, and this metadata does not assert that the link is unchanged at
launch. The producer runs in the capability root.

## Properties

| Field | Type | Multiplicity | Constraints |
|-------|------|--------------|-------------|
| repository | String | 1 | nonEmpty |
| revision | String | 1 | nonEmpty |
| digest | String | 1 | nonEmpty |
