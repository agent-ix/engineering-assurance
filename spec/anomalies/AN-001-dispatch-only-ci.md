---
id: AN-001
title: "CI validation does not run on repository changes"
type: Anomaly
status: identified
safety_impact: unassessed
---
# [AN-001] CI validation does not run on repository changes

## Description

The repository's `.github/workflows/ci.yml` is triggered only by a manual
`workflow_dispatch` event. A push or pull request therefore does not run that
workflow's validation jobs. This leaves the result of those checks unavailable
for changes merged without a separate manual run. No severity or safety impact
has been assigned in this record.
