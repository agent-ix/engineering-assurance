---
id: SR-063
title: "Base review of the Rust workflow-invariant slice"
type: SpecReview
analysis: base
scope: "FR-016-AC-2, FR-016-CON-3, TC-106"
review_set: base
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-016"
    type: reviews
---

# SR-063: Base review of the Rust workflow-invariant slice

## Summary

This owner-selected base subset reviews only the additive Rust implementation
of the eleven existing Engineering Assurance workflow invariants. It does not
approve an ix-flow provider bridge, deletion of either JavaScript path,
onboarding I/O, workflow lifecycle changes, or agent-evaluation work.

The result is **PASS after fixes**. The request and result boundaries, clock
input, complete canonical population, malformed-input refusals, and retained
same-instant differential evidence are now explicit and mapped to TC-106.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | **Closed through `/specify`.** Exception expiry previously depended on JavaScript `Date.now()` while the Rust library is required to be deterministic and I/O-free. The request now supplies one RFC 3339 evaluation instant shared by both engines. | FR-016-AC-2; FR-016-CON-3; TC-106 | missing-requirement |
| FND-002 | high | **Closed through `/specify`.** “Same complete ordered failure set” did not define a protocol, ordering input, unknown-name behavior, or whether success occupied a result position. The v1 request now supplies an ordered name list and the v1 result carries one typed outcome per name; unknown names refuse before any outcome. | FR-016 inputs/outputs; FR-016-AC-2; TC-106 | missing-requirement |
| FND-003 | medium | **Closed through `/specify`.** Malformed instance snapshots and invalid evaluation instants were not named error paths and could have become successful empty results. Both are now fail-closed request errors with stable codes. | FR-016 Error Conditions; TC-106 | missing-requirement |
| FND-004 | medium | **Closed through `/specify`.** The prior matrix row did not require all eleven canonical invariants, valid boundary cases, or malformed Rust requests, so a partial port could satisfy it. TC-106 now separates same-domain differential evidence from strict Rust intake refusals and enumerates both populations. | TC-106 | correct-requirement-no-evidence |

The broader FR-016-AC-4 host integration remains unapproved because ix-flow
0.2.3 accepts only `scripts/invariants.js`; this review deliberately grants no
bridge or removal disposition.
