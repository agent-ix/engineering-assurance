---
id: SR-20261114084850
title: "EA-123 base spec review of private Git record store"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance; spec/functional/FR-054-record-sink-interface-and-implementations.md; FR-055-per-repo-sink-configuration-and-visibility-default.md; FR-056-public-and-private-record-split.md; FR-058-approval-statement-and-predicate.md; FR-064-verify-approval-statement-with-workflow-identity-policy.md"
review_set: subset
---

## Summary

Reviewed the changed storage, configuration, canonical statement, and verification requirements. The separate destination and fixed ref reduce accidental exposure, but repository identity comparison and outcome-marker authenticity are not defined strongly enough to preserve privacy and trusted verification.

## Scope

- `FR-055-AC-8` — `spec/functional/FR-055-per-repo-sink-configuration-and-visibility-default.md` (A `private_store.destination` that is not exactly `owner/name` or names the publishing repository is refused with `sink_config_invalid` before a private form is written.)
- `FR-064-AC-23` — `spec/functional/FR-064-verify-approval-statement-with-workflow-identity-policy.md` (After the Git private store round trip, the exact RFC 8785 statement bytes, signature-bundle string, and final publish outcome are returned unchanged; the verifier accepts a published fixture statement and rejects a one-byte change.)

## Verdict

Not clean: two high-severity trust-boundary defects remain in destination identity validation and outcome provenance.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Destination self-check does not define canonical repository identity comparison | spec/functional/FR-055-per-repo-sink-configuration-and-visibility-default.md:96 | wrong-requirement |
| FND-002 | high | The reader trusts an unsigned mutable publish outcome as authoritative | spec/functional/FR-055-per-repo-sink-configuration-and-visibility-default.md:96 | missing-requirement |
