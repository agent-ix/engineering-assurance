---
id: SR-20261124033413
title: "Code review — engineering-assurance PLAT-1817 real-tool lane"
type: SpecReview
analysis: code-review
scope: "Makefile; scripts/real_tool_test.py; spec/evidence/**"
review_set: all
---
## Summary

Reviewed the PLAT-1817 local real-tool lane and retained-run behavior. The runner records entries, but does not bind them to criteria, so a future passing run remains invisible as run evidence in the computed matrix.

## Verdict

**CONDITIONAL** — one medium finding prevents the retained run from serving as criterion-linked evidence.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The runner writes traceIds into SUITE-004 run entries but never creates bindings. The committed bindings file is empty; quoin matrix reports the real-tool criteria as tagged by ignored tests with no run evidence. Add the suite-to-criterion bindings so a passing retained run is usable by the matrix and evidence audit. | scripts/real_tool_test.py:240; spec/evidence/bindings.json:2 |
