---
id: SR-20261114084852
title: "EA-123 integrity review of private Git record store"
type: SpecReview
analysis: integrity
scope: "agent-ix/engineering-assurance; spec/functional/FR-054-record-sink-interface-and-implementations.md; spec/functional/FR-055-per-repo-sink-configuration-and-visibility-default.md; spec/functional/FR-056-public-and-private-record-split.md; spec/functional/FR-058-approval-statement-and-predicate.md; spec/functional/FR-064-verify-approval-statement-with-workflow-identity-policy.md"
review_set: subset
---

## Summary

Reviewed consistency and completeness across the new private-store protocol and its linked forms. Preparation, terminal outcomes, and replay are described, but the concurrent same-record-ID path can still allow duplicate external sink effects.

## Scope

- `FR-054-AC-15` — `spec/functional/FR-054-record-sink-interface-and-implementations.md` (Replaying a record with an identical private form and a valid `published` marker returns `published` without invoking its sink; replaying one with a valid `refused` marker returns `refused` with `private_store_record_previously_refused` without invoking its sink.)
- `FR-054-AC-10` — `spec/functional/FR-054-record-sink-interface-and-implementations.md` (Two concurrent writes with different record IDs both remain readable after retrying a non-fast-forward push; after five failed push attempts and a refreshed target ref that lacks the record file, the write fails with `private_store_write_failed` without force-pushing or deleting a record.)

## Verdict

Not clean: the new retry contract does not define single-winner behavior for concurrent preparation of the same record ID.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Concurrent preparation of one record ID may invoke the selected sink more than once | spec/functional/FR-054-record-sink-interface-and-implementations.md:151 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | after_excerpt: `When two same-ID preparations race, at most one returns NewRecord; the other observes the winner and returns its valid terminal outcome or fails closed if the marker is incomplete, and the selected sink is invoked at most once.` |
