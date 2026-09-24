---
id: EN-004
title: Campaign Attempt Status
object: campaign_enum
type: FR
name: CampaignAttemptStatus
---

# Campaign Attempt Status

## Values

| Value | Description |
|-------|-------------|
| completed | EA producer execution completed |
| failed | EA producer execution failed |
| refused | EA refused the request before launch |
| malformed_response | Response adapter rejected output |
| containment_failure | Containment unavailable or breached |
| cancelled | Bound cancellation event ended execution |
| unavailable | Executable unavailable |
| timed_out | Execution deadline elapsed |
| invalid_request | No valid request identity minted |
