---
id: SR-023
title: "Dependency review of the Rust-native Engineering Assurance migration"
type: SpecReview
analysis: dependency
scope: "ADR-002, FR-014..FR-018, NFR-005"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-018"
    type: reviews
---

## Summary

The migration order is explicit and deletion is correctly last. The ix-flow and
cli-agent-evals host interfaces are genuine external prerequisites, but the
candidate does not identify an accepted interface artifact/revision or make that
identity an executable entry gate.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-056 | high | FR-016 and FR-017 depend on “a reviewed” structured host interface, but neither requirement names an interface identity, version source, owner acceptance record, or refusal when the configured host resolves different bytes. TC-108/TC-109 are merely marked pending host interface. Host-bound implementation must remain blocked until exact accepted artifacts are recorded. | ADR-002:122-125,144-145; FR-016:69-72; FR-017:72-75; spec/tests.md:273-275,458-462 | missing-requirement |

## Dependency graph

Intended order: accepted ADR-002 → FR-014 → FR-015 → FR-016 and FR-017 →
FR-018. NFR-005 constrains every stage. ix-flow gates FR-016/TC-108;
cli-agent-evals gates FR-017/TC-109. FND-050 covers the missing internal graph
edges; FND-056 covers the unresolved external artifacts.

## Repeat-review dispositions

| ID | Disposition |
| --- | --- |
| FND-056 | Specification gap fixed: FR-016-CON-3/AC-5 and FR-017-CON-4/AC-5 require exact accepted host artifact identity, version, revision, digest, and mismatch refusal (TC-123/TC-124). The external accepted artifacts remain intentionally open blockers; no host-bound implementation is authorized. |
