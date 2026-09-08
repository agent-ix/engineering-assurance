---
id: SR-028
title: "Base review of the Rust migration CI posture amendment"
type: SpecReview
analysis: base
scope: "FR-017, NFR-005, TC-112, TC-127"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/NFR-005"
    type: reviews
---

## Summary

Implementation intake exposed a direct conflict between the accepted
pull-request evidence gate and the prohibition on changing hosted dispatch.
Both requirements cannot govern one Rust-migration pull request.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-063 | high | NFR-005-AC-6/TC-127 require hosted statuses on Rust-migration pull requests, while FR-017-CON-3 and TC-112 prohibit the migration from dispatching hosted CI and require unchanged triggers. A candidate cannot produce the required evidence without violating the prohibition. | FR-017:51,65,75; NFR-005:34,51; spec/tests.md:284,299 | wrong-requirement |

## Disposition

Fixed in the proposed amendment by separating deterministic repository
qualification from real-agent and release operations. Pull-request checks are
required; external evaluation, publication, and release remain manual. Human
owner acceptance is still required because the original ticket granted no CI
dispatch authority.
